use alloy::primitives::{Address, TxHash, U256};
use chrono::{DateTime, Utc};
use eyre::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::{
    fs::OpenOptions,
    io::AsyncWriteExt,
    sync::mpsc::{self, Sender},
};
use tracing::error;

#[derive(Debug, Clone)]
pub enum PipelineStep {
    EventReceived {
        chain_id: u32,
        block_number: u64,
        market: Address,
        event_type: String,
    },
    EventProcessed {
        chain_id: u32,
        dst_chain_id: u32,
        market: Address,
        event_type: String,
        amount: U256,
    },
    ProofGenerated {
        duration_ms: u64,
        journal: String, // hex string
        seal: String,    // hex string
    },
    TransactionSubmitted {
        tx_hash: TxHash, // The new transaction hash
        method: String,
        gas_used: U256,
        gas_price: U256,
    },
    TransactionVerified {
        tx_hash: TxHash,
        method: String,
        block_number: u64,
        status: u64,
    },
    TransactionFailed {
        tx_hash: TxHash,
        error: String,
        chain_id: u32,
    },
    BatchProcessed {
        chain_id: u32,
        status: String,
        tx_hash: TxHash,
    },
}

#[derive(Debug)]
#[allow(dead_code)]
struct LogEntry {
    timestamp: DateTime<Utc>,
    chain_id: u32,
    block_number: Option<u64>,
    market: Address,
    event_type: String,
    dst_chain_id: Option<u32>,
    amount: Option<U256>,
    proof_duration_ms: Option<u64>,
    journal: Option<String>,
    seal: Option<String>,
}

#[derive(Debug)]
struct LogEvent {
    tx_hash: TxHash,
    timestamp: DateTime<Utc>,
    step: PipelineStep,
}

#[derive(Clone)]
pub struct PipelineLogger {
    event_sender: Sender<LogEvent>,
    log_path: PathBuf,
}

impl PipelineLogger {
    pub async fn new(file_path: PathBuf) -> Result<Self> {
        let (event_sender, event_receiver) = mpsc::channel(100);

        // Clone file_path before moving into spawned task
        let writer_path = file_path.clone();

        // Spawn background task for file writing
        tokio::spawn(async move {
            if let Err(e) = Self::log_writer(event_receiver, writer_path).await {
                error!("Logger task failed: {}", e);
            }
        });

        Ok(Self {
            event_sender,
            log_path: file_path,
        })
    }

    pub async fn log_step(&self, tx_hash: TxHash, step: PipelineStep) -> Result<()> {
        let event = LogEvent {
            tx_hash,
            timestamp: Utc::now(),
            step,
        };

        self.event_sender
            .send(event)
            .await
            .map_err(|e| eyre::eyre!("Failed to send log event: {}", e))?;

        Ok(())
    }

    async fn log_writer(mut receiver: mpsc::Receiver<LogEvent>, file_path: PathBuf) -> Result<()> {
        let mut pending_logs: HashMap<TxHash, (u64, LogEntry)> = HashMap::new();

        while let Some(event) = receiver.recv().await {
            match &event.step {
                PipelineStep::EventReceived {
                    chain_id,
                    block_number,
                    market,
                    event_type,
                } => {
                    let log_entry = LogEntry {
                        timestamp: event.timestamp,
                        chain_id: *chain_id,
                        block_number: Some(*block_number),
                        market: *market,
                        event_type: event_type.clone(),
                        dst_chain_id: None,
                        amount: None,
                        proof_duration_ms: None,
                        journal: None,
                        seal: None,
                    };

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&file_path)
                        .await?;

                    let position = file.metadata().await?.len();

                    let log_line = format!(
                        "{}, TxHash: {}, {}, Block: {}, {}, {}, Amount: Pending\n",
                        log_entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        hex::encode(event.tx_hash.0),
                        get_chain_name(log_entry.chain_id),
                        block_number,
                        get_market_name(log_entry.market),
                        get_event_name(&log_entry.event_type),
                    );

                    file.write_all(log_line.as_bytes()).await?;
                    pending_logs.insert(event.tx_hash, (position, log_entry));
                }
                PipelineStep::EventProcessed {
                    chain_id: _,
                    dst_chain_id,
                    market: _,
                    event_type: _,
                    amount,
                } => {
                    if let Some((_, ref mut entry)) = pending_logs.get_mut(&event.tx_hash) {
                        entry.dst_chain_id = Some(*dst_chain_id);
                        entry.amount = Some(*amount);

                        let log_line = format!(
                            "{}, TxHash: {}, {} -> {}, Block: {}, {}, {}, Amount: {}\n",
                            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                            hex::encode(event.tx_hash.0),
                            get_chain_name(entry.chain_id),
                            get_chain_name(*dst_chain_id),
                            entry.block_number.unwrap_or(0),
                            get_market_name(entry.market),
                            get_event_name(&entry.event_type),
                            amount,
                        );

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&file_path)
                            .await?;

                        file.write_all(log_line.as_bytes()).await?;
                    }
                }
                PipelineStep::ProofGenerated {
                    duration_ms,
                    journal,
                    seal,
                } => {
                    if let Some((_, ref mut entry)) = pending_logs.get_mut(&event.tx_hash) {
                        entry.proof_duration_ms = Some(*duration_ms);
                        entry.journal = Some(journal.clone());
                        entry.seal = Some(seal.clone());

                        let log_line = format!(
                            "{}, TxHash: {}, {} -> {}, Block: {}, {}, {}, Amount: {}, Proof: {:.2}s, Journal: 0x{}, Seal: 0x{}\n",
                            Utc::now().format("%Y-%m-%d %H:%M:%S"),
                            hex::encode(event.tx_hash.0),
                            get_chain_name(entry.chain_id),
                            get_chain_name(entry.dst_chain_id.unwrap_or(0)),
                            entry.block_number.unwrap_or(0),
                            get_market_name(entry.market),
                            get_event_name(&entry.event_type),
                            entry.amount.unwrap_or_default(),
                            *duration_ms as f64 / 1000.0,
                            journal,
                            seal,
                        );

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&file_path)
                            .await?;

                        file.write_all(log_line.as_bytes()).await?;

                        // Remove the entry after successful write
                        pending_logs.remove(&event.tx_hash);
                    }
                }
                PipelineStep::TransactionSubmitted {
                    tx_hash: new_tx_hash,
                    method,
                    gas_used,
                    gas_price,
                } => {
                    if let Some((_, ref entry)) = pending_logs.get(&event.tx_hash) {
                        let log_line = format!(
                            "{}, TxHash: {}, {} -> {}, Block: {}, {}, {}, Amount: {}, Transaction: method={}, tx={}, gas={}, price={} gwei\n",
                            Utc::now().format("%Y-%m-%d %H:%M:%S"),
                            hex::encode(event.tx_hash.0),
                            get_chain_name(entry.chain_id),
                            get_chain_name(entry.dst_chain_id.unwrap_or(0)),
                            entry.block_number.unwrap_or(0),
                            get_market_name(entry.market),
                            get_event_name(&entry.event_type),
                            entry.amount.unwrap_or_default(),
                            method,
                            hex::encode(new_tx_hash.0),
                            gas_used,
                            gas_price / U256::from(1_000_000_000), // Convert to gwei
                        );

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&file_path)
                            .await?;

                        file.write_all(log_line.as_bytes()).await?;

                        // Remove the entry after successful write
                        pending_logs.remove(&event.tx_hash);
                    }
                }
                PipelineStep::TransactionVerified {
                    tx_hash: new_tx_hash,
                    method,
                    block_number,
                    status,
                } => {
                    if let Some((_, ref entry)) = pending_logs.get(&event.tx_hash) {
                        let status_str = if *status == 1 { "Success" } else { "Failed" };
                        let log_line = format!(
                            "{}, TxHash: {}, {} -> {}, Block: {}, {}, {}, Amount: {}, Transaction: Verified, method={}, tx={}, block={}, status={}\n",
                            Utc::now().format("%Y-%m-%d %H:%M:%S"),
                            hex::encode(event.tx_hash.0),
                            get_chain_name(entry.chain_id),
                            get_chain_name(entry.dst_chain_id.unwrap_or(0)),
                            entry.block_number.unwrap_or(0),
                            get_market_name(entry.market),
                            get_event_name(&entry.event_type),
                            entry.amount.unwrap_or_default(),
                            method,
                            hex::encode(new_tx_hash.0),
                            block_number,
                            status_str,
                        );

                        let mut file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&file_path)
                            .await?;

                        file.write_all(log_line.as_bytes()).await?;

                        // Remove the entry after successful write
                        pending_logs.remove(&event.tx_hash);
                    }
                }
                PipelineStep::TransactionFailed {
                    tx_hash,
                    error,
                    chain_id,
                } => {
                    let log_line = format!(
                        "{}, TxHash: {}, {}, Error: {}\n",
                        Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        hex::encode(tx_hash.0),
                        get_chain_name(*chain_id),
                        error,
                    );

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&file_path)
                        .await?;

                    file.write_all(log_line.as_bytes()).await?;
                }
                PipelineStep::BatchProcessed {
                    chain_id,
                    status,
                    tx_hash,
                } => {
                    let log_line = format!(
                        "{}, TxHash: {}, {}, Status: {}, BatchHash: {}\n",
                        Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        hex::encode(event.tx_hash.0), // init_hash we passed in
                        get_chain_name(*chain_id),
                        status,
                        hex::encode(tx_hash.0), // batch transaction hash
                    );

                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&file_path)
                        .await?;

                    file.write_all(log_line.as_bytes()).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn write_to_log(&self, message: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        file.write_all(message.as_bytes()).await?;
        Ok(())
    }
}

fn get_chain_name(chain_id: u32) -> &'static str {
    match chain_id {
        59141 => "Linea Sepolia",
        11155420 => "Optimism Sepolia",
        11155111 => "Ethereum Sepolia",
        _ => "Unknown Chain",
    }
}

fn get_market_name(market: Address) -> &'static str {
    match market {
        addr if addr == crate::constants::WETH_MARKET_SEPOLIA => "WETH",
        addr if addr == crate::constants::USDC_MARKET_SEPOLIA => "USDC",
        _ => "Unknown Market",
    }
}

fn get_event_name(event_type: &str) -> String {
    // Extract just the event name without parameters
    if let Some(end) = event_type.find('(') {
        event_type[..end].to_string()
    } else {
        event_type.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tokio::fs;

    #[tokio::test]
    async fn test_basic_logging() -> Result<()> {
        let test_file = PathBuf::from("test_pipeline.log");

        // Clean up any existing test file
        let _ = fs::remove_file(&test_file).await;

        let logger = PipelineLogger::new(test_file.clone()).await?;

        // Create a test transaction hash
        let tx_hash =
            TxHash::from_str("0x1234567890123456789012345678901234567890123456789012345678901234")?;

        // Log a test event
        logger
            .log_step(
                tx_hash,
                PipelineStep::EventReceived {
                    chain_id: 1,
                    block_number: 100,
                    market: Address::from_str("0x1234567890123456789012345678901234567890")?,
                    event_type: String::from("TestEvent"),
                },
            )
            .await?;

        // Give some time for the background task to write
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify the log file exists and contains our event
        let contents = fs::read_to_string(&test_file).await?;
        assert!(!contents.is_empty());
        assert!(contents.contains("Amount: Pending"));

        // Test processing step
        logger
            .log_step(
                tx_hash,
                PipelineStep::EventProcessed {
                    chain_id: 1,
                    dst_chain_id: 2,
                    market: Address::from_str("0x1234567890123456789012345678901234567890")?,
                    event_type: String::from("TestEvent"),
                    amount: U256::from(100),
                },
            )
            .await?;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify updated log
        let contents = fs::read_to_string(&test_file).await?;
        assert!(contents.contains("Amount: 100"));

        // Clean up
        fs::remove_file(&test_file).await?;

        Ok(())
    }
}
