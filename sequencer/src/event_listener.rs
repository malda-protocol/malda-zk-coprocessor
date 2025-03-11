use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::{Filter, Log},
    transports::http::reqwest::Url,
};
use eyre::{Result, WrapErr};
use futures_util::StreamExt;
use sequencer::logger::{PipelineLogger, PipelineStep};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub struct EventConfig {
    pub ws_url: String,
    pub market: Address,
    pub event_signature: String,
    pub chain_id: u64,
}

#[derive(Debug)]
pub struct RawEvent {
    pub log: Log,
    pub market: Address,
    pub chain_id: u64,
}

pub struct EventListener {
    config: EventConfig,
    event_sender: mpsc::Sender<RawEvent>,
    logger: PipelineLogger,
}

impl EventListener {
    pub fn new(
        config: EventConfig,
        event_sender: mpsc::Sender<RawEvent>,
        logger: PipelineLogger,
    ) -> Self {
        Self {
            config,
            event_sender,
            logger,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting event listener for market={:?} chain={} event={}",
            self.config.market, self.config.chain_id, self.config.event_signature
        );

        let ws_url: Url = self
            .config
            .ws_url
            .parse()
            .wrap_err_with(|| format!("Invalid WSS URL: {}", self.config.ws_url))?;

        debug!("Connecting to WebSocket at {}", ws_url);
        let ws = WsConnect::new(ws_url);
        let provider = ProviderBuilder::new()
            .on_ws(ws)
            .await
            .wrap_err("Failed to connect to WebSocket")?;

        let filter = Filter::new()
            .event(&self.config.event_signature)
            .address(self.config.market);

        debug!("Subscribing to events with filter: {:?}", filter);
        let sub = provider.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        info!("Successfully subscribed to events");

        while let Some(log) = stream.next().await {
            debug!(
                "Received event on chain {} for market {:?}",
                self.config.chain_id, self.config.market
            );

            if let Err(e) = self
                .logger
                .log_step(
                    log.transaction_hash.expect("Log should have tx hash"),
                    PipelineStep::EventReceived {
                        chain_id: self.config.chain_id as u32,
                        block_number: u64::try_from(
                            log.block_number.expect("Log should have block number"),
                        )
                        .expect("Block number should fit in u64"),
                        market: self.config.market,
                        event_type: self.config.event_signature.clone(),
                    },
                )
                .await
            {
                error!("Failed to log event: {}", e);
            }

            let raw_event = RawEvent {
                log,
                market: self.config.market,
                chain_id: self.config.chain_id,
            };

            if let Err(e) = self.event_sender.send(raw_event).await {
                error!("Failed to send event to channel: {}", e);
            }
        }

        warn!("Event stream ended unexpectedly");
        Ok(())
    }
}
