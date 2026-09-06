//! A repeatable, opt-in hardware check using the exact adapter shipped in the
//! desktop app. It only prints masked identifiers and performs no device writes.
use pocket_studio_lib::{
    application::{
        EventSink, OperationLog, discovery::DeviceDiscovery, preparation::PreparationService,
    },
    domain::{device::DeviceEvent, log::LogEntry, operation::OperationEvent},
    infrastructure::legacy_ios::{LegacyIosProbe, preparation::LegacyPreparationDriver},
};
use std::sync::Arc;
use tracing_subscriber::prelude::*;

struct ConsoleSink;
impl EventSink for ConsoleSink {
    fn device(&self, _: DeviceEvent) {}
    fn operation(&self, _: OperationEvent) {}
    fn log(&self, _: LogEntry) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "off".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    !metadata.target().starts_with("idevice")
                })),
        )
        .init();
    let sink = Arc::new(ConsoleSink);
    let log = Arc::new(OperationLog::new(sink.clone()));
    let probe = Arc::new(LegacyIosProbe::default());
    let discovery = DeviceDiscovery::new(probe.clone(), sink.clone(), log.clone());
    let snapshot = discovery.refresh().await;
    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    if std::env::args().any(|arg| arg == "--plan-preparation") {
        let preparation = PreparationService::new(
            Arc::new(LegacyPreparationDriver::new(
                probe,
                std::env::temp_dir().join("pocket-studio-plan-check"),
            )),
            sink,
            log,
        );
        for device in &snapshot.devices {
            let plan = preparation.plan(device).await?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
    }
    Ok(())
}
