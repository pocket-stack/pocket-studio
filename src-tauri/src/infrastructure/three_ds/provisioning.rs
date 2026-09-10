//! Explicitly invoked SD/ftpd provisioning. Secrets remain in the native layer.
use super::super::super::domain::store::sha256_hex;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub mod ftp;
pub use crate::application::device_setup::SetupError;
pub const KEY: &str = "pocketjs/runtime/dev.key";
pub fn token_text(token: &[u8; 32]) -> String {
    token.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn parse_token(bytes: &[u8]) -> Result<[u8; 32], SetupError> {
    let bytes = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if bytes.len() != 64 {
        return Err(SetupError::InvalidKey);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| SetupError::InvalidKey)?;
    if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(SetupError::InvalidKey);
    }
    let mut token = [0; 32];
    for (i, byte) in token.iter_mut().enumerate() {
        *byte = u8::from_str_radix(
            text.get(i * 2..i * 2 + 2).ok_or(SetupError::InvalidKey)?,
            16,
        )
        .map_err(|_| SetupError::InvalidKey)?;
    }
    Ok(token)
}
pub fn private_write(path: &Path, bytes: &[u8]) -> Result<(), SetupError> {
    let parent = path.parent().ok_or(SetupError::Storage)?;
    std::fs::create_dir_all(parent).map_err(|_| SetupError::Storage)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| SetupError::Storage)?;
    // NamedTempFile creates mode 0600 on Unix and inherits the user's private
    // application-data directory ACL on Windows. Never open the old key in place.
    file.write_all(bytes).map_err(|_| SetupError::Storage)?;
    file.as_file().sync_all().map_err(|_| SetupError::Storage)?;
    file.persist(path).map_err(|_| SetupError::Storage)?;
    Ok(())
}
#[derive(Clone)]
pub struct Card {
    root: PathBuf,
    fingerprint: String,
}
impl Card {
    pub fn open(path: &Path) -> Result<Self, SetupError> {
        let root = path.canonicalize().map_err(|_| SetupError::InvalidCard)?;
        // The markers must be real entries of this filesystem, not links into
        // another one; a decoy folder must not pass as a card.
        let entry = |name: &str| std::fs::symlink_metadata(root.join(name)).ok();
        if !entry("Nintendo 3DS").is_some_and(|m| m.is_dir())
            || !entry("boot.firm").is_some_and(|m| m.is_file())
        {
            return Err(SetupError::InvalidCard);
        }
        // Directory identity pins a plan to the mounted filesystem. It also
        // catches swapping two cards that contain identical firmware/key files.
        let fingerprint = super::storage_platform::fingerprint(&root)?;
        Ok(Self { root, fingerprint })
    }
    pub fn display(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }
    pub fn check(&self) -> Result<(), SetupError> {
        if super::storage_platform::fingerprint(&self.root)? != self.fingerprint {
            return Err(SetupError::Changed);
        }
        Ok(())
    }
    fn path(&self, relative: &str) -> Result<PathBuf, SetupError> {
        let mut path = self.root.clone();
        for part in relative.split('/') {
            if part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':']) {
                return Err(SetupError::Storage);
            }
            path.push(part);
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_symlink() => return Err(SetupError::InvalidCard),
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(SetupError::Storage),
            }
        }
        Ok(path)
    }
    pub fn read(&self, relative: &str, limit: u64) -> Result<Option<Vec<u8>>, SetupError> {
        use std::io::Read;
        self.check()?;
        let path = self.path(relative)?;
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(SetupError::Storage),
        };
        let mut bytes = vec![];
        file.take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| SetupError::Storage)?;
        if bytes.len() as u64 > limit {
            return Err(SetupError::Storage);
        }
        Ok(Some(bytes))
    }
    pub fn write(&self, relative: &str, bytes: &[u8]) -> Result<(), SetupError> {
        self.check()?;
        let path = self.path(relative)?;
        private_write(&path, bytes)?;
        let result = self
            .read(relative, bytes.len() as u64)?
            .ok_or(SetupError::Storage)?;
        if sha256_hex(bytes) != sha256_hex(&result) {
            return Err(SetupError::Storage);
        }
        Ok(())
    }
}
use crate::application::device_setup::{
    SetupDestination, SetupFuture, SetupObservation, SetupPort, SetupRequest, SetupResult,
};
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
enum Destination {
    Sd(Card),
    Ftp(ftp::Ftp),
}
impl Destination {
    async fn open(request: &SetupRequest) -> Result<Self, SetupError> {
        match &request.destination {
            SetupDestination::Sd { path } => Ok(Self::Sd(Card::open(Path::new(path))?)),
            SetupDestination::Ftp {
                address,
                port,
                username,
                password,
            } => {
                let mut ftp = ftp::Ftp::connect(
                    address.parse().map_err(|_| SetupError::FtpConnection)?,
                    *port,
                    username,
                    password,
                )
                .await?;
                ftp.verify_sd_root().await?;
                Ok(Self::Ftp(ftp))
            }
        }
    }
    async fn read(&mut self, path: &str, limit: u64) -> Result<Option<Vec<u8>>, SetupError> {
        match self {
            Self::Sd(card) => card.read(path, limit),
            Self::Ftp(ftp) => ftp.read(path, limit).await,
        }
    }
    async fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), SetupError> {
        match self {
            Self::Sd(card) => card.write(path, bytes),
            Self::Ftp(ftp) => ftp.write(path, bytes).await,
        }
    }
    async fn observe(
        &mut self,
        request: &SetupRequest,
        file: Option<&str>,
    ) -> Result<SetupObservation, SetupError> {
        let key = self.read(KEY, 65).await?;
        let token = key.as_deref().map(parse_token).transpose()?;
        let previous_digest = match file {
            Some(path) => self
                .read(path, 128 * 1024 * 1024)
                .await?
                .map(|v| sha256_hex(&v)),
            None => None,
        };
        let (label, binding) = match self {
            Self::Sd(card) => (card.display(), card.fingerprint.clone()),
            Self::Ftp(_) => {
                let SetupDestination::Ftp { address, port, .. } = &request.destination else {
                    unreachable!()
                };
                (
                    format!("ftpd {address}:{port}"),
                    format!("{address}:{port}"),
                )
            }
        };
        Ok(SetupObservation {
            label,
            binding,
            token,
            previous_digest,
        })
    }
}
pub struct NativeSetup(pub Arc<super::ThreeDsBridge>);
impl SetupPort for NativeSetup {
    fn inspect<'a>(
        &'a self,
        request: &'a SetupRequest,
        file: Option<&'a str>,
    ) -> SetupFuture<'a, SetupObservation> {
        Box::pin(async move {
            Destination::open(request)
                .await?
                .observe(request, file)
                .await
        })
    }
    fn write<'a>(
        &'a self,
        request: &'a SetupRequest,
        observed: &'a SetupObservation,
        file: Option<(&'a str, &'a Path, &'a str)>,
    ) -> SetupFuture<'a, SetupResult> {
        Box::pin(async move {
            let mut destination = Destination::open(request).await?;
            let current = destination.observe(request, file.map(|v| v.0)).await?;
            if current.binding != observed.binding
                || current.token != observed.token
                || current.previous_digest != observed.previous_digest
            {
                return Err(SetupError::Changed);
            }
            let mut token = observed.token.unwrap_or([0; 32]);
            if observed.token.is_none() {
                getrandom::fill(&mut token).map_err(|_| SetupError::Storage)?;
            }
            let address = request
                .address
                .as_deref()
                .map(str::parse::<IpAddr>)
                .transpose()
                .map_err(|_| SetupError::Device)?
                .unwrap_or(Ipv4Addr::UNSPECIFIED.into());
            // The card is the source of truth for the key: a pairing is only
            // recorded once the key has been written and read back, so a failed
            // write leaves nothing behind and a retry reuses the card's key.
            if observed.token.is_none() {
                destination
                    .write(KEY, token_text(&token).as_bytes())
                    .await?;
            }
            if let Some((relative, path, expected)) = file {
                let bytes = tokio::fs::read(path)
                    .await
                    .map_err(|_| SetupError::Storage)?;
                if sha256_hex(&bytes) != expected {
                    return Err(SetupError::Store(
                        crate::application::store::StoreError::ChecksumMismatch,
                    ));
                }
                destination.write(relative, &bytes).await?;
            }
            let pairing_id = self.0.add_pair(token, address, super::wire::PORT)?;
            Ok(SetupResult {
                pairing_id,
                files_verified: true,
                restart_required: true,
            })
        })
    }
    fn connect<'a>(&'a self, id: &'a str, address: Option<&'a str>) -> SetupFuture<'a, ()> {
        Box::pin(async move {
            if let Some(address) = address {
                let peer = self.0.peer(id).ok_or(SetupError::Device)?;
                let parsed = address.parse().map_err(|_| SetupError::Device)?;
                let previous = std::mem::replace(
                    &mut peer.config.lock().map_err(|_| SetupError::Storage)?.address,
                    parsed,
                );
                peer.session.lock().await.connection = None;
                let result = self.0.confirm(id).await;
                if result.is_err() {
                    // An address the console never answered from is not kept,
                    // in memory or on disk.
                    if let Ok(mut config) = peer.config.lock() {
                        config.address = previous;
                    }
                    peer.session.lock().await.connection = None;
                }
                return result;
            }
            use crate::application::discovery::DeviceProbe;
            self.0.scan().await;
            self.0.confirm(id).await
        })
    }
    fn hint(&self, address: Option<IpAddr>) -> Result<(), SetupError> {
        self.0.set_hint(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn card_io_is_scoped_and_rejects_swapped_cards() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Nintendo 3DS")).unwrap();
        std::fs::write(dir.path().join("boot.firm"), b"fixture").unwrap();
        let card = Card::open(dir.path()).unwrap();
        let token = [13; 32];
        card.write(KEY, token_text(&token).as_bytes()).unwrap();
        assert_eq!(
            parse_token(&card.read(KEY, 65).unwrap().unwrap()).unwrap(),
            token
        );
        assert!(card.write("../dev.key", b"no").is_err());
        let moved = dir.path().with_extension("moved");
        std::fs::rename(dir.path(), &moved).unwrap();
        std::fs::create_dir(dir.path()).unwrap();
        assert!(card.check().is_err());
        std::fs::remove_dir_all(moved).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn cannot_follow_card_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Nintendo 3DS")).unwrap();
        std::fs::write(dir.path().join("boot.firm"), b"fixture").unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("pocketjs")).unwrap();
        assert!(Card::open(dir.path()).unwrap().write(KEY, b"no").is_err());
        assert!(!outside.path().join("dev.key").exists());
    }
}
