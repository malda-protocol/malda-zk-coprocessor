use alloy::primitives::{Address, Bytes, TxHash, U256};
use eyre::Result;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, Instant};
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::event_processor::ProcessedEvent;
use malda_rs::viewcalls::get_proof_data_prove_sdk;
use sequencer::logger::{PipelineLogger, PipelineStep};

#[derive(Debug, Clone)]
pub struct ProofReadyEvent {
    pub tx_hash: TxHash,
    pub market: Address,
    pub journal: Bytes,
    pub seal: Bytes,
    pub amount: Vec<U256>,
    pub receiver: Address,
    pub method: String,
    pub dst_chain_id: u32,
}

pub struct ProofGenerator {
    event_receiver: Box<dyn Stream<Item = ProcessedEvent> + Unpin + Send>,
    proof_sender: mpsc::Sender<Vec<ProofReadyEvent>>,
    max_retries: u32,
    retry_delay: Duration,
    logger: PipelineLogger,
}

impl ProofGenerator {
    pub fn new(
        event_receiver: impl Stream<Item = ProcessedEvent> + Unpin + Send + 'static,
        proof_sender: mpsc::Sender<Vec<ProofReadyEvent>>,
        max_retries: u32,
        retry_delay: Duration,
        logger: PipelineLogger,
    ) -> Self {
        Self {
            event_receiver: Box::new(event_receiver),
            proof_sender,
            max_retries,
            retry_delay,
            logger,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting proof generator, waiting for events...");

        let mut batch = Vec::new();
        let batch_timeout = Duration::from_secs(crate::constants::BATCH_WINDOW);
        let mut last_proof_time = Instant::now();

        loop {
            // Wait for the first event
            if let Some(event) = self.event_receiver.next().await {
                info!(
                    "Received event for processing: type={}",
                    match &event {
                        ProcessedEvent::HostWithdraw { .. } => "HostWithdraw",
                        ProcessedEvent::HostBorrow { .. } => "HostBorrow",
                        ProcessedEvent::ExtensionSupply { .. } => "ExtensionSupply",
                    }
                );
                batch.push(event);

                // Set deadline for batch collection
                let deadline = Instant::now() + batch_timeout;

                // Collect any additional events until deadline
                while Instant::now() < deadline {
                    tokio::select! {
                        Some(event) = self.event_receiver.next() => {
                            info!("Additional event received during batch window");
                            batch.push(event);
                        }
                        _ = sleep(Duration::from_millis(100)) => {}
                    }
                }

                // Check if we need to wait for the proof delay
                let time_since_last_proof = last_proof_time.elapsed();
                let proof_delay = Duration::from_secs(crate::constants::PROOF_REQUEST_DELAY);

                if time_since_last_proof < proof_delay {
                    let wait_time = proof_delay - time_since_last_proof;
                    debug!("Waiting {:?} to respect proof delay", wait_time);
                    sleep(wait_time).await;
                }

                // Process whatever we've collected
                let events_to_process = std::mem::take(&mut batch);
                let proof_sender = self.proof_sender.clone();
                let max_retries = self.max_retries;
                let retry_delay = self.retry_delay;
                let logger = self.logger.clone();

                tokio::spawn(async move {
                    let proof_generator = ProofGeneratorWorker {
                        max_retries,
                        retry_delay,
                    };

                    match proof_generator
                        .process_batch(events_to_process, &logger)
                        .await
                    {
                        Ok(proof_events) => {
                            info!(
                                "Successfully generated proofs for {} events",
                                proof_events.len()
                            );

                            if let Err(e) = proof_sender.send(proof_events).await {
                                error!("Failed to send proof ready events: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to generate proofs for batch: {}", e);
                        }
                    }
                });
                last_proof_time = Instant::now();
            }
        }
    }
}

struct ProofGeneratorWorker {
    max_retries: u32,
    retry_delay: Duration,
}

impl ProofGeneratorWorker {
    async fn process_batch(
        &self,
        events: Vec<ProcessedEvent>,
        logger: &PipelineLogger,
    ) -> Result<Vec<ProofReadyEvent>> {
        // Sort events by src_chain (Linea first) and dst_chain
        let mut sorted_events = events;
        sorted_events.sort_by(|a, b| {
            let (a_src, a_dst) = match a {
                ProcessedEvent::HostWithdraw { dst_chain_id, .. }
                | ProcessedEvent::HostBorrow { dst_chain_id, .. } => {
                    (malda_rs::constants::LINEA_SEPOLIA_CHAIN_ID, *dst_chain_id)
                }
                ProcessedEvent::ExtensionSupply {
                    src_chain_id,
                    dst_chain_id,
                    ..
                } => (*src_chain_id as u64, *dst_chain_id),
            };

            let (b_src, b_dst) = match b {
                ProcessedEvent::HostWithdraw { dst_chain_id, .. }
                | ProcessedEvent::HostBorrow { dst_chain_id, .. } => {
                    (malda_rs::constants::LINEA_SEPOLIA_CHAIN_ID, *dst_chain_id)
                }
                ProcessedEvent::ExtensionSupply {
                    src_chain_id,
                    dst_chain_id,
                    ..
                } => (*src_chain_id as u64, *dst_chain_id),
            };

            // Sort by src_chain first (Linea first), then by dst_chain
            match a_src.cmp(&b_src) {
                std::cmp::Ordering::Equal => a_dst.cmp(&b_dst),
                other => other,
            }
        });

        // Initialize vectors for proof generation
        let mut users: Vec<Vec<Address>> = Vec::new();
        let mut markets: Vec<Vec<Address>> = Vec::new();
        let mut dst_chain_ids: Vec<Vec<u64>> = Vec::new();
        let mut src_chain_ids: Vec<u64> = Vec::new();
        let mut event_details = Vec::new();

        // Group events by source chain
        let mut current_src_chain: Option<u64> = None;
        let mut current_users: Vec<Address> = Vec::new();
        let mut current_markets: Vec<Address> = Vec::new();
        let mut current_dst_chains: Vec<u64> = Vec::new();

        // Process sorted events
        for event in sorted_events {
            let (src_chain, user, market, dst_chain, tx_hash, amount, method) = match event {
                ProcessedEvent::HostWithdraw {
                    tx_hash,
                    sender,
                    dst_chain_id,
                    amount,
                    market,
                }
                | ProcessedEvent::HostBorrow {
                    tx_hash,
                    sender,
                    dst_chain_id,
                    amount,
                    market,
                } => (
                    malda_rs::constants::LINEA_SEPOLIA_CHAIN_ID,
                    sender,
                    market,
                    dst_chain_id as u64,
                    tx_hash,
                    amount,
                    "outHere".to_string(),
                ),
                ProcessedEvent::ExtensionSupply {
                    tx_hash,
                    from,
                    amount,
                    src_chain_id,
                    dst_chain_id,
                    market,
                    method_selector,
                } => {
                    let method = if method_selector == crate::events::MINT_EXTERNAL_SELECTOR {
                        "mintExternal"
                    } else if method_selector == crate::events::REPAY_EXTERNAL_SELECTOR {
                        "repayExternal"
                    } else {
                        return Err(eyre::eyre!("Invalid method selector: {}", method_selector));
                    };

                    (
                        src_chain_id as u64,
                        from,
                        market,
                        dst_chain_id as u64,
                        tx_hash,
                        amount,
                        method.to_string(),
                    )
                }
            };

            // If we encounter a new source chain, push the current batch and start a new one
            if current_src_chain != Some(src_chain) {
                if !current_users.is_empty() {
                    users.push(current_users);
                    markets.push(current_markets);
                    dst_chain_ids.push(current_dst_chains);
                    src_chain_ids.push(current_src_chain.unwrap());
                }
                current_users = Vec::new();
                current_markets = Vec::new();
                current_dst_chains = Vec::new();
                current_src_chain = Some(src_chain);
            }

            // Add to current batch
            current_users.push(user);
            current_markets.push(market);
            current_dst_chains.push(dst_chain);
            event_details.push((tx_hash, amount, market, dst_chain as u32, method));
        }

        // Push the last batch
        if !current_users.is_empty() {
            users.push(current_users);
            markets.push(current_markets);
            dst_chain_ids.push(current_dst_chains);
            src_chain_ids.push(current_src_chain.unwrap());
        }

        let start_time = Instant::now();
        debug!(
            "Starting batch proof generation for {} source chains at {:?}",
            src_chain_ids.len(),
            start_time
        );

        // Generate single proof for all events
        let (journal, seal) = self
            .generate_proof_with_retry(
                users.clone(),
                markets.clone(),
                dst_chain_ids.clone(),
                src_chain_ids,
            )
            .await?;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        debug!("Batch proof generation completed in {}ms", duration_ms);

        // Log the proof generation for each event
        for (tx_hash, _, _, _, _) in &event_details {
            logger
                .log_step(
                    *tx_hash,
                    PipelineStep::ProofGenerated {
                        duration_ms,
                        journal: hex::encode(&journal),
                        seal: hex::encode(&seal),
                    },
                )
                .await?;
        }

        // Create proof events for each original event
        Ok(event_details
            .into_iter()
            .map(
                |(tx_hash, amount, market, dst_chain_id, method)| ProofReadyEvent {
                    tx_hash,
                    market,
                    journal: journal.clone(),
                    seal: seal.clone(),
                    amount: vec![amount],
                    receiver: Address::ZERO,
                    method,
                    dst_chain_id,
                },
            )
            .collect())
    }

    async fn generate_proof_with_retry(
        &self,
        users: Vec<Vec<Address>>,
        markets: Vec<Vec<Address>>,
        dst_chain_ids: Vec<Vec<u64>>,
        src_chain_ids: Vec<u64>,
    ) -> Result<(Bytes, Bytes)> {
        let mut attempts = 0;
        debug!(
            "Starting proof generation attempt for markets={:?}, src_chains={:?}, dst_chains={:?}",
            markets, src_chain_ids, dst_chain_ids
        );

        loop {
            match get_proof_data_prove_sdk(
                users.clone(),
                markets.clone(),
                dst_chain_ids.clone(),
                src_chain_ids.clone(),
                false,
            )
            .await
            {
                Ok(proof_info) => {
                    info!("Successfully generated proof data");
                    let receipt = proof_info.receipt;
                    let seal = match risc0_ethereum_contracts::encode_seal(&receipt) {
                        Ok(seal_data) => {
                            debug!("Successfully encoded seal");
                            Bytes::from(seal_data)
                        }
                        Err(e) => {
                            error!("Failed to encode seal: {}", e);
                            return Err(eyre::eyre!("Failed to encode seal: {}", e));
                        }
                    };
                    let journal = Bytes::from(receipt.journal.bytes);

                    info!(
                        "Generated proof - journal size: {}, seal size: {}",
                        journal.len(),
                        seal.len()
                    );
                    debug!(
                        "Proof details - journal: 0x{}, seal: 0x{}",
                        hex::encode(&journal),
                        hex::encode(&seal)
                    );

                    return Ok((journal, seal));
                }
                Err(e) if attempts < self.max_retries => {
                    attempts += 1;
                    warn!(
                        "Proof generation attempt {} failed: {}. Retrying...",
                        attempts, e
                    );
                    tokio::time::sleep(self.retry_delay).await;
                }
                Err(e) => {
                    error!(
                        "Failed to generate proof after {} attempts: {}",
                        attempts, e
                    );
                    return Err(eyre::eyre!(
                        "Failed to generate proof after {} attempts: {}",
                        attempts,
                        e
                    ));
                }
            }
        }
    }
}
