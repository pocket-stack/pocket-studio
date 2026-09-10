use crate::domain::three_ds::ThreeDsInstallation;
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    time::timeout,
};

pub const PORT: u16 = 8131;
const MAX_FRAME: usize = 64 * 1024;
#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("3DS connection is unavailable")]
    Unavailable,
    #[error("3DS pairing was rejected")]
    Authentication,
    #[error("3DS returned an invalid management response")]
    Protocol,
    #[error("3DS rejected the requested operation: {0}")]
    Rejected(String),
}
pub struct Connection {
    stream: TcpStream,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Receipt {
    pub app_id: String,
    pub version: String,
    pub build_id: String,
    pub target_id: String,
    pub host_abi: u64,
    pub runtime_id: String,
    pub runtime_version: String,
    pub container_kind: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Info {
    pub binding: Option<String>,
    pub model: Option<String>,
    pub region: Option<String>,
    pub firmware: Option<String>,
    pub cfw: Option<bool>,
    pub battery_percent: Option<u8>,
    pub storage_total_bytes: Option<u64>,
    pub storage_free_bytes: Option<u64>,
    pub native_management: bool,
    pub host: Receipt,
    pub management_version: u32,
    pub container_id: String,
    pub launcher: bool,
    pub busy: bool,
}
#[derive(Debug, Deserialize)]
pub struct Inventory {
    pub installations: Vec<ThreeDsInstallation>,
    pub complete: bool,
    pub next: u32,
}
#[derive(Debug, Deserialize)]
pub struct Operation {
    pub status: String,
    pub error: Option<String>,
}

pub fn key_id(token: &[u8; 32]) -> u64 {
    token.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
impl Connection {
    pub async fn connect(address: SocketAddr, token: &[u8; 32]) -> Result<Self, WireError> {
        let mut stream = timeout(Duration::from_secs(3), TcpStream::connect(address))
            .await
            .map_err(|_| WireError::Unavailable)?
            .map_err(|_| WireError::Unavailable)?;
        let mut hello = Vec::with_capacity(40);
        hello.extend(0x54524b50u32.to_le_bytes());
        hello.extend([1, 0, 32, 0]);
        hello.extend(token);
        stream
            .write_all(&hello)
            .await
            .map_err(|_| WireError::Unavailable)?;
        let mut ack = [0u8; 24];
        timeout(Duration::from_secs(3), stream.read_exact(&mut ack))
            .await
            .map_err(|_| WireError::Unavailable)?
            .map_err(|_| WireError::Unavailable)?;
        if ack[..4] != 0x54524b50u32.to_le_bytes() || ack[4] != 1 {
            return Err(WireError::Protocol);
        }
        if ack[5] != 0 {
            return Err(WireError::Authentication);
        }
        Ok(Self { stream })
    }
    pub async fn send(&mut self, kind: u8, bytes: &[u8]) -> Result<(), WireError> {
        if bytes.len() > MAX_FRAME {
            return Err(WireError::Protocol);
        }
        let mut header = [0u8; 8];
        header[0] = kind;
        header[4..].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        timeout(Duration::from_secs(10), async {
            self.stream.write_all(&header).await?;
            self.stream.write_all(bytes).await
        })
        .await
        .map_err(|_| WireError::Unavailable)?
        .map_err(|_| WireError::Unavailable)
    }
    async fn frame(&mut self) -> Result<(u8, Vec<u8>), WireError> {
        let mut header = [0u8; 8];
        self.stream
            .read_exact(&mut header)
            .await
            .map_err(|_| WireError::Unavailable)?;
        let size = u32::from_le_bytes(header[4..].try_into().unwrap()) as usize;
        if header[1..4] != [0, 0, 0] || size > MAX_FRAME {
            return Err(WireError::Protocol);
        }
        let mut data = vec![0; size];
        self.stream
            .read_exact(&mut data)
            .await
            .map_err(|_| WireError::Unavailable)?;
        Ok((header[0], data))
    }
    pub async fn rpc<T: DeserializeOwned>(
        &mut self,
        command: &str,
        mut payload: Value,
    ) -> Result<T, WireError> {
        let id = uuid::Uuid::new_v4().to_string();
        let object = payload.as_object_mut().ok_or(WireError::Protocol)?;
        object.insert("t".into(), json!(format!("studio.{command}")));
        object.insert("id".into(), json!(&id));
        let bytes = serde_json::to_vec(&payload).map_err(|_| WireError::Protocol)?;
        if bytes.len() > 16 * 1024 {
            return Err(WireError::Protocol);
        }
        self.send(0x10, &bytes).await?;
        timeout(Duration::from_secs(30), async {
            loop {
                let (kind, data) = self.frame().await?;
                if kind != 0x10 {
                    continue;
                }
                let response: Value =
                    serde_json::from_slice(&data).map_err(|_| WireError::Protocol)?;
                if response["t"] != "studio.result" || response["id"] != id {
                    continue;
                }
                if response["ok"] != true {
                    return Err(WireError::Rejected(
                        response["error"]
                            .as_str()
                            .unwrap_or("operation rejected")
                            .chars()
                            .take(256)
                            .collect(),
                    ));
                }
                return serde_json::from_value(response["data"].clone())
                    .map_err(|_| WireError::Protocol);
            }
        })
        .await
        .map_err(|_| WireError::Unavailable)?
    }
    pub async fn info(&mut self) -> Result<Info, WireError> {
        let info: Info = self.rpc("info", json!({})).await?;
        if info.management_version != 1
            || info.host.target_id != "3ds-dev"
            || info.host.runtime_id != "pocketjs-3ds"
            || info.host.host_abi == 0
        {
            return Err(WireError::Protocol);
        }
        Ok(info)
    }
    pub async fn inventory(&mut self) -> Result<Vec<ThreeDsInstallation>, WireError> {
        let mut result = vec![];
        let mut offset = 0;
        loop {
            let page: Inventory = self.rpc("list", json!({"offset":offset})).await?;
            if !page.complete || result.len() + page.installations.len() > 1024 {
                return Err(WireError::Protocol);
            }
            result.extend(page.installations);
            if page.next == 0 {
                return Ok(result);
            }
            if page.next <= offset {
                return Err(WireError::Protocol);
            }
            offset = page.next;
        }
    }
}

pub async fn discover(addresses: &[IpAddr]) -> Result<Vec<(u64, SocketAddr)>, WireError> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|_| WireError::Unavailable)?;
    socket
        .set_broadcast(true)
        .map_err(|_| WireError::Unavailable)?;
    let request = [0x50, 0x4b, 0x52, 0x44, 1, 1, 0, 0];
    for address in addresses {
        let _ = socket
            .send_to(&request, SocketAddr::new(*address, PORT))
            .await;
    }
    let mut found = vec![];
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    loop {
        let mut reply = [0u8; 65];
        let received = tokio::time::timeout_at(deadline, socket.recv_from(&mut reply)).await;
        let Ok(Ok((length, sender))) = received else {
            break;
        };
        if length != 64
            || reply[..4] != 0x44524b50u32.to_le_bytes()
            || reply[4] != 1
            || reply[5] != 2
        {
            continue;
        }
        let id = u64::from_le_bytes(reply[24..32].try_into().unwrap());
        let port = u16::from_le_bytes(reply[8..10].try_into().unwrap());
        if port != 0 && !found.iter().any(|(prior, _)| *prior == id) {
            found.push((id, SocketAddr::new(sender.ip(), port)));
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    #[tokio::test]
    async fn fragmented_handshake_and_rpc_keep_request_identity_and_bound_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut hello = [0; 40];
            stream.read_exact(&mut hello).await.unwrap();
            assert_eq!(&hello[8..], &[9; 32]);
            let mut ack = [0; 24];
            ack[..4].copy_from_slice(&0x54524b50u32.to_le_bytes());
            ack[4] = 1;
            for part in ack.chunks(3) {
                stream.write_all(part).await.unwrap();
            }
            let mut header = [0; 8];
            stream.read_exact(&mut header).await.unwrap();
            assert_eq!(header[0], 0x10);
            let mut payload = vec![0; u32::from_le_bytes(header[4..].try_into().unwrap()) as usize];
            stream.read_exact(&mut payload).await.unwrap();
            let request: Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(request["t"], "studio.operation");
            for id in [json!("unrelated-response"), request["id"].clone()] {
                let bytes = serde_json::to_vec(
                    &json!({"t":"studio.result","id":id,"ok":true,"data":{"status":"verified"}}),
                )
                .unwrap();
                header[4..].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
                stream.write_all(&header).await.unwrap();
                for part in bytes.chunks(7) {
                    stream.write_all(part).await.unwrap();
                }
            }
            header[4..].copy_from_slice(&((MAX_FRAME + 1) as u32).to_le_bytes());
            stream.write_all(&header).await.unwrap();
        });
        let mut client = Connection::connect(address, &[9; 32]).await.unwrap();
        let result: Operation = client
            .rpc("operation", json!({"operation_id":"already-submitted"}))
            .await
            .unwrap();
        assert_eq!(result.status, "verified");
        assert!(matches!(client.frame().await, Err(WireError::Protocol)));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn wrong_pairing_stops_before_management_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut hello = [0; 40];
            stream.read_exact(&mut hello).await.unwrap();
            let mut ack = [0; 24];
            ack[..4].copy_from_slice(&0x54524b50u32.to_le_bytes());
            ack[4] = 1;
            ack[5] = 1;
            stream.write_all(&ack).await.unwrap();
            assert_eq!(
                stream.read_u8().await.unwrap_err().kind(),
                std::io::ErrorKind::UnexpectedEof
            );
        });
        assert!(matches!(
            Connection::connect(address, &[8; 32]).await,
            Err(WireError::Authentication)
        ));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn discovery_uses_the_same_magic_port_and_key_identifier_as_native() {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, PORT)).await.unwrap();
        let server = tokio::spawn(async move {
            let mut request = [0; 8];
            let (_, sender) = socket.recv_from(&mut request).await.unwrap();
            assert_eq!(&request[..4], &0x44524b50u32.to_le_bytes());
            let mut response = [0; 64];
            response[..4].copy_from_slice(&request[..4]);
            response[4] = 1;
            response[5] = 2;
            response[8..10].copy_from_slice(&PORT.to_le_bytes());
            response[24..32].copy_from_slice(&key_id(&[5; 32]).to_le_bytes());
            socket.send_to(&response, sender).await.unwrap();
        });
        let result = discover(&[Ipv4Addr::LOCALHOST.into()]).await.unwrap();
        assert_eq!(result[0].0, key_id(&[5; 32]));
        assert_eq!(result[0].1.port(), PORT);
        server.await.unwrap();
    }
    use std::net::Ipv4Addr;
}
