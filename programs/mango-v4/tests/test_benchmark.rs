#![cfg(feature = "test-bpf")]

use program_test::*;
use solana_program_test::*;

mod program_test;

#[tokio::test]
async fn test_benchmark() -> Result<(), BanksClientError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    send_tx(solana, BenchmarkInstruction {}).await.unwrap();

    Ok(())
}
