use mango_feeds_connector::GrpcSourceConfig;
use serde_derive::Deserialize;
use services_mango_lib::env_helper::string_or_env;

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    #[serde(deserialize_with = "string_or_env")]
    pub mango_group: String,
    pub source_configuration: SourceConfiguration,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SourceConfiguration {
    #[serde(deserialize_with = "string_or_env")]
    pub rpc_http_url: String,
    #[serde(deserialize_with = "string_or_env")]
    pub rpc_ws_url: String,

    pub snapshot_interval_secs: u64,

    pub dedup_queue_size: usize,
    pub grpc_sources: Vec<GrpcSourceConfig>,
}
