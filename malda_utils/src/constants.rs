//! Constants used throughout the project for chain IDs, addresses, and cryptographic values.
//!
//! This module contains various constant definitions that are used across different chains
//! and components of the Malda Protocol.

use alloy_primitives::{address, Address, U256};

pub const MULTICALL: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");

/// Chain ID for the Ethereum Mainnet network.
pub const ETHEREUM_CHAIN_ID: u64 = 1;
/// Chain ID for the Optimism network.
pub const OPTIMISM_CHAIN_ID: u64 = 10;
/// Chain ID for the Linea network.
pub const LINEA_CHAIN_ID: u64 = 59144;
/// Chain ID for the Scroll network.
pub const SCROLL_CHAIN_ID: u64 = 534352;
/// Chain ID for the Base network.
pub const BASE_CHAIN_ID: u64 = 8453;

/// Chain ID for the Ethereum sepolia network.
pub const ETHEREUM_SEPOLIA_CHAIN_ID: u64 = 11155111;
/// Chain ID for the Optimism sepolia network.
pub const OPTIMISM_SEPOLIA_CHAIN_ID: u64 = 11155420;
/// Chain ID for the Linea sepolia network.
pub const LINEA_SEPOLIA_CHAIN_ID: u64 = 59141;
/// Chain ID for the Scroll sepolia network.
pub const SCROLL_SEPOLIA_CHAIN_ID: u64 = 534351;
/// Chain ID for the Base network.
pub const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

/// The address of the Optimism sequencer contract.
pub const OPTIMISM_SEQUENCER: Address = address!("AAAA45d9549EDA09E70937013520214382Ffc4A2");
/// The address of the Base sequencer contract.
pub const BASE_SEQUENCER: Address = address!("Af6E19BE0F9cE7f8afd49a1824851023A8249e8a");
/// The address of the Linea sequencer contract.
pub const LINEA_SEQUENCER: Address = address!("8f81e2e3f8b46467523463835f965ffe476e1c9e");

/// The address of the Optimism sequencer contract on the sepolia network.
pub const OPTIMISM_SEPOLIA_SEQUENCER: Address =
    address!("57CACBB0d30b01eb2462e5dC940c161aff3230D3");
/// The address of the Base sequencer contract on the sepolia network.
pub const BASE_SEPOLIA_SEQUENCER: Address = address!("b830b99c95Ea32300039624Cb567d324D4b1D83C");
/// The address of the Linea sequencer contract on the sepolia network.
pub const LINEA_SEPOLIA_SEQUENCER: Address = address!("a27342f1b74c0cfb2cda74bac1628d0c1a9752f2");

/// The address of the L1Block contract on Optimism.
/// This contract provides L1 block information to L2.
pub const L1_BLOCK_ADDRESS_OPTIMISM: Address = address!("4200000000000000000000000000000000000015");

/// Half of the secp256k1 curve order (n/2).
///
/// This value is used in signature normalization to ensure s values are in the lower half
/// of the curve order, which is required by some networks (like Ethereum) as a transaction validity rule.
pub const SECP256K1N_HALF: U256 = U256::from_be_bytes([
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0x5D, 0x57, 0x6E, 0x73, 0x57, 0xA4, 0x50, 0x1D, 0xDF, 0xE9, 0x2F, 0x46, 0x68, 0x1B, 0x20, 0xA0,
]);

/// The number of blocks to wait before considering a chain reorganization unlikely.
///
/// This value is used as a safety measure to ensure transaction finality
/// across different blockchain networks.
pub const REORG_PROTECTION_DEPTH_OPTIMISM: u64 = 2;
pub const REORG_PROTECTION_DEPTH_BASE: u64 = 2;
pub const REORG_PROTECTION_DEPTH_LINEA: u64 = 2;
pub const REORG_PROTECTION_DEPTH_ETHEREUM: u64 = 2;
pub const REORG_PROTECTION_DEPTH_SCROLL: u64 = 2;
pub const REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA: u64 = 0;
pub const REORG_PROTECTION_DEPTH_BASE_SEPOLIA: u64 = 0;
pub const REORG_PROTECTION_DEPTH_LINEA_SEPOLIA: u64 = 0;
pub const REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA: u64 = 0;
pub const REORG_PROTECTION_DEPTH_SCROLL_SEPOLIA: u64 = 0;

pub const USDC_MARKET_SEPOLIA: Address = address!("f7E8c0e5b418081112FC045Aaced424961Fe8b99");
pub const WETH_MARKET_SEPOLIA: Address = address!("8Ef9d2057Fed09Fd18cbF393D789C6507CD3E875");

// ONLY FOR TESTING UNTIL NEW PROTOCOL IS DEPLOYED
pub const GETPROOFDATA_MARKET_SEPOLIA: Address =
    address!("dDA5fF7F75D0C28cCD14e654fdB8C3F9CBF0639D");
