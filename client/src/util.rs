use solana_sdk::signature::Keypair;

// #[allow(dead_code)]
// pub fn retry<T>(request: impl Fn() -> Result<T, anchor_client::ClientError>) -> anyhow::Result<T> {
//     for _i in 0..5 {
//         match request() {
//             Ok(res) => return Ok(res),
//             Err(err) => {
//                 // TODO: only retry for recoverable errors
//                 log::error!("{:#?}", err);
//                 continue;
//             }
//         }
//     }
//     Err(anyhow!("Retry failed"))
// }

pub trait MyClone {
    fn clone(&self) -> Self;
}

impl MyClone for Keypair {
    fn clone(&self) -> Keypair {
        Self::from_bytes(&self.to_bytes()).unwrap()
    }
}
