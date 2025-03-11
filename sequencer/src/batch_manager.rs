use alloy::primitives::TxHash;
use eyre::Result;
use tokio::sync::mpsc;
use tracing::{info, debug};

use crate::event_processor::ProcessedEvent;
use crate::proof_generator::ProofReadyEvent;
use sequencer::logger::{PipelineLogger, PipelineStep};
use alloy::primitives::Address;

// Will be expanded in future implementations
#[derive(Debug, Clone)]
pub enum BatchingStrategy {
    // Current behavior: forward immediately
    Immediate,
}

pub struct BatchManager {
    event_receiver: mpsc::Receiver<ProcessedEvent>,
    proof_sender: mpsc::Sender<ProofReadyEvent>,
    strategy: BatchingStrategy,
    logger: PipelineLogger,
}

impl BatchManager {
    pub fn new(
        event_receiver: mpsc::Receiver<ProcessedEvent>,
        proof_sender: mpsc::Sender<ProofReadyEvent>,
        strategy: BatchingStrategy,
        logger: PipelineLogger,
    ) -> Self {
        Self {
            event_receiver,
            proof_sender,
            strategy,
            logger,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting batch manager with {:?} strategy", self.strategy);

        while let Some(event) = self.event_receiver.recv().await {
            debug!("Batch manager received event");
            
            // For now, just log and forward
            match &event {
                ProcessedEvent::HostWithdraw { tx_hash, .. } |
                ProcessedEvent::HostBorrow { tx_hash, .. } |
                ProcessedEvent::ExtensionSupply { tx_hash, .. } => {
                    self.log_event(*tx_hash).await?;
                }
            }

            // Forward to proof generator (current behavior)
            if let Err(e) = self.proof_sender.send(event.into()).await {
                tracing::error!("Failed to forward event to proof generator: {}", e);
            }
        }

        Ok(())
    }

    async fn log_event(&self, tx_hash: TxHash) -> Result<()> {
        self.logger.log_step(
            tx_hash,
            PipelineStep::EventReceived {
                chain_id: 0, // Will be properly implemented later
                block_number: 0,
                market: Default::default(),
                event_type: "BatchManagerReceived".to_string(),
            },
        ).await?;
        Ok(())
    }
}

impl From<ProcessedEvent> for ProofReadyEvent {
    fn from(event: ProcessedEvent) -> Self {
        match event {
            ProcessedEvent::HostWithdraw { tx_hash, sender: _, dst_chain_id, amount, market } => {
                ProofReadyEvent {
                    tx_hash,
                    market,
                    journal: Default::default(), // Will be set by proof generator
                    seal: Default::default(),    // Will be set by proof generator
                    amount: vec![amount],
                    receiver: Address::ZERO,
                    method: "outHere".to_string(),
                    dst_chain_id,
                }
            },
            ProcessedEvent::HostBorrow { tx_hash, sender: _, dst_chain_id, amount, market } => {
                ProofReadyEvent {
                    tx_hash,
                    market,
                    journal: Default::default(),
                    seal: Default::default(),
                    amount: vec![amount],
                    receiver: Address::ZERO,
                    method: "outHere".to_string(),
                    dst_chain_id,
                }
            },
            ProcessedEvent::ExtensionSupply { 
                tx_hash, 
                from: _, 
                amount, 
                src_chain_id: _, 
                dst_chain_id, 
                market, 
                method_selector 
            } => {
                let method = if method_selector == crate::events::MINT_EXTERNAL_SELECTOR {
                    "mintExternal"
                } else if method_selector == crate::events::REPAY_EXTERNAL_SELECTOR {
                    "repayExternal"
                } else {
                    "outHere" // Default case
                };

                ProofReadyEvent {
                    tx_hash,
                    market,
                    journal: Default::default(),
                    seal: Default::default(),
                    amount: vec![amount],
                    receiver: Address::ZERO,
                    method: method.to_string(),
                    dst_chain_id,
                }
            }
        }
    }
}

impl std::fmt::Debug for BatchManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchManager")
            .field("strategy", &self.strategy)
            // Skip fields that don't implement Debug
            .finish_non_exhaustive()
    }
} 