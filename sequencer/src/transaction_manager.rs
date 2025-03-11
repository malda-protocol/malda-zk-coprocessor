use alloy::{
    primitives::{FixedBytes, TxHash, U256},
    providers::Provider,
    transports::http::reqwest::Url,
};
use eyre::Result;
use futures::future::join_all;
use sequencer::logger::{PipelineLogger, PipelineStep};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

type Bytes4 = FixedBytes<4>;

use crate::{
    constants::{BATCH_SUBMITTER, sequencer_address, sequencer_private_key, TX_TIMEOUT},
    create_provider,
    events::{MINT_EXTERNAL_SELECTOR_FB4, OUT_HERE_SELECTOR_FB4, REPAY_EXTERNAL_SELECTOR_FB4},
    proof_generator::ProofReadyEvent,
    types::{BatchProcessMsg, IBatchSubmitter},
    ProviderType,
};

#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub rpc_urls: Vec<(u32, String)>, // (chain_id, url)
}

pub struct TransactionManager {
    event_receiver: mpsc::Receiver<Vec<ProofReadyEvent>>,
    config: TransactionConfig,
    logger: PipelineLogger,
}

impl std::fmt::Debug for TransactionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransactionManager")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl TransactionManager {
    pub fn new(
        event_receiver: mpsc::Receiver<Vec<ProofReadyEvent>>,
        config: TransactionConfig,
        logger: PipelineLogger,
    ) -> Self {
        Self {
            event_receiver,
            config,
            logger,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting transaction manager");

        while let Some(proof_events) = self.event_receiver.recv().await {
            let mut current_chain_id = None;
            let mut chain_start_idx = 0;
            let mut chain_tasks = Vec::new();
            let config = self.config.clone();
            let logger = self.logger.clone();

            // Process all events including the last batch
            for (idx, event) in proof_events.iter().enumerate() {
                if current_chain_id != Some(event.dst_chain_id) {
                    // Process previous chain's batch (if any)
                    if let Some(chain_id) = current_chain_id {
                        let chain_events = proof_events[chain_start_idx..idx].to_vec();
                        let config = config.clone();
                        let logger = logger.clone();

                        chain_tasks.push(tokio::spawn(async move {
                            match Self::process_chain_batch(
                                &chain_events,
                                chain_start_idx,
                                idx,
                                chain_id,
                                &config,
                                &logger
                            ).await {
                                Ok(tx_hash) => {
                                    info!(
                                        "Batch transaction submitted successfully for chain {}: {:?} (indices {}-{})",
                                        chain_id, tx_hash, chain_start_idx, idx
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to process batch for chain {}: {}",
                                        chain_id, e
                                    );
                                    // Log failure for each event in the batch
                                    for event in chain_events {
                                        if let Err(log_err) = logger.log_step(
                                            event.tx_hash,
                                            PipelineStep::TransactionFailed {
                                                tx_hash: event.tx_hash,
                                                error: format!("Batch processing failed: {}", e),
                                                chain_id,
                                            }
                                        ).await {
                                            error!("Failed to log transaction failure: {}", log_err);
                                        }
                                    }
                                }
                            }
                        }));
                    }
                    current_chain_id = Some(event.dst_chain_id);
                    chain_start_idx = idx;
                }
            }

            // Process the final chain's batch
            if let Some(chain_id) = current_chain_id {
                let chain_events = proof_events[chain_start_idx..].to_vec();
                let config = config.clone();
                let logger = logger.clone();

                chain_tasks.push(tokio::spawn(async move {
                    match Self::process_chain_batch(
                        &chain_events,
                        chain_start_idx,
                        proof_events.len(),
                        chain_id,
                        &config,
                        &logger
                    ).await {
                        Ok(tx_hash) => {
                            info!(
                                "Batch transaction submitted successfully for chain {}: {:?} (indices {}-{})",
                                chain_id, tx_hash, chain_start_idx, proof_events.len()
                            );
                        }
                        Err(e) => {
                            error!(
                                "Failed to process batch for chain {}: {}",
                                chain_id, e
                            );
                            // Log failure for each event in the batch
                            for event in chain_events {
                                if let Err(log_err) = logger.log_step(
                                    event.tx_hash,
                                    PipelineStep::TransactionFailed {
                                        tx_hash: event.tx_hash,
                                        error: format!("Batch processing failed: {}", e),
                                        chain_id,
                                    }
                                ).await {
                                    error!("Failed to log transaction failure: {}", log_err);
                                }
                            }
                        }
                    }
                }));
            }

            // Wait for all chain transactions to complete
            if !chain_tasks.is_empty() {
                join_all(chain_tasks).await;
            }
        }

        warn!("Transaction manager channel closed");
        Ok(())
    }

    async fn get_provider_for_chain(
        chain_id: u32,
        config: &TransactionConfig,
    ) -> Result<ProviderType> {
        let rpc_url = config
            .rpc_urls
            .iter()
            .find(|(id, _)| *id == chain_id)
            .map(|(_, url)| url.clone())
            .ok_or_else(|| eyre::eyre!("No RPC URL configured for chain {}", chain_id))?;

        let url = Url::parse(&rpc_url)?;
        create_provider(url, sequencer_private_key())
            .await
            .map_err(|e| eyre::eyre!("Failed to create provider: {}", e))
    }

    async fn process_chain_batch(
        events: &[ProofReadyEvent],
        start_idx: usize,
        _end_idx: usize,
        chain_id: u32,
        config: &TransactionConfig,
        logger: &PipelineLogger,
    ) -> Result<TxHash> {
        let provider = Self::get_provider_for_chain(chain_id, config).await?;

        // Create batch submitter contract instance
        let batch_submitter = IBatchSubmitter::new(BATCH_SUBMITTER, provider.clone());

        // Collect all data for the batch
        let mut receivers = Vec::new();
        let mut markets = Vec::new();
        let mut amounts = Vec::new();
        let mut selectors = Vec::new();
        let mut init_hashes = Vec::new();

        // Use the first event's journal and seal for the entire batch
        let journal_data = events[0].journal.clone();
        let seal = events[0].seal.clone();

        for event in events {
            receivers.push(event.receiver);
            markets.push(event.market);
            amounts.extend(event.amount.clone());
            selectors.push(match event.method.as_str() {
                "outHere" => Bytes4::from_slice(OUT_HERE_SELECTOR_FB4),
                "mintExternal" => Bytes4::from_slice(MINT_EXTERNAL_SELECTOR_FB4),
                "repayExternal" => Bytes4::from_slice(REPAY_EXTERNAL_SELECTOR_FB4),
                method => {
                    error!("Invalid transaction method: {}", method);
                    return Err(eyre::eyre!("Invalid method: {}", method));
                }
            });
            init_hashes.push(event.tx_hash.into());
        }

        let msg = BatchProcessMsg {
            receivers, // Now correctly an array
            journalData: journal_data,
            seal,
            mTokens: markets,
            amounts,
            selectors,
            initHashes: init_hashes, // Added initHashes
            startIndex: U256::from(start_idx as u64),
        };

        info!(
            "Broadcasting batch transaction for chain {} starting at index {}: journal_size={}, seal_size={}, markets={:?}, tx_count={}",
            chain_id,
            start_idx,
            msg.journalData.len(),
            msg.seal.len(),
            msg.mTokens,
            events.len()
        );

        // Submit the batch
        let action = batch_submitter.batchProcess(msg).from(sequencer_address());

        // Estimate gas with a buffer
        let estimated_gas = action.estimate_gas().await?;
        let gas_limit = estimated_gas + (estimated_gas / 2); // Add 50% buffer

        debug!(
            "Estimated gas: {}, using gas limit: {}",
            estimated_gas, gas_limit
        );

        let pending_tx = action.gas(gas_limit).send().await?;
        let tx_hash = pending_tx.tx_hash();

        // Log transaction submission for each event in the batch
        for event in events {
            logger
                .log_step(
                    event.tx_hash,
                    PipelineStep::TransactionSubmitted {
                        tx_hash: *tx_hash,
                        method: "batchProcess".to_string(),
                        gas_used: U256::from(0u64),
                        gas_price: U256::from(provider.get_gas_price().await?),
                    },
                )
                .await?;
        }

        info!("Batch transaction sent with hash {}", tx_hash);

        match pending_tx.with_timeout(Some(TX_TIMEOUT)).watch().await {
            Ok(hash) => {
                info!("Batch transaction confirmed with hash {:?}", hash);

                let receipt = provider
                    .get_transaction_receipt(hash)
                    .await?
                    .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;

                // Log completion for each event in the batch
                for event in events {
                    logger
                        .log_step(
                            event.tx_hash,
                            PipelineStep::TransactionSubmitted {
                                tx_hash: hash,
                                method: "batchProcess".to_string(),
                                gas_used: U256::from(receipt.gas_used),
                                gas_price: U256::from(receipt.effective_gas_price),
                            },
                        )
                        .await?;

                    logger
                        .log_step(
                            event.tx_hash,
                            PipelineStep::TransactionVerified {
                                tx_hash: hash,
                                block_number: receipt.block_number.unwrap_or_default(),
                                method: "batchProcess".to_string(),
                                status: if receipt.status() { 1 } else { 0 },
                            },
                        )
                        .await?;
                }

                Ok(hash)
            }
            Err(e) => {
                error!("Batch transaction failed: {}", e);
                Err(e.into())
            }
        }
    }
}
