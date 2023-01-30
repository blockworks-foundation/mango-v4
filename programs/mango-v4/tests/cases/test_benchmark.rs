use super::*;

#[tokio::test]
async fn test_benchmark() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    send_tx(solana, BenchmarkInstruction {}).await.unwrap();

    Ok(())
}
