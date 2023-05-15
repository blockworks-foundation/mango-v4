use super::*;

#[tokio::test]
async fn test_benchmark() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let solana = &context.solana.clone();

    send_tx(
        solana,
        BenchmarkInstruction {
            event_queue: solana
                .create_account_for_type::<EventQueue>(&mango_v4::id())
                .await,
        },
    )
    .await
    .unwrap();

    Ok(())
}
