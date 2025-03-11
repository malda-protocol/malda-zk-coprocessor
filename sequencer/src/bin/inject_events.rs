use alloy::primitives::{address, b256, Address, TxHash, U256};
use eyre::Result;
use malda_rs::constants::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessedEvent {
    HostWithdraw {
        tx_hash: TxHash,
        sender: Address,
        dst_chain_id: u32,
        amount: U256,
        market: Address,
    },
    HostBorrow {
        tx_hash: TxHash,
        sender: Address,
        dst_chain_id: u32,
        amount: U256,
        market: Address,
    },
    ExtensionSupply {
        tx_hash: TxHash,
        from: Address,
        amount: U256,
        src_chain_id: u32,
        dst_chain_id: u32,
        market: Address,
        method_selector: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a sample event
    let event = ProcessedEvent::ExtensionSupply {
        tx_hash: b256!("055d7d4261d4104a3e7a365fb8cd6ebcdd06cfe14b8e0a4d8bb6c6e429d2e9ef"),
        from: address!("5F4A2babca5FAc4BfE0E80B58c7C048AC0798A2e"), // You'll need to replace this with the actual sender address
        amount: U256::from(4088287),
        src_chain_id: ETHEREUM_SEPOLIA_CHAIN_ID as u32, // Ethereum Sepolia
        dst_chain_id: LINEA_SEPOLIA_CHAIN_ID as u32,    // Linea Sepolia
        market: USDC_MARKET_SEPOLIA, // You'll need to replace this with the actual USDC market address
        method_selector: "08fee263".to_string(),
    };

    // Inject the event
    inject_event(event).await?;

    println!("Event injected successfully");
    Ok(())
}

async fn inject_event(event: ProcessedEvent) -> Result<()> {
    let socket_path = "/tmp/sequencer.sock";
    let mut stream = tokio::net::UnixStream::connect(socket_path).await?;

    // Serialize and send the event
    let json = serde_json::to_string(&event)?;
    stream.write_all(json.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}
