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

//! Chain ID helper functions for multi-chain support.
//!
//! This module provides utility functions for working with different blockchain networks,
//! including reorg protection depth lookups and chain type identification.

use alloy_primitives::Address;
use risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC;

use crate::constants::*;
use crate::types::{EthChainSpec, LINEA_MAINNET_CHAIN_SPEC, LINEA_SEPOLIA_CHAIN_SPEC};

/// Returns the reorg protection depth for a given chain ID.
///
/// This value represents the number of blocks to wait before considering
/// a transaction final and safe from chain reorganizations.
///
/// # Arguments
/// * `chain_id` - The chain ID to get the protection depth for.
///
/// # Returns
/// * `u64` - The reorg protection depth in blocks.
///
/// # Panics
/// Panics if an invalid chain ID is provided.
pub fn get_reorg_protection_depth(chain_id: u64) -> u64 {
    match chain_id {
        OPTIMISM_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM,
        BASE_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE,
        LINEA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA,
        ETHEREUM_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM,
        OPTIMISM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
        BASE_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
        LINEA_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
        ETHEREUM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
        _ => panic!("invalid chain id: {}", chain_id),
    }
}

/// Returns the OptimismPortal contract address for a given OpStack chain ID.
///
/// # Arguments
/// * `chain_id` - The OpStack chain ID (Optimism or Base, mainnet or Sepolia).
///
/// # Returns
/// * `Address` - The portal contract address.
///
/// # Panics
/// Panics if an invalid or non-OpStack chain ID is provided.
pub fn get_portal_address(chain_id: u64) -> Address {
    match chain_id {
        OPTIMISM_CHAIN_ID => OPTIMISM_PORTAL,
        OPTIMISM_SEPOLIA_CHAIN_ID => OPTIMISM_SEPOLIA_PORTAL,
        BASE_CHAIN_ID => BASE_PORTAL,
        BASE_SEPOLIA_CHAIN_ID => BASE_SEPOLIA_PORTAL,
        _ => panic!("invalid chain id for portal: {}", chain_id),
    }
}

/// Checks if a chain ID corresponds to an OpStack L2 chain.
///
/// # Arguments
/// * `chain_id` - The chain ID to check.
///
/// # Returns
/// * `bool` - True if the chain is an OpStack chain (Optimism or Base, mainnet or Sepolia).
pub fn is_opstack_chain(chain_id: u64) -> bool {
    matches!(
        chain_id,
        OPTIMISM_CHAIN_ID | BASE_CHAIN_ID | OPTIMISM_SEPOLIA_CHAIN_ID | BASE_SEPOLIA_CHAIN_ID
    )
}

/// Checks if a chain ID corresponds to a Linea L2 chain.
///
/// # Arguments
/// * `chain_id` - The chain ID to check.
///
/// # Returns
/// * `bool` - True if the chain is a Linea chain (mainnet or Sepolia).
pub fn is_linea_chain(chain_id: u64) -> bool {
    matches!(chain_id, LINEA_CHAIN_ID | LINEA_SEPOLIA_CHAIN_ID)
}

/// Checks if a chain ID corresponds to an Ethereum L1 chain.
///
/// # Arguments
/// * `chain_id` - The chain ID to check.
///
/// # Returns
/// * `bool` - True if the chain is an Ethereum chain (mainnet or Sepolia).
pub fn is_ethereum_chain(chain_id: u64) -> bool {
    matches!(chain_id, ETHEREUM_CHAIN_ID | ETHEREUM_SEPOLIA_CHAIN_ID)
}

/// Returns the L1 message service contract address for a given Linea chain ID.
///
/// # Arguments
/// * `chain_id` - The Linea chain ID (mainnet or Sepolia).
///
/// # Returns
/// * `Address` - The L1 message service contract address.
///
/// # Panics
/// Panics if an invalid or non-Linea chain ID is provided.
pub fn get_linea_message_service_address(chain_id: u64) -> Address {
    match chain_id {
        LINEA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA,
        LINEA_SEPOLIA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA_SEPOLIA,
        _ => panic!("invalid chain id for linea message service: {}", chain_id),
    }
}

/// Returns the risc0-steel chain spec for a given chain ID.
///
/// Used by the risc0-steel coprocessor to configure EVM environments.
/// Returns Linea-specific chain specs for Linea chains, otherwise defaults
/// to the Ethereum mainnet chain spec.
///
/// # Arguments
/// * `chain_id` - The chain ID to get the spec for.
///
/// # Returns
/// * `&'static EthChainSpec` - Reference to the risc0-steel chain specification.
pub fn get_steel_chain_spec(chain_id: u64) -> &'static EthChainSpec {
    match chain_id {
        LINEA_CHAIN_ID => &LINEA_MAINNET_CHAIN_SPEC,
        LINEA_SEPOLIA_CHAIN_ID => &LINEA_SEPOLIA_CHAIN_SPEC,
        _ => &ETH_MAINNET_CHAIN_SPEC,
    }
}
