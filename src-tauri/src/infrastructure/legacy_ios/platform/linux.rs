use super::{DiscoveryIssueCode, HostEnvironment};
pub struct Linux;
impl HostEnvironment for Linux {
    fn mux_unavailable(&self) -> DiscoveryIssueCode {
        DiscoveryIssueCode::LinuxMuxUnavailable
    }
}
