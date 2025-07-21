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
//
//! Constants module containing RPC URLs, contract addresses, and other network-specific constants.
//!
//! This module provides centralized access to various network-specific constants, including:
//! - RPC endpoint URLs for different blockchain networks
//! - Sequencer request URLs for L2 networks
//! - WETH contract addresses across supported chains
//! - Constants used throughout the project for chain IDs, addresses, and cryptographic values.
//!
//! This module contains a comprehensive set of constant definitions that are used across different chains
//! and components of the Malda Protocol.

#[path = "../../malda_utils/src/constants.rs"]
mod constants;

pub use constants::*;

/// Generic function to retrieve environment variables as static strings
fn get_env_var(env_var: &str) -> &'static str {
    Box::leak(
        dotenvy::var(env_var)
            .unwrap_or_else(|_| panic!("{env_var} must be set in environment"))
            .into_boxed_str(),
    )
}

/// Unified function to get RPC URL for any chain
///
/// # Arguments
/// * `chain_name` - The chain name (e.g., "LINEA", "ETHEREUM", "BASE", "OPTIMISM")
/// * `fallback` - Whether to use fallback URL (default: false)
/// * `testnet` - Whether to use testnet (Sepolia) URL (default: false)
pub fn get_rpc_url(chain_name: &str, fallback: bool, testnet: bool) -> &'static str {
    let chain_upper = chain_name.to_uppercase();
    let fallback_suffix = if fallback { "_FALLBACK" } else { "" };
    let testnet_suffix = if testnet { "_SEPOLIA" } else { "" };

    let env_var = format!(
        "RPC_URL_{}{}{}",
        chain_upper, testnet_suffix, fallback_suffix
    );
    get_env_var(&env_var)
}

/// Unified function to get sequencer request URL for L2 chains
///
/// # Arguments
/// * `chain_name` - The L2 chain name (e.g., "OPTIMISM", "BASE")
/// * `fallback` - Whether to use fallback URL (default: false)
/// * `testnet` - Whether to use testnet (Sepolia) URL (default: false)
pub fn get_sequencer_request_url(chain_name: &str, fallback: bool, testnet: bool) -> &'static str {
    let chain_upper = chain_name.to_uppercase();
    let fallback_suffix = if fallback { "_FALLBACK" } else { "" };
    let testnet_suffix = if testnet { "_SEPOLIA" } else { "" };

    let env_var = format!(
        "SEQUENCER_REQUEST_{}{}{}",
        chain_upper, testnet_suffix, fallback_suffix
    );
    get_env_var(&env_var)
}
