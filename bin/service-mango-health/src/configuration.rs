use std::collections::HashSet;
use mango_feeds_connector::{MetricsConfig, SourceConfig};
use serde_derive::Deserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    pub source: SourceConfig,
    pub metrics: MetricsConfig,
    pub postgres: Option<PostgresConfiguration>,
    pub rpc_http_url: String,
    pub mango_group: String,
    pub computing_configuration: ComputingConfiguration,
    pub logging_configuration: LoggingConfiguration,
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
pub struct PostgresConfiguration {
    pub connection_string: String,
    /// Number of parallel postgres connections used for insertions
    pub connection_count: u64,
    /// Maximum batch size for inserts over one connection
    pub max_batch_size: usize,
    /// Max size of queues
    pub max_queue_size: usize,
    /// Number of queries retries before fatal error
    pub retry_query_max_count: u64,
    /// Seconds to sleep between query retries
    pub retry_query_sleep_secs: u64,
    /// Seconds to sleep between connection attempts
    pub retry_connection_sleep_secs: u64,
    /// Fatal error when the connection can't be reestablished this long
    pub fatal_connection_timeout_secs: u64,
    /// Allow invalid TLS certificates, passed to native_tls danger_accept_invalid_certs
    pub allow_invalid_certs: bool,
    pub tls: Option<PostgresTlsConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresTlsConfig {
    /// CA Cert file or env var
    pub ca_cert_path: String,
    /// PKCS12 client cert path
    pub client_key_path: String,
}
