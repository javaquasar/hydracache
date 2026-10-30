use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};

pub fn configure(
    config: ChitchatDiscoveryConfig,
    detector: chitchat::FailureDetectorConfig,
) -> ChitchatDiscoveryConfig {
    config.failure_detector_config(detector)
}

pub fn identity(discovery: &ChitchatDiscovery) -> &chitchat::ChitchatId {
    discovery.chitchat_id()
}

pub async fn transport(
    config: ChitchatDiscoveryConfig,
    channel: &chitchat::transport::ChannelTransport,
) -> hydracache::CacheResult<ChitchatDiscovery> {
    ChitchatDiscovery::spawn_with_transport(config, channel).await
}
