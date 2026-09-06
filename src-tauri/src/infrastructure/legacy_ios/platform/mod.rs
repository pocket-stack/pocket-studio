//! All desktop targets reuse existing system pairing records. Discovery never
//! falls back to claiming USB interfaces or changing a driver's binding.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::domain::device::DiscoveryIssueCode;
use legacy_ios_services::NormalBackend;

pub trait HostEnvironment: Send + Sync {
    fn normal_backend(&self) -> NormalBackend {
        NormalBackend::System
    }
    fn mux_unavailable(&self) -> DiscoveryIssueCode;
}

pub fn current() -> Box<dyn HostEnvironment> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::Macos)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::Linux)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::Windows)
    }
}
