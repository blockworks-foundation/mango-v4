use anyhow::anyhow;

pub fn retry<T>(request: impl Fn() -> Result<T, anchor_client::ClientError>) -> anyhow::Result<T> {
    for _i in 0..5 {
        match request() {
            Ok(res) => return Ok(res),
            Err(err) => {
                // TODO: only retry for recoverable errors
                log::error!("{:#?}", err);
                continue;
            }
        }
    }
    Err(anyhow!("Retry failed"))
}
