use super::{DiscoveryIssueCode, HostEnvironment};
pub struct Windows;
impl HostEnvironment for Windows {
    fn mux_unavailable(&self) -> DiscoveryIssueCode {
        DiscoveryIssueCode::WindowsMuxUnavailable
    }
}
