use super::{DiscoveryIssueCode, HostEnvironment};
pub struct Macos;
impl HostEnvironment for Macos {
    fn mux_unavailable(&self) -> DiscoveryIssueCode {
        DiscoveryIssueCode::MacosMuxUnavailable
    }
}
