use serde_derive::Deserialize;
use services_mango_lib::env_helper::string_or_env;
use services_mango_lib::postgres_configuration::PostgresTlsConfig;

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    #[serde(deserialize_with = "string_or_env")]
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
