use alloy::primitives::{address, Address};
use std::time::Duration;

// Channel capacities
pub const EVENT_CHANNEL_CAPACITY: usize = 1000;
pub const PROCESSED_CHANNEL_CAPACITY: usize = 1000;
pub const PROOF_CHANNEL_CAPACITY: usize = 1000;

// Retry configurations
pub const MAX_PROOF_RETRIES: u32 = 3;
pub const PROOF_RETRY_DELAY: Duration = Duration::from_secs(1);

pub const MAX_TX_RETRIES: u32 = 3;
pub const TX_RETRY_DELAY: Duration = Duration::from_secs(1);
pub const TX_TIMEOUT: Duration = Duration::from_secs(30);

// Gas configurations
pub const GAS_MULTIPLIER: f64 = 1.1;
pub const PRIORITY_FEE_MULTIPLIER: f64 = 1.2;

// Import necessary constants from malda_rs
pub use malda_rs::constants::{
    ETHEREUM_SEPOLIA_CHAIN_ID,

    // Chain IDs
    LINEA_SEPOLIA_CHAIN_ID,
    OPTIMISM_SEPOLIA_CHAIN_ID,
    // Markets (non-chain specific)
    USDC_MARKET_SEPOLIA,
    WETH_MARKET_SEPOLIA,
};

// WebSocket URLs
pub const WS_URL_ETH_SEPOLIA: &str =
    "wss://eth-sepolia.g.alchemy.com/v2/uGenJq8d9bfW9gXcaUZln_ZBDhS61oJY";
pub const WS_URL_OPT_SEPOLIA: &str =
    "wss://opt-sepolia.g.alchemy.com/v2/uGenJq8d9bfW9gXcaUZln_ZBDhS61oJY";
pub const WS_URL_LINEA_SEPOLIA: &str =
    "wss://linea-sepolia.g.alchemy.com/v2/uGenJq8d9bfW9gXcaUZln_ZBDhS61oJY";

// Sequencer configuration
pub fn sequencer_address() -> Address {
    let addr = dotenvy::var("SEQUENCER_ADDRESS")
        .expect("SEQUENCER_ADDRESS must be set in environment");
    Address::parse_checksummed(&addr, None)
        .expect("Invalid sequencer address format")
}

pub fn sequencer_private_key() -> &'static str {
    Box::leak(dotenvy::var("SEQUENCER_PRIVATE_KEY")
        .expect("SEQUENCER_PRIVATE_KEY must be set in environment")
        .into_boxed_str())
}

// Timing configurations
pub const LISTENER_SPAWN_DELAY: Duration = Duration::from_millis(100);
pub const ETHEREUM_BLOCK_DELAY: u64 = 12;

// Add this with other constants
pub const PROOF_REQUEST_DELAY: u64 = 15;

pub const BATCH_SUBMITTER: Address = address!("b4282799022073790c8Ae500Ac6C91C622021079");

/// The time window to wait for additional events to batch together (in seconds)
pub const BATCH_WINDOW: u64 = 2;

