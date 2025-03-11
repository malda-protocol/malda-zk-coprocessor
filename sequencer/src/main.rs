use alloy::{
    network::EthereumWallet,
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider,
    },
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};

use eyre::Result;
use malda_rs::constants::*;
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

pub mod constants;
pub mod events;
pub mod types;

use crate::{constants::*, events::*};

mod event_listener;
use event_listener::{EventConfig, EventListener};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

mod event_processor;
use event_processor::EventProcessor;
use event_processor::ProcessedEvent;

mod proof_generator;
use proof_generator::{ProofGenerator, ProofReadyEvent};

mod transaction_manager;
use transaction_manager::{TransactionConfig, TransactionManager};

use sequencer::logger::PipelineLogger;
use std::path::PathBuf;

mod batch_event_listener;
use batch_event_listener::{BatchEventConfig, BatchEventListener};

use std::fs;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;

pub const TX_TIMEOUT: Duration = Duration::from_secs(30);

type ProviderType = alloy::providers::fillers::FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        alloy::providers::fillers::WalletFiller<EthereumWallet>,
    >,
    RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>>,
    alloy::transports::http::Http<alloy::transports::http::Client>,
    alloy::network::Ethereum,
>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with custom format
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
                .add_directive("sequencer=debug".parse()?),
        )
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(false)
        .init();

    info!("Starting sequencer...");

    // Create channels with proper capacities
    let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
    let (processed_tx, processed_rx) = mpsc::channel(PROCESSED_CHANNEL_CAPACITY);
    let (manual_tx, manual_rx) = mpsc::channel(32);
    let (proof_tx, proof_rx) = mpsc::channel::<Vec<ProofReadyEvent>>(PROOF_CHANNEL_CAPACITY);

    info!("Initialized channels");

    // Merge manual and processed events
    let processed_stream = ReceiverStream::new(processed_rx);
    let manual_stream = ReceiverStream::new(manual_rx);
    let merged_stream = processed_stream.merge(manual_stream);

    // Markets
    let markets = vec![WETH_MARKET_SEPOLIA, USDC_MARKET_SEPOLIA];
    info!("Configured markets: {:?}", markets);

    // Chain configurations
    let chain_configs = vec![
        (
            WS_URL_LINEA_SEPOLIA,
            LINEA_SEPOLIA_CHAIN_ID,
            vec![
                HOST_BORROW_ON_EXTENSION_CHAIN_SIG,
                HOST_WITHDRAW_ON_EXTENSION_CHAIN_SIG,
            ],
        ),
        (
            WS_URL_OPT_SEPOLIA,
            OPTIMISM_SEPOLIA_CHAIN_ID,
            vec![EXTENSION_SUPPLIED_SIG],
        ),
        (
            WS_URL_ETH_SEPOLIA,
            ETHEREUM_SEPOLIA_CHAIN_ID,
            vec![EXTENSION_SUPPLIED_SIG],
        ),
    ];
    info!(
        "Configured chains: {:?}",
        chain_configs
            .iter()
            .map(|(_, id, _)| id)
            .collect::<Vec<_>>()
    );

    // After initializing channels and before starting the main pipeline components
    info!("Initializing batch event listeners...");

    // Batch submitter configurations for each chain
    let batch_configs = vec![
        (WS_URL_LINEA_SEPOLIA, LINEA_SEPOLIA_CHAIN_ID),
        (WS_URL_OPT_SEPOLIA, OPTIMISM_SEPOLIA_CHAIN_ID),
        (WS_URL_ETH_SEPOLIA, ETHEREUM_SEPOLIA_CHAIN_ID),
    ];

    // Spawn batch event listeners
    let mut handles = vec![];

    let batch_logger = PipelineLogger::new(PathBuf::from("batch_pipeline.log")).await?;

    for (ws_url, chain_id) in batch_configs {
        info!(
            "Starting batch event listener for chain={}, submitter={:?}",
            chain_id, BATCH_SUBMITTER
        );

        let config = BatchEventConfig {
            ws_url: ws_url.to_string(),
            batch_submitter: BATCH_SUBMITTER,
            chain_id,
        };

        let listener = BatchEventListener::new(config, batch_logger.clone());
        let handle = tokio::spawn(async move {
            if let Err(e) = listener.start().await {
                error!("Batch event listener failed: {:?}", e);
            }
        });

        handles.push(handle);
        tokio::time::sleep(LISTENER_SPAWN_DELAY).await;
    }

    info!("All batch event listeners started");

    // Spawn event listeners
    let mut handles = vec![];

    let logger = PipelineLogger::new(PathBuf::from("batch_pipeline.log")).await?;

    for market in markets {
        for (ws_url, chain_id, events) in chain_configs.iter() {
            for event in events {
                info!(
                    "Starting listener for market={:?}, chain={}, event={}",
                    market, chain_id, event
                );

                let config = EventConfig {
                    ws_url: ws_url.to_string(),
                    market,
                    event_signature: event.to_string(),
                    chain_id: *chain_id,
                };

                let listener = EventListener::new(config, event_tx.clone(), logger.clone());
                let handle = tokio::spawn(async move {
                    if let Err(e) = listener.start().await {
                        error!("Event listener failed: {:?}", e);
                    }
                });

                handles.push(handle);
                tokio::time::sleep(LISTENER_SPAWN_DELAY).await;
            }
        }
    }

    info!("All event listeners started");

    // Create logger before spawning tasks
    let event_logger = logger.clone();
    let proof_logger = logger.clone();

    // Spawn event processor
    let processor_handle = tokio::spawn(async move {
        let mut processor = EventProcessor::new(event_rx, processed_tx, event_logger);
        if let Err(e) = processor.start().await {
            error!("Event processor failed: {:?}", e);
        }
    });
    handles.push(processor_handle);

    // Spawn proof generator
    let proof_generator_handle = tokio::spawn(async move {
        let mut generator = ProofGenerator::new(
            merged_stream,
            proof_tx,
            MAX_PROOF_RETRIES,
            PROOF_RETRY_DELAY,
            proof_logger,
        );
        if let Err(e) = generator.start().await {
            error!("Proof generator failed: {:?}", e);
        }
    });
    handles.push(proof_generator_handle);

    // Create transaction manager config
    let tx_config = TransactionConfig {
        rpc_urls: vec![
            (
                ETHEREUM_SEPOLIA_CHAIN_ID as u32,
                rpc_url_ethereum_sepolia().to_string(),
            ),
            (
                OPTIMISM_SEPOLIA_CHAIN_ID as u32,
                rpc_url_optimism_sepolia().to_string(),
            ),
            (
                LINEA_SEPOLIA_CHAIN_ID as u32,
                rpc_url_linea_sepolia().to_string(),
            ),
        ],
    };

    // Spawn transaction manager
    let tx_manager_handle = tokio::spawn(async move {
        let mut manager = TransactionManager::new(proof_rx, tx_config, logger.clone());
        if let Err(e) = manager.start().await {
            error!("Transaction manager failed: {:?}", e);
        }
    });
    handles.push(tx_manager_handle);

    info!("All components initialized and running");

    // Set up Unix socket for manual event injection
    let socket_path = "/tmp/sequencer.sock";
    // Remove the socket file if it exists
    let _ = fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;

    let manual_tx_clone = manual_tx.clone();
    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                let tx = manual_tx_clone.clone();
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    if let Ok(_) = socket.read_to_end(&mut buf).await {
                        if let Ok(event) = serde_json::from_slice::<ProcessedEvent>(&buf) {
                            if let Err(e) = tx.send(event).await {
                                error!("Failed to forward manual event: {}", e);
                            }
                        }
                    }
                });
            }
        }
    });

    // Wait for all tasks to complete
    for handle in handles {
        if let Err(e) = handle.await {
            error!("Task failed: {:?}", e);
        }
    }

    warn!("Sequencer shutting down");
    Ok(())
}

async fn create_provider(
    rpc_url: Url,
    private_key: &str,
) -> Result<ProviderType, Box<dyn std::error::Error>> {
    let signer: PrivateKeySigner = private_key.parse().expect("should parse private key");
    let wallet = EthereumWallet::from(signer);

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_http(rpc_url);

    Ok(provider)
}
