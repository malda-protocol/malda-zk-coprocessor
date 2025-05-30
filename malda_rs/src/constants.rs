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

/// RPC endpoint URLs for supported networks
pub fn rpc_url_linea() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_LINEA")
            .expect("RPC_URL_LINEA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_scroll() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_SCROLL")
            .expect("RPC_URL_SCROLL must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_ethereum() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_ETHEREUM")
            .expect("RPC_URL_ETHEREUM must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_base() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_BASE")
            .expect("RPC_URL_BASE must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_optimism() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_OPTIMISM")
            .expect("RPC_URL_OPTIMISM must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_arbitrum() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_ARBITRUM")
            .expect("RPC_URL_ARBITRUM must be set in environment")
            .into_boxed_str(),
    )
}

/// Sepolia testnet RPCs
pub fn rpc_url_linea_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_LINEA_SEPOLIA")
            .expect("RPC_URL_LINEA_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_scroll_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_SCROLL_SEPOLIA")
            .expect("RPC_URL_SCROLL_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_ethereum_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_ETHEREUM_SEPOLIA")
            .expect("RPC_URL_ETHEREUM_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_base_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_BASE_SEPOLIA")
            .expect("RPC_URL_BASE_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_optimism_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_OPTIMISM_SEPOLIA")
            .expect("RPC_URL_OPTIMISM_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_arbitrum_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_ARBITRUM_SEPOLIA")
            .expect("RPC_URL_ARBITRUM_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn rpc_url_beacon() -> &'static str {
    Box::leak(
        dotenvy::var("RPC_URL_BEACON")
            .expect("RPC_URL_BEACON must be set in environment")
            .into_boxed_str(),
    )
}

/// Sequencer request URLs for Layer 2 networks
pub fn sequencer_request_optimism() -> &'static str {
    Box::leak(
        dotenvy::var("SEQUENCER_REQUEST_OPTIMISM")
            .expect("SEQUENCER_REQUEST_OPTIMISM must be set in environment")
            .into_boxed_str(),
    )
}

pub fn sequencer_request_base() -> &'static str {
    Box::leak(
        dotenvy::var("SEQUENCER_REQUEST_BASE")
            .expect("SEQUENCER_REQUEST_BASE must be set in environment")
            .into_boxed_str(),
    )
}

pub fn sequencer_request_optimism_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("SEQUENCER_REQUEST_OPTIMISM_SEPOLIA")
            .expect("SEQUENCER_REQUEST_OPTIMISM_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}

pub fn sequencer_request_base_sepolia() -> &'static str {
    Box::leak(
        dotenvy::var("SEQUENCER_REQUEST_BASE_SEPOLIA")
            .expect("SEQUENCER_REQUEST_BASE_SEPOLIA must be set in environment")
            .into_boxed_str(),
    )
}
