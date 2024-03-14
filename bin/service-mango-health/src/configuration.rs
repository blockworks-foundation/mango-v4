use serde_derive::Deserialize;
use services_mango_lib::env_helper::string_or_env;
use services_mango_lib::postgres_configuration::PostgresConfiguration;
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    pub postgres: Option<PostgresConfiguration>,
    #[serde(deserialize_with = "string_or_env")]
    pub rpc_http_url: String,
    #[serde(deserialize_with = "string_or_env")]
    pub rpc_ws_url: String,
    #[serde(deserialize_with = "string_or_env")]
    pub mango_group: String,
    pub computing_configuration: ComputingConfiguration,
    pub logging_configuration: LoggingConfiguration,
    pub persistence_configuration: PersistenceConfiguration,

    pub snapshot_interval_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ComputingConfiguration {
    pub recompute_interval_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingConfiguration {
    pub log_health_to_stdout: bool,
    pub log_health_for_accounts: Option<HashSet<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PersistenceConfiguration {
    pub enabled: bool,
    pub history_time_to_live_secs: i64,
    pub persist_max_periodicity_secs: i64,
    pub max_failure_duration_secs: i64,
    pub max_retry_count: u64,
    pub snapshot_queue_length: usize,
}
