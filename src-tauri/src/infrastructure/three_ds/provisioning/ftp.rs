//! ftpd provisioning over suppaftp's Tokio client. Studio owns the transfer
//! limits, target binding and verified file promotion; the library owns FTP.
use super::SetupError;
use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use suppaftp::{FtpError, FtpResult, Status, tokio::AsyncFtpStream, types::FileType};
use tokio::{io::AsyncReadExt, time::timeout};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(120);

async fn bounded<T>(
    duration: Duration,
    operation: impl Future<Output = FtpResult<T>>,
) -> FtpResult<T> {
    timeout(duration, operation).await.map_err(|_| {
        FtpError::ConnectionError(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "FTP operation timed out",
        ))
    })?
}

fn setup_error(error: FtpError) -> SetupError {
    match error {
        FtpError::ConnectionError(_) => SetupError::FtpConnection,
        FtpError::UnexpectedResponse(response) if response.status == Status::NotLoggedIn => {
            SetupError::FtpAuthentication
        }
        FtpError::UnexpectedResponse(response) => {
            // Do not log the response body: it can echo credentials or device paths.
            tracing::warn!(
                reply_code = response.status.code(),
                "ftpd rejected an operation"
            );
            SetupError::FtpProtocol
        }
        _ => SetupError::FtpProtocol,
    }
}

// ftpd resolves the parent before RETR: ENOENT is 553 for a missing parent
// and 450 for a missing leaf. A permission/filename error never establishes
// absence, which would otherwise allow replacing an unreadable pairing key.
fn missing_path(error: &FtpError) -> bool {
    let FtpError::UnexpectedResponse(response) = error else {
        return false;
    };
    matches!(response.status.code(), 450 | 550 | 553)
        && matches!(
            std::str::from_utf8(&response.body)
                .ok()
                .and_then(|body| body.trim_end().split_once(' ').map(|(_, reason)| reason)),
            Some("No such file or directory" | "Not found" | "File not found")
        )
}

fn check_argument(value: &str) -> Result<(), SetupError> {
    if value.len() > 1024 || value.contains(['\r', '\n', '\0']) {
        return Err(SetupError::FtpProtocol);
    }
    Ok(())
}

pub struct Ftp {
    stream: AsyncFtpStream,
}
impl Ftp {
    pub async fn connect(
        address: IpAddr,
        port: u16,
        username: &str,
        password: &str,
    ) -> Result<Self, SetupError> {
        check_argument(username)?;
        check_argument(password)?;
        let mut stream = bounded(
            COMMAND_TIMEOUT,
            AsyncFtpStream::connect_timeout(SocketAddr::new(address, port), Duration::from_secs(5)),
        )
        .await
        .map_err(setup_error)?;
        // Ignore the host advertised by PASV. Data and credentials must stay on
        // the control peer explicitly selected by the user.
        stream.set_passive_nat_workaround(true);
        bounded(COMMAND_TIMEOUT, stream.login(username, password))
            .await
            .map_err(setup_error)?;
        bounded(COMMAND_TIMEOUT, stream.transfer_type(FileType::Binary))
            .await
            .map_err(setup_error)?;
        Ok(Self { stream })
    }
    pub async fn exists_dir(&mut self, path: &str) -> Result<bool, SetupError> {
        check_argument(path)?;
        match bounded(COMMAND_TIMEOUT, self.stream.cwd(format!("/{path}"))).await {
            Ok(()) => Ok(true),
            Err(error) if missing_path(&error) => Ok(false),
            Err(error) => Err(setup_error(error)),
        }
    }
    pub async fn verify_sd_root(&mut self) -> Result<(), SetupError> {
        if !self.exists_dir("Nintendo 3DS").await?
            || self.read("boot.firm", 16 * 1024 * 1024).await?.is_none()
        {
            return Err(SetupError::FtpRoot);
        }
        Ok(())
    }
    pub async fn read(&mut self, path: &str, limit: u64) -> Result<Option<Vec<u8>>, SetupError> {
        check_argument(path)?;
        let data = match bounded(
            COMMAND_TIMEOUT,
            self.stream.retr_as_stream(format!("/{path}")),
        )
        .await
        {
            Ok(data) => data,
            Err(error) if missing_path(&error) => return Ok(None),
            Err(error) => return Err(setup_error(error)),
        };
        let mut data = data.take(limit.saturating_add(1));
        let mut bytes = vec![];
        bounded(TRANSFER_TIMEOUT, async {
            data.read_to_end(&mut bytes)
                .await
                .map_err(FtpError::ConnectionError)
        })
        .await
        .map_err(setup_error)?;
        if bytes.len() as u64 > limit {
            // Caller abandons this session; do not promote an oversized payload.
            return Err(SetupError::Storage);
        }
        bounded(
            COMMAND_TIMEOUT,
            self.stream.finalize_retr_stream(data.into_inner()),
        )
        .await
        .map_err(setup_error)?;
        Ok(Some(bytes))
    }
    pub async fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), SetupError> {
        check_argument(path)?;
        let mut directory = String::new();
        let parts: Vec<_> = path.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            directory.push('/');
            directory.push_str(part);
            match bounded(COMMAND_TIMEOUT, self.stream.mkdir(&directory)).await {
                Ok(()) => {}
                // ftpd's successful MKD is 250, which suppaftp's mkdir does not
                // currently accept (it accepts 200/257). The reply is consumed.
                Err(FtpError::UnexpectedResponse(response))
                    if response.status == Status::RequestedFileActionOk => {}
                Err(FtpError::UnexpectedResponse(response))
                    if response.status == Status::FileUnavailable =>
                {
                    if !self.exists_dir(directory.trim_start_matches('/')).await? {
                        return Err(SetupError::FtpProtocol);
                    }
                }
                Err(error) => return Err(setup_error(error)),
            }
        }
        let temporary = format!("/{path}.studio-{}.tmp", uuid::Uuid::new_v4());
        let mut source = bytes;
        bounded(
            TRANSFER_TIMEOUT,
            self.stream.put_file(&temporary, &mut source),
        )
        .await
        .map_err(setup_error)?;
        if let Err(error) = self.promote(&temporary, path, bytes).await {
            // Never leave a half-verified staging file on the card. The
            // original failure is what the user needs to see.
            let _ = bounded(COMMAND_TIMEOUT, self.stream.rm(&temporary)).await;
            return Err(error);
        }
        if self.read(path, bytes.len() as u64).await?.as_deref() != Some(bytes) {
            return Err(SetupError::Storage);
        }
        Ok(())
    }
    /// Reads the staged copy back and moves it into place only when it matches.
    async fn promote(
        &mut self,
        temporary: &str,
        path: &str,
        bytes: &[u8],
    ) -> Result<(), SetupError> {
        if self
            .read(temporary.trim_start_matches('/'), bytes.len() as u64)
            .await?
            .as_deref()
            != Some(bytes)
        {
            return Err(SetupError::Storage);
        }
        bounded(
            COMMAND_TIMEOUT,
            self.stream.rename(temporary, &format!("/{path}")),
        )
        .await
        .map_err(setup_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
        task::JoinHandle,
    };

    struct Transcript {
        files: HashMap<String, Vec<u8>>,
        commands: Vec<String>,
    }
    async fn server(
        cwd_code: u16,
        mkd_code: u16,
        corrupt_readback: bool,
    ) -> (SocketAddr, JoinHandle<Transcript>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            stream.get_mut().write_all(b"220 Hello!\r\n").await.unwrap();
            let mut directories = HashSet::from([
                "/".to_owned(),
                "/Nintendo 3DS".to_owned(),
                "/pocketjs".to_owned(),
                "/3ds".to_owned(),
            ]);
            let mut files =
                HashMap::from([("/boot.firm".to_owned(), b"firmware fixture".to_vec())]);
            let mut data: Option<TcpListener> = None;
            let mut rename = None;
            let mut commands = vec![];
            loop {
                let mut line = String::new();
                match stream.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {}
                    // Rejecting an oversized file abandons the control session
                    // before its transfer-complete reply is consumed.
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
                    Err(error) => panic!("mock FTP control read failed: {error}"),
                }
                let (command, arg) = line
                    .trim_end_matches(['\r', '\n'])
                    .split_once(' ')
                    .unwrap_or((line.trim(), ""));
                commands.push(format!("{command} {arg}"));
                let reply = match command {
                    "USER" if arg == "password-user" => "331 Password required\r\n".into(),
                    "USER" => "230 OK\r\n".into(),
                    "PASS" if arg == "synthetic-password" => "230 OK\r\n".into(),
                    "PASS" => "530 Login incorrect\r\n".into(),
                    "TYPE" => "200 OK\r\n".into(),
                    "CWD" => {
                        if arg == "/denied" {
                            "530 Not logged in\r\n".into()
                        } else if arg == "/write-denied" {
                            "550 Permission denied\r\n".into()
                        } else if cwd_code == 550 {
                            "550 No such file or directory\r\n".into()
                        } else if directories.contains(arg) {
                            format!("{cwd_code} OK\r\n")
                        } else {
                            "550 Not found\r\n".into()
                        }
                    }
                    "MKD" => {
                        if arg == "/write-denied" {
                            "550 Permission denied\r\n".into()
                        } else if directories.insert(arg.into()) {
                            format!("{mkd_code} OK\r\n")
                        } else {
                            "550 Already exists\r\n".into()
                        }
                    }
                    "PASV" => {
                        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let port = listener.local_addr().unwrap().port();
                        data = Some(listener);
                        // The client must ignore the advertised host and pin the
                        // data connection to the selected control peer.
                        format!(
                            "227 Passive (203,0,113,7,{},{})\r\n",
                            port / 256,
                            port % 256
                        )
                    }
                    "STOR" | "RETR" => {
                        let listener = data.take().expect("PASV precedes a transfer");
                        let (mut socket, _) = listener.accept().await.unwrap();
                        let parent = arg
                            .rsplit_once('/')
                            .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
                            .unwrap();
                        if arg == "/permission-denied" {
                            "553 Permission denied\r\n".into()
                        } else if arg == "/unreadable" {
                            "550 Permission denied\r\n".into()
                        } else if !directories.contains(parent) {
                            "553 No such file or directory\r\n".into()
                        } else if command == "RETR" && !files.contains_key(arg) {
                            "450 No such file or directory\r\n".into()
                        } else {
                            stream
                                .get_mut()
                                .write_all(b"150 Opening data\r\n")
                                .await
                                .unwrap();
                            if command == "STOR" {
                                let mut bytes = vec![];
                                socket.read_to_end(&mut bytes).await.unwrap();
                                files.insert(arg.into(), bytes);
                            } else {
                                let bytes = if corrupt_readback && arg.contains(".studio-") {
                                    b"damaged".as_slice()
                                } else {
                                    files[arg].as_slice()
                                };
                                socket.write_all(bytes).await.unwrap();
                                socket.shutdown().await.unwrap();
                            }
                            drop(socket);
                            "226 Transfer complete\r\n".into()
                        }
                    }
                    "RNFR" => {
                        assert!(files.contains_key(arg));
                        rename = Some(arg.to_owned());
                        "350 Ready\r\n".into()
                    }
                    "RNTO" => {
                        let from = rename.take().unwrap();
                        let bytes = files.remove(&from).unwrap();
                        files.insert(arg.into(), bytes);
                        "250 Renamed\r\n".into()
                    }
                    "DELE" => {
                        if files.remove(arg).is_some() {
                            "250 Deleted\r\n".into()
                        } else {
                            "550 No such file or directory\r\n".into()
                        }
                    }
                    other => panic!("unexpected command {other}"),
                };
                match stream.get_mut().write_all(reply.as_bytes()).await {
                    Ok(()) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
                        ) =>
                    {
                        break;
                    }
                    Err(error) => panic!("mock FTP control write failed: {error}"),
                }
            }
            Transcript { files, commands }
        });
        (address, handle)
    }
    async fn finished(handle: JoinHandle<Transcript>) -> Transcript {
        timeout(Duration::from_secs(3), handle)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn directory_inspection_accepts_ftpd_and_standard_replies_without_writing() {
        for code in [200, 250] {
            let (address, handle) = server(code, 250, false).await;
            let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
                .await
                .unwrap();
            ftp.verify_sd_root().await.unwrap();
            assert!(!ftp.exists_dir("missing").await.unwrap());
            assert!(matches!(
                ftp.exists_dir("denied").await,
                Err(SetupError::FtpAuthentication)
            ));
            drop(ftp);
            let transcript = finished(handle).await;
            assert!(transcript.commands.iter().all(|c| {
                !["STOR ", "MKD ", "RNFR ", "RNTO "]
                    .iter()
                    .any(|prefix| c.starts_with(prefix))
            }));
        }
    }
    #[tokio::test]
    async fn missing_remote_root_has_an_ftp_error_instead_of_a_local_card_prompt() {
        let (address, handle) = server(550, 250, false).await;
        let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
            .await
            .unwrap();
        let error = ftp.verify_sd_root().await.unwrap_err();
        assert_eq!(error.code(), "ftpRootUnavailable");
        drop(ftp);
        finished(handle).await;
    }
    #[tokio::test]
    async fn writes_handle_existing_ftpd_directories_and_verify_both_readbacks() {
        for (cwd, mkd) in [(200, 250), (250, 257)] {
            let (address, handle) = server(cwd, mkd, false).await;
            let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
                .await
                .unwrap();
            ftp.write("pocketjs/runtime/dev.key", b"synthetic pairing fixture")
                .await
                .unwrap();
            drop(ftp);
            let transcript = finished(handle).await;
            assert_eq!(
                transcript.files["/pocketjs/runtime/dev.key"],
                b"synthetic pairing fixture"
            );
            assert!(transcript.commands.contains(&"CWD /pocketjs".to_owned()));
            assert!(
                transcript
                    .commands
                    .contains(&"RETR /pocketjs/runtime/dev.key".to_owned())
            );
        }
    }
    #[tokio::test]
    async fn a_bad_readback_never_promotes_the_staged_file() {
        let (address, handle) = server(200, 250, true).await;
        let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
            .await
            .unwrap();
        assert!(matches!(
            ftp.write("pocketjs/runtime/dev.key", b"synthetic pairing fixture")
                .await,
            Err(SetupError::Storage)
        ));
        drop(ftp);
        let transcript = finished(handle).await;
        assert!(!transcript.files.contains_key("/pocketjs/runtime/dev.key"));
        assert!(!transcript.commands.iter().any(|c| c.starts_with("RNFR ")));
        // The rejected staging copy is removed instead of littering the card.
        assert!(transcript.commands.iter().any(|c| c.starts_with("DELE ")));
        assert!(
            transcript
                .files
                .keys()
                .all(|name| !name.contains(".studio-"))
        );
    }
    #[tokio::test]
    async fn first_install_reads_absent_key_and_launcher_before_creating_them() {
        let (address, handle) = server(200, 250, false).await;
        let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
            .await
            .unwrap();
        ftp.verify_sd_root().await.unwrap();
        let key = "pocketjs/runtime/dev.key";
        let launcher = "3ds/dev.pocket-stack.launcher/boot.3dsx";
        assert!(ftp.read(key, 65).await.unwrap().is_none());
        assert!(ftp.read(launcher, 4096).await.unwrap().is_none());
        assert!(ftp.read("boot-missing.firm", 4096).await.unwrap().is_none());
        ftp.write(key, b"synthetic pairing fixture").await.unwrap();
        ftp.write(launcher, b"synthetic launcher fixture")
            .await
            .unwrap();
        assert_eq!(
            ftp.read(key, 65).await.unwrap().unwrap(),
            b"synthetic pairing fixture"
        );
        assert_eq!(
            ftp.read(launcher, 4096).await.unwrap().unwrap(),
            b"synthetic launcher fixture"
        );
        drop(ftp);
        let transcript = finished(handle).await;
        assert!(
            transcript
                .commands
                .iter()
                .position(|c| c == "RETR /pocketjs/runtime/dev.key")
                .unwrap()
                < transcript
                    .commands
                    .iter()
                    .position(|c| c.starts_with("MKD "))
                    .unwrap()
        );
    }
    #[tokio::test]
    async fn permission_failures_do_not_masquerade_as_missing_optional_files() {
        let (address, handle) = server(200, 250, false).await;
        let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
            .await
            .unwrap();
        for path in ["permission-denied", "unreadable"] {
            assert!(matches!(
                ftp.read(path, 65).await,
                Err(SetupError::FtpProtocol)
            ));
        }
        assert!(matches!(
            ftp.write("write-denied/dev.key", b"synthetic pairing fixture")
                .await,
            Err(SetupError::FtpProtocol)
        ));
        drop(ftp);
        let transcript = finished(handle).await;
        assert!(!transcript.commands.iter().any(|c| c.starts_with("STOR ")));
        assert!(!missing_path(&FtpError::UnexpectedResponse(
            suppaftp::types::Response::new(Status::from(553), b"553 Invalid filename\r\n".to_vec())
        )));
        assert!(!missing_path(&FtpError::UnexpectedResponse(
            suppaftp::types::Response::new(Status::from(450), b"450 File busy\r\n".to_vec())
        )));
    }

    #[tokio::test]
    async fn password_login_preserves_authentication_errors() {
        for (password, succeeds) in [("synthetic-password", true), ("incorrect", false)] {
            let (address, handle) = server(200, 250, false).await;
            let result =
                Ftp::connect(address.ip(), address.port(), "password-user", password).await;
            if succeeds {
                let mut ftp = result.unwrap();
                ftp.verify_sd_root().await.unwrap();
            } else {
                assert!(matches!(result, Err(SetupError::FtpAuthentication)));
            }
            let transcript = finished(handle).await;
            assert!(!transcript.commands.iter().any(|c| c.starts_with("STOR ")));
        }
    }

    #[tokio::test]
    async fn oversized_download_is_rejected_without_writes() {
        let (address, handle) = server(200, 250, false).await;
        let mut ftp = Ftp::connect(address.ip(), address.port(), "anonymous", "")
            .await
            .unwrap();
        assert!(matches!(
            ftp.read("boot.firm", 4).await,
            Err(SetupError::Storage)
        ));
        drop(ftp);
        let transcript = finished(handle).await;
        assert!(!transcript.commands.iter().any(|c| c.starts_with("STOR ")));
    }
}
