pub mod provisioning;
mod storage_platform;
pub mod wire;
use crate::{
    application::{
        discovery::{DeviceProbe, DeviceRecord, ProbeFuture, ProbeSnapshot},
        installed::{InstalledError, InstalledFuture, InstalledReader},
        packages::{
            PackageDriver, PackageError, PackageFuture, PackageObserver, PackageProgress,
            PackageTarget,
        },
        store::StoreCancellation,
    },
    domain::{
        device::*,
        installed::InstallationObservation,
        packages::*,
        store::{RuntimeCapability, RuntimeProvides, is_digest},
        three_ds::*,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{io::AsyncReadExt, sync::Mutex as AsyncMutex};
use wire::{Connection, Info, WireError};

#[derive(Clone, Serialize, Deserialize)]
struct Pair {
    token: String,
    address: IpAddr,
    port: u16,
    binding: Option<String>,
}
struct Session {
    connection: Option<Connection>,
}
struct Peer {
    id: String,
    config: Mutex<Pair>,
    session: AsyncMutex<Session>,
    observed: Mutex<Option<Info>>,
}
pub struct ThreeDsBridge {
    path: PathBuf,
    peers: Mutex<HashMap<String, Arc<Peer>>>,
    /// Address from Preferences, asked directly on every scan. It is a hint
    /// only: a console is trusted through its pairing, never its address.
    hint: Mutex<Option<IpAddr>>,
}
impl ThreeDsBridge {
    pub fn new(path: PathBuf) -> Result<Self, provisioning::SetupError> {
        let pairs: Vec<Pair> = match std::fs::read(&path) {
            Ok(bytes) if bytes.len() <= 65536 => {
                serde_json::from_slice(&bytes).map_err(|_| provisioning::SetupError::Storage)?
            }
            Ok(_) => return Err(provisioning::SetupError::Storage),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(_) => return Err(provisioning::SetupError::Storage),
        };
        let mut peers = HashMap::new();
        for pair in pairs {
            provisioning::parse_token(pair.token.as_bytes())?;
            let peer = Self::peer_from(pair);
            peers.insert(peer.id.clone(), peer);
        }
        Ok(Self {
            path,
            peers: Mutex::new(peers),
            hint: Mutex::new(None),
        })
    }
    pub fn set_hint(&self, address: Option<IpAddr>) -> Result<(), provisioning::SetupError> {
        *self
            .hint
            .lock()
            .map_err(|_| provisioning::SetupError::Storage)? = address;
        Ok(())
    }
    fn peer_from(config: Pair) -> Arc<Peer> {
        Arc::new(Peer {
            id: uuid::Uuid::new_v4().to_string(),
            config: Mutex::new(config),
            session: AsyncMutex::new(Session { connection: None }),
            observed: Mutex::new(None),
        })
    }
    fn peer(&self, id: &str) -> Option<Arc<Peer>> {
        self.peers.lock().ok()?.get(id).cloned()
    }
    pub fn contains(&self, id: &str) -> bool {
        self.peer(id).is_some()
    }
    fn save(&self) -> Result<(), provisioning::SetupError> {
        let pairs: Vec<_> = self
            .peers
            .lock()
            .map_err(|_| provisioning::SetupError::Storage)?
            .values()
            .map(|p| p.config.lock().expect("pairing poisoned").clone())
            .collect();
        provisioning::private_write(
            &self.path,
            &serde_json::to_vec(&pairs).map_err(|_| provisioning::SetupError::Storage)?,
        )
    }
    pub fn add_pair(
        &self,
        token: [u8; 32],
        address: IpAddr,
        port: u16,
    ) -> Result<String, provisioning::SetupError> {
        let token = provisioning::token_text(&token);
        let existing = self
            .peers
            .lock()
            .map_err(|_| provisioning::SetupError::Storage)?
            .values()
            .find(|p| p.config.lock().expect("pairing poisoned").token == token)
            .cloned();
        let peer = if let Some(peer) = existing {
            let mut config = peer
                .config
                .lock()
                .map_err(|_| provisioning::SetupError::Storage)?;
            config.address = address;
            config.port = port;
            drop(config);
            peer
        } else {
            let peer = Self::peer_from(Pair {
                token,
                address,
                port,
                binding: None,
            });
            self.peers
                .lock()
                .map_err(|_| provisioning::SetupError::Storage)?
                .insert(peer.id.clone(), peer.clone());
            peer
        };
        self.save()?;
        Ok(peer.id.clone())
    }
    pub async fn confirm(&self, id: &str) -> Result<(), provisioning::SetupError> {
        let peer = self.peer(id).ok_or(provisioning::SetupError::Device)?;
        let mut session = peer.session.lock().await;
        let info = read_info(&peer, &mut session)
            .await
            .map_err(|_| provisioning::SetupError::Device)?;
        let binding = info
            .binding
            .filter(|value| is_digest(value))
            .ok_or(provisioning::SetupError::Device)?;
        let mut config = peer
            .config
            .lock()
            .map_err(|_| provisioning::SetupError::Storage)?;
        if config
            .binding
            .as_ref()
            .is_some_and(|previous| previous != &binding)
        {
            return Err(provisioning::SetupError::Device);
        }
        config.binding = Some(binding);
        drop(config);
        self.save()
    }
}
async fn read_info(peer: &Peer, session: &mut Session) -> Result<Info, WireError> {
    let config = peer
        .config
        .lock()
        .map_err(|_| WireError::Unavailable)?
        .clone();
    if session.connection.is_none() {
        let token = provisioning::parse_token(config.token.as_bytes())
            .map_err(|_| WireError::Authentication)?;
        session.connection =
            Some(Connection::connect(SocketAddr::new(config.address, config.port), &token).await?);
    }
    let result = session.connection.as_mut().unwrap().info().await;
    if result.is_err() {
        session.connection = None;
    }
    let info = result?;
    if config
        .binding
        .as_ref()
        .is_some_and(|binding| info.binding.as_ref() != Some(binding))
    {
        session.connection = None;
        return Err(WireError::Authentication);
    }
    *peer.observed.lock().map_err(|_| WireError::Unavailable)? = Some(info.clone());
    Ok(info)
}
fn record(peer: &Peer, info: &Info) -> DeviceRecord {
    let firmware = info.firmware.as_deref().and_then(parse_firmware);
    let trusted = peer
        .config
        .lock()
        .expect("pairing poisoned")
        .binding
        .as_ref()
        .is_some_and(|binding| info.binding.as_ref() == Some(binding));
    let mut capabilities = vec![RuntimeCapability::GuestUpdate];
    if info.launcher {
        capabilities.extend([
            RuntimeCapability::AppLibrary,
            RuntimeCapability::FileManagement,
        ]);
        if info.native_management {
            capabilities.push(RuntimeCapability::CiaManagement);
        }
    }
    let name = match info.model.as_deref() {
        Some("KTR") => "New Nintendo 3DS",
        Some("RED") => "New Nintendo 3DS XL / LL",
        Some("JAN") => "New Nintendo 2DS XL",
        _ => "Nintendo 3DS",
    };
    DeviceRecord {
        summary: DeviceSummary {
            id: peer.id.clone(),
            platform: Platform::ThreeDs,
            model_identifier: info.model.clone(),
            marketing_name: name.into(),
            chip: None,
            board_config: None,
            os_version: firmware.as_ref().map(|(v, _)| v.clone()),
            build_number: None,
            udid_masked: None,
            ecid_masked: None,
            serial_masked: None,
            storage_gb: None,
            storage_total_bytes: info.storage_total_bytes,
            storage_free_bytes: info.storage_free_bytes,
            battery_percent: info.battery_percent,
            mode: DeviceMode::Normal,
            transport: Transport::Network,
            three_ds: Some(ThreeDsDetails {
                region: info.region.clone(),
                firmware_revision: firmware.map(|(_, r)| r),
                firmware: info.firmware.clone(),
                runtime: RuntimeProvides {
                    id: info.host.runtime_id.clone(),
                    version: info.host.runtime_version.clone(),
                    capabilities,
                },
                host_abi: info.host.host_abi,
                host_app_id: info.host.app_id.clone(),
                launcher: info.launcher,
                busy: info.busy,
                native_management: info.native_management,
                hardware_verified: false,
            }),
        },
        facts: DeviceFacts {
            cfw: info.cfw,
            pairing_trusted: Some(trusted),
            ..Default::default()
        },
    }
}
impl DeviceProbe for ThreeDsBridge {
    fn scan(&self) -> ProbeFuture<'_> {
        Box::pin(async move {
            let peers: Vec<_> = self
                .peers
                .lock()
                .expect("peers poisoned")
                .values()
                .cloned()
                .collect();
            if peers.is_empty() {
                return ProbeSnapshot::default();
            }
            let mut addresses = vec![IpAddr::V4(Ipv4Addr::BROADCAST)];
            addresses.extend(self.hint.lock().ok().and_then(|hint| *hint));
            addresses.extend(
                peers
                    .iter()
                    .map(|p| p.config.lock().expect("pairing poisoned").address),
            );
            if let Ok(found) = wire::discover(&addresses).await {
                for peer in &peers {
                    let mut config = peer.config.lock().expect("pairing poisoned");
                    if let Ok(token) = provisioning::parse_token(config.token.as_bytes())
                        && let Some((_, address)) =
                            found.iter().find(|(id, _)| *id == wire::key_id(&token))
                    {
                        config.address = address.ip();
                        config.port = address.port();
                    }
                }
            }
            let mut tasks = tokio::task::JoinSet::new();
            for peer in peers {
                tasks.spawn(async move {
                    let info = if let Ok(mut state) = peer.session.try_lock() {
                        read_info(&peer, &mut state).await.ok()
                    } else {
                        peer.observed.lock().ok().and_then(|v| v.clone())
                    };
                    info.map(|info| record(&peer, &info))
                });
            }
            let mut snapshot = ProbeSnapshot::default();
            while let Some(result) = tasks.join_next().await {
                if let Ok(Some(record)) = result {
                    snapshot.records.push(record);
                }
            }
            snapshot
                .records
                .sort_by(|a, b| a.summary.id.cmp(&b.summary.id));
            snapshot
        })
    }
}
impl InstalledReader for ThreeDsBridge {
    fn platform(&self, id: &str) -> Result<Platform, InstalledError> {
        if self.contains(id) {
            Ok(Platform::ThreeDs)
        } else {
            Err(InstalledError::DeviceUnavailable)
        }
    }
    fn binding(&self, id: &str) -> Result<String, InstalledError> {
        self.peer(id)
            .and_then(|p| p.config.lock().ok()?.binding.clone())
            .ok_or(InstalledError::DeviceUnavailable)
    }
    fn read<'a>(
        &'a self,
        id: &'a str,
        app_ids: &'a [String],
    ) -> InstalledFuture<'a, InstallationObservation> {
        Box::pin(async move {
            let peer = self.peer(id).ok_or(InstalledError::DeviceUnavailable)?;
            let mut state = peer.session.lock().await;
            read_info(&peer, &mut state)
                .await
                .map_err(|_| InstalledError::ReadFailed)?;
            let result = state.connection.as_mut().unwrap().inventory().await;
            if result.is_err() {
                state.connection = None;
            }
            let managed = result
                .map_err(|_| InstalledError::ReadFailed)?
                .into_iter()
                .filter(|item| app_ids.contains(&item.app_id))
                .collect();
            Ok(InstallationObservation {
                applications: vec![],
                managed,
                observed_at: crate::domain::now_millis(),
            })
        })
    }
}
struct Target {
    peer: Arc<Peer>,
    binding: String,
}
impl PackageDriver for ThreeDsBridge {
    fn target(&self, id: &str) -> Result<Arc<dyn PackageTarget>, PackageError> {
        let peer = self.peer(id).ok_or(PackageError::DeviceChanged)?;
        let binding = peer
            .config
            .lock()
            .map_err(|_| PackageError::Storage)?
            .binding
            .clone()
            .ok_or(PackageError::RuntimeRequired)?;
        Ok(Arc::new(Target { peer, binding }))
    }
}
impl PackageTarget for Target {
    fn verify_operation<'a>(&'a self, plan: &'a PackagePlan) -> PackageFuture<'a, ()> {
        Box::pin(async move {
            let mut state = self.peer.session.lock().await;
            let info = read_info(&self.peer, &mut state)
                .await
                .map_err(|_| PackageError::VerificationUnavailable)?;
            if info.binding.as_deref() != Some(&self.binding) {
                return Err(PackageError::DeviceChanged);
            }
            let result = state
                .connection
                .as_mut()
                .unwrap()
                .rpc::<wire::Operation>("operation", json!({"operation_id": plan.id}))
                .await;
            if result.is_err() {
                state.connection = None;
            }
            match result
                .map_err(|_| PackageError::VerificationUnavailable)?
                .status
                .as_str()
            {
                "verified" => Ok(()),
                "failed" => Err(PackageError::VerificationFailed),
                _ => Err(PackageError::VerificationUnavailable),
            }
        })
    }
    fn platform(&self) -> Platform {
        Platform::ThreeDs
    }
    fn binding(&self) -> &str {
        &self.binding
    }
    fn inspect<'a>(&'a self, app_ids: &'a [String]) -> PackageFuture<'a, PackageObservation> {
        Box::pin(async move {
            let mut state = self.peer.session.lock().await;
            let info = read_info(&self.peer, &mut state)
                .await
                .map_err(|_| PackageError::DeviceChanged)?;
            if info.binding.as_deref() != Some(&self.binding) {
                return Err(PackageError::DeviceChanged);
            }
            let result = state.connection.as_mut().unwrap().inventory().await;
            if result.is_err() {
                state.connection = None;
            }
            let current = record(&self.peer, &info);
            Ok(PackageObservation {
                device: current.summary,
                facts: current.facts,
                appsync: RequirementState::Unknown,
                applications: vec![],
                managed: result
                    .map_err(|_| PackageError::VerificationUnavailable)?
                    .into_iter()
                    .filter(|r| app_ids.contains(&r.app_id))
                    .collect(),
            })
        })
    }
    fn write<'a>(
        &'a self,
        plan: &'a PackagePlan,
        path: Option<&'a Path>,
        cancel: Arc<StoreCancellation>,
        observer: Arc<dyn PackageObserver>,
    ) -> PackageFuture<'a, ()> {
        Box::pin(async move {
            let managed = plan.managed().ok_or(PackageError::Incompatible)?;
            let mut state = self.peer.session.lock().await;
            let info = read_info(&self.peer, &mut state)
                .await
                .map_err(|_| PackageError::DeviceChanged)?;
            if info.binding.as_deref() != Some(&self.binding) {
                return Err(PackageError::DeviceChanged);
            }
            let connection = state
                .connection
                .as_mut()
                .ok_or(PackageError::DeviceChanged)?;
            let mut identity = serde_json::to_value(&managed.native_identity)
                .map_err(|_| PackageError::InvalidAction)?;
            if plan.action == PackageAction::Uninstall {
                identity = serde_json::to_value(
                    managed
                        .previous
                        .as_ref()
                        .and_then(|p| p.native_identity.as_ref()),
                )
                .map_err(|_| PackageError::InvalidAction)?;
            }
            let metadata = json!({"operation_id":plan.id,"action":plan.action,"app_id":plan.app_id,"delivery":managed.delivery,"format":managed.format,
            "installation_id":if plan.action==PackageAction::Install {None}else{Some(&managed.installation_id)},"expected_generation":managed.expected_generation,
            "native_identity":identity,"sha256":plan.artifact.as_ref().map(|a|&a.blob.sha256),"size_bytes":plan.artifact.as_ref().map_or(0,|a|a.blob.size_bytes),
            "version":plan.version,"revision":plan.revision,"release_id":plan.release_id,"artifact_id":plan.artifact.as_ref().map(|a|&a.id),"build_id":plan.artifact.as_ref().map(|a|&a.build_id),"delete_data":plan.delete_data});
            let result = async {
                connection
                    .rpc::<Value>("begin", metadata)
                    .await
                    .map_err(package_error)?;
                if let Some(path) = path
                    && let Err(error) =
                        upload(connection, path, plan, &cancel, observer.as_ref()).await
                {
                    // The device keeps the operation open until told otherwise;
                    // an abort is attempted whatever interrupted the upload.
                    let _ = connection
                        .rpc::<Value>("abort", json!({"operation_id":plan.id}))
                        .await;
                    return Err(error);
                }
                observer.before_submit()?;
                connection
                    .rpc::<Value>("commit", json!({"operation_id":plan.id}))
                    .await
                    .map_err(package_error)?;
                observer.progress(PackageProgress::Submitted);
                for _ in 0..1200 {
                    let operation: wire::Operation = connection
                        .rpc("operation", json!({"operation_id":plan.id}))
                        .await
                        .map_err(package_error)?;
                    match operation.status.as_str() {
                        "verified" => return Ok(()),
                        "failed" => return Err(PackageError::WriteRejected),
                        "unverified" => return Err(PackageError::VerificationUnavailable),
                        _ => {}
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
                Err(PackageError::VerificationUnavailable)
            }
            .await;
            if result.is_err() {
                state.connection = None;
            }
            result
        })
    }
}
/// Streams the staged artifact in framed chunks. Cancellation and local read
/// failures are reported to the caller, which owns the abort.
async fn upload(
    connection: &mut Connection,
    path: &Path,
    plan: &PackagePlan,
    cancel: &StoreCancellation,
    observer: &dyn PackageObserver,
) -> Result<(), PackageError> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|_| PackageError::ChecksumMismatch)?;
    let total = plan.artifact.as_ref().map_or(0, |a| a.blob.size_bytes);
    let mut offset = 0u32;
    let mut buffer = vec![0; 60 * 1024];
    loop {
        if cancel.is_cancelled() {
            return Err(PackageError::Cancelled);
        }
        let read = file
            .read(&mut buffer[4..])
            .await
            .map_err(|_| PackageError::ChecksumMismatch)?;
        if read == 0 {
            return Ok(());
        }
        buffer[..4].copy_from_slice(&offset.to_le_bytes());
        connection
            .send(0x50, &buffer[..read + 4])
            .await
            .map_err(package_error)?;
        offset += read as u32;
        observer.progress(PackageProgress::Transfer {
            bytes: u64::from(offset),
            total,
        });
    }
}
fn package_error(error: WireError) -> PackageError {
    match error {
        WireError::Rejected(message) if message.contains("state changed") => {
            PackageError::StateChanged
        }
        WireError::Rejected(message) if message.contains("checksum") => {
            PackageError::ChecksumMismatch
        }
        WireError::Rejected(message) if message.contains("busy") => PackageError::Busy,
        WireError::Rejected(_) => PackageError::WriteRejected,
        _ => PackageError::DeviceChanged,
    }
}
