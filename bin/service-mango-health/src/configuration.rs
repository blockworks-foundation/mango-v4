use serde_derive::Deserialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    pub postgres: Option<PostgresConfiguration>,
    pub rpc_http_url: String,
    pub rpc_ws_url: String,
    pub mango_group: String,
    pub computing_configuration: ComputingConfiguration,
    pub logging_configuration: LoggingConfiguration,
    pub persistence_configuration: PersistenceConfiguration,
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
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct PostgresConfiguration {
    pub connection_string: String,
    pub allow_invalid_certs: bool,
    pub tls: Option<PostgresTlsConfig>,
    pub max_retry_count: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresTlsConfig {
    /// CA Cert file or env var
    pub ca_cert_path: String,
    /// PKCS12 client cert path
    pub client_key_path: String,
}
