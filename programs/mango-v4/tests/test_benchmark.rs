#![cfg(feature = "test-bpf")]

use program_test::*;
use solana_program_test::*;
use solana_sdk::transport::TransportError;

mod program_test;

#[tokio::test]
async fn test_benchmark() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    send_tx(solana, BenchmarkInstruction {}).await.unwrap();

    Ok(())
}
