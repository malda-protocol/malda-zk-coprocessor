// Copyright (c) 2025 Merge Layers Inc.
//
// This source code is licensed under the Business Source License 1.1
// (the "License"); you may not use this file except in compliance with the
// License. You may obtain a copy of the License at
//
//     https://github.com/malda-protocol/malda-zk-coprocessor/blob/main/LICENSE-BSL
//
// See the License for the specific language governing permissions and
// limitations under the License.
//
// This file contains code derived from or inspired by Risc0,
// originally licensed under the Apache License 2.0. See LICENSE-RISC0
// and the NOTICE file for original license terms and attributions.
//! Constants used throughout the project for chain IDs, addresses, and cryptographic values.
//!
//! This module contains various constant definitions that are used across different chains
//! and components of the Malda Protocol.

use alloy_primitives::{address, Address, B256};

pub const MULTICALL: Address = address!("cA11bde05977b3631167028862bE2a173976CA11");
/// Selector for getProofData(address,uint32)
pub const SELECTOR_MALDA_GET_PROOF_DATA: [u8; 4] = [0x07, 0xd9, 0x23, 0xe9];

/// Chain ID for the Ethereum Mainnet network.
pub const ETHEREUM_CHAIN_ID: u64 = 1;
/// Chain ID for the Optimism network.
pub const OPTIMISM_CHAIN_ID: u64 = 10;
/// Chain ID for the Linea network.
pub const LINEA_CHAIN_ID: u64 = 59144;
/// Chain ID for the Base network.
pub const BASE_CHAIN_ID: u64 = 8453;

/// Chain ID for the Ethereum sepolia network.
pub const ETHEREUM_SEPOLIA_CHAIN_ID: u64 = 11155111;
/// Chain ID for the Optimism sepolia network.
pub const OPTIMISM_SEPOLIA_CHAIN_ID: u64 = 11155420;
/// Chain ID for the Linea sepolia network.
pub const LINEA_SEPOLIA_CHAIN_ID: u64 = 59141;
/// Chain ID for the Base network.
pub const BASE_SEPOLIA_CHAIN_ID: u64 = 84532;

/// The address of the Optimism sequencer contract.
pub const OPTIMISM_SEQUENCER: Address = address!("AAAA45d9549EDA09E70937013520214382Ffc4A2");
/// The address of the Base sequencer contract.
pub const BASE_SEQUENCER: Address = address!("Af6E19BE0F9cE7f8afd49a1824851023A8249e8a");

/// The address of the Optimism sequencer contract on the sepolia network.
pub const OPTIMISM_SEPOLIA_SEQUENCER: Address =
  address!("57CACBB0d30b01eb2462e5dC940c161aff3230D3");
/// The address of the Base sequencer contract on the sepolia network.
pub const BASE_SEPOLIA_SEQUENCER: Address = address!("b830b99c95Ea32300039624Cb567d324D4b1D83C");

/// The address of the L1Block contract on Optimism.
/// This contract provides L1 block information to L2.
pub const L1_BLOCK_ADDRESS_OPSTACK: Address = address!("4200000000000000000000000000000000000015");
/// The address of the MessagePasser contract on Optimism.
pub const MESSAGE_PASSER_ADDRESS_OPSTACK: Address =
  address!("4200000000000000000000000000000000000016");
pub const ROOT_VERSION_OPSTACK: B256 = B256::ZERO;
pub const TIME_DELAY_OP_CHALLENGE: u64 = 300;

pub const DISPUTE_GAME_FACTORY_OPTIMISM: Address =
  address!("e5965Ab5962eDc7477C8520243A95517CD252fA9");
pub const DISPUTE_GAME_FACTORY_OPTIMISM_SEPOLIA: Address =
  address!("05F9613aDB30026FFd634f38e5C4dFd30a197Fa1");
pub const DISPUTE_GAME_FACTORY_BASE: Address = address!("43edB88C4B80fDD2AdFF2412A7BebF9dF42cB40e");
pub const DISPUTE_GAME_FACTORY_BASE_SEPOLIA: Address =
  address!("d6E6dBf4F7EA0ac412fD8b65ED297e64BB7a06E1");

pub const L1_MESSAGE_SERVICE_LINEA: Address = address!("d19d4B5d358258f05D7B411E21A1460D11B0876F");
pub const L1_MESSAGE_SERVICE_LINEA_SEPOLIA: Address =
  address!("B218f8A4Bc926cF1cA7b3423c154a0D627Bdb7E5");

/// The number of blocks to wait before considering a chain reorganization unlikely.
///
/// This value is used as a safety measure to ensure transaction finality
/// across different blockchain networks.
pub const REORG_PROTECTION_DEPTH_OPTIMISM: u64 = 2;
pub const REORG_PROTECTION_DEPTH_BASE: u64 = 2;
pub const REORG_PROTECTION_DEPTH_LINEA: u64 = 2;
pub const REORG_PROTECTION_DEPTH_ETHEREUM: u64 = 2;
pub const REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA: u64 = 2;
pub const REORG_PROTECTION_DEPTH_BASE_SEPOLIA: u64 = 2;
pub const REORG_PROTECTION_DEPTH_LINEA_SEPOLIA: u64 = 2;
pub const REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA: u64 = 2;

pub const OPTIMISM_PORTAL: Address = address!("bEb5Fc579115071764c7423A4f12eDde41f106Ed");
pub const OPTIMISM_SEPOLIA_PORTAL: Address = address!("16Fc5058F25648194471939df75CF27A2fdC48BC");
pub const BASE_PORTAL: Address = address!("49048044D57e1C92A77f79988d21Fa8fAF74E97e");
pub const BASE_SEPOLIA_PORTAL: Address = address!("49f53e41452C74589E85cA1677426Ba426459e85");
