use mango_feeds_connector::GrpcSourceConfig;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Configuration {
    pub grpc_sources: Vec<GrpcSourceConfig>,
}
