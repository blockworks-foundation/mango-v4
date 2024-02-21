use mango_v4_client::MangoClient;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use tracing::*;

pub async fn report_regularly(client: Arc<MangoClient>, min_health_ratio: f64) {
    let mut interval = mango_v4_client::delay_interval(std::time::Duration::from_secs(600));
    loop {
        interval.tick().await;
        if let Err(e) = report(&client, min_health_ratio).await {
            warn!("telemetry call failed: {e:?}");
        }
    }
}

async fn report(client: &MangoClient, min_health_ratio: f64) -> anyhow::Result<()> {
    let message = serde_json::json!({
        "mango_account_pk": client.mango_account_address.to_string(),
        "target_init_health": min_health_ratio,
        "timestamp": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    })
    .to_string();

    let signature = client.owner.sign_message(message.as_bytes());
    let payload = serde_json::json!({
        "wallet_pk": client.owner.pubkey().to_string(),
        "message": message,
        "signature": signature.to_string(),
    });

    let url = "https://api.mngo.cloud/data/v4/user-data/liquidator-capacity";
    let response = client.http_client.post(url).json(&payload).send().await?;
    let res_text = response.text().await?;
    if res_text != "null" {
        anyhow::bail!("unexpected reporting response: {res_text}");
    }

    Ok(())
}
