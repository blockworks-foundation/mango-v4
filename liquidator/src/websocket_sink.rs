use {
    crate::Config,
    anyhow::Context,
    fixed::types::I80F48,
    futures_util::{SinkExt, StreamExt},
    log::*,
    serde::Serialize,
    //serde_derive::Serialize,
    solana_sdk::pubkey::Pubkey,
    tokio::net::{TcpListener, TcpStream},
    //std::str::FromStr,
    tokio::sync::broadcast,
};

#[derive(Clone, Debug)]
pub struct HealthInfo {
    pub account: Pubkey,
    pub being_liquidated: bool,
    pub health_fraction: I80F48, // always maint
    pub assets: I80F48,          // always maint
    pub liabilities: I80F48,     // always maint
}

#[derive(Clone, Debug)]
pub enum LiquidationCanditate {
    Start { info: HealthInfo },
    Now { info: HealthInfo },
    Stop { info: HealthInfo },
}

#[derive(Serialize)]
struct JsonRpcEnvelope<T: Serialize> {
    jsonrpc: String,
    method: String,
    params: T,
}

#[derive(Serialize)]
struct JsonRpcLiquidatablePayload {
    account: String,
    being_liquidated: bool,
    health_fraction: f64,
    assets: u64,
    liabilities: u64,
}

impl From<&HealthInfo> for JsonRpcLiquidatablePayload {
    fn from(info: &HealthInfo) -> Self {
        Self {
            account: info.account.to_string(),
            being_liquidated: info.being_liquidated,
            health_fraction: info.health_fraction.to_num::<f64>(),
            assets: info.assets.to_num::<u64>(),
            liabilities: info.liabilities.to_num::<u64>(),
        }
    }
}

fn jsonrpc_message(method: &str, payload: impl Serialize) -> String {
    serde_json::to_string(&JsonRpcEnvelope {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params: payload,
    })
    .unwrap()
}

async fn accept_connection(
    stream: TcpStream,
    mut rx: broadcast::Receiver<LiquidationCanditate>,
) -> anyhow::Result<()> {
    use tokio_tungstenite::tungstenite::Message;

    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");
    info!("new tcp client at address: {}", addr);

    let mut ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("error during the websocket handshake");
    info!("new websocket client at address: {}", addr);

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));

    loop {
        tokio::select! {
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => ws_stream.send(Message::Pong(data)).await?,
                    Some(Ok(_)) => continue, // ignore other incoming
                    None | Some(Err(_)) => break, // disconnected
                }
            },
            data = rx.recv() => {
                if data.is_err() {
                    // broadcast stream is lagging or disconnected
                    // -> drop websocket connection
                    warn!("liquidation info broadcast receiver had error: {:?}", data);
                    ws_stream.close(None).await?;
                    break;
                }

                let message = match data.unwrap() {
                    LiquidationCanditate::Start{info} => {
                        jsonrpc_message(&"candidateStart", JsonRpcLiquidatablePayload::from(&info))
                    },
                    LiquidationCanditate::Now{info} => {
                        jsonrpc_message(&"candidate",JsonRpcLiquidatablePayload::from(&info))
                    },
                    LiquidationCanditate::Stop{info} => {
                        jsonrpc_message(&"candidateStop",JsonRpcLiquidatablePayload::from(&info))
                    },
                };
                ws_stream.send(Message::Text(message)).await?;
            },
            _ = interval.tick() => {
                ws_stream.send(Message::Ping(vec![])).await?;
            },
        }
    }

    Ok(())
}

pub async fn start(config: Config) -> anyhow::Result<broadcast::Sender<LiquidationCanditate>> {
    // The channel that liquidatable event changes are sent through, to
    // be forwarded to websocket clients
    let (tx, _) = broadcast::channel(1000);

    let websocket_listener = TcpListener::bind(&config.websocket_server_bind_address)
        .await
        .context("binding websocket server")?;
    info!(
        "websocket server listening on: {}",
        &config.websocket_server_bind_address
    );
    let tx_c = tx.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = websocket_listener.accept().await {
            tokio::spawn(accept_connection(stream, tx_c.subscribe()));
        }
    });

    Ok(tx)
}
