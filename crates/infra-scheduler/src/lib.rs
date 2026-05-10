use std::time::Duration;

use domain_config::SettingsService;
use domain_core::CoreService;
use domain_stats::StatsService;
use tokio::{task::JoinHandle, time};
use tracing::{debug, warn};

const RUNTIME_STATS_INTERVAL: Duration = Duration::from_secs(10);

pub fn spawn_runtime_stats_collector(
    settings: SettingsService,
    stats: StatsService,
    core: CoreService,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = time::interval(RUNTIME_STATS_INTERVAL);
        loop {
            interval.tick().await;
            let traffic_age = match settings.traffic_age().await {
                Ok(value) => value,
                Err(error) => {
                    warn!(
                        "runtime stats collection disabled by invalid trafficAge: {}",
                        error.message()
                    );
                    0
                }
            };

            if let Err(error) = stats.collect_runtime_traffic(&core, traffic_age).await {
                debug!("runtime stats collection skipped: {}", error.message());
            }
        }
    })
}
