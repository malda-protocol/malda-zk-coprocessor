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
//! Ethereum view call utilities for cross-chain view call proof.
//!
//! This module provides functionality to:
//! - Generate zero-knowledge proofs for proof data queries across EVM chains
//! - Execute and verify proof data queries using RISC Zero
//! - Handle Ethereum consensus layer (beacon chain) data verification
//! - Process block headers for reorg protection
//! - Build execution environments for zero-knowledge proofs

use alloy_consensus::Header;
use alloy_primitives::{Address, B256};
use alloy_primitives_old::B256 as OldB256;

use consensus::rpc::{nimbus_rpc::NimbusRpc, ConsensusRpc};
use consensus_core::{
    calc_sync_period,
    types::{Bootstrap, OptimisticUpdate, Update},
};

use risc0_steel::{
    ethereum::{EthEvmEnv, EthEvmInput},
    host::BlockNumberOrTag,
    serde::RlpHeader,
    Contract, EvmInput,
};
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv, ProveInfo, SessionInfo};

use anyhow::Error;
use tokio;
use url::Url;

use crate::constants::*;
use crate::elfs_ids::GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_ELF;
use crate::types::{IMaldaMarket, SequencerCommitment};

/// Generates a zero-knowledge proof for a user's proof data query.
///
/// # Arguments
///
/// * `user` - The user's Ethereum address
/// * `market` - The market contract address to query
/// * `chain_id` - The target chain identifier
/// * `trusted_hash` - The trusted beacon chain block hash to anchor verification from
///
/// # Returns
///
/// Returns a `Result` containing the zero-knowledge `ProveInfo` or an error
pub async fn get_proof_data_prove(
    user: Address,
    market: Address,
    chain_id: u64,
    trusted_hash: B256,
) -> Result<ProveInfo, Error> {
    // Move all the work including env creation into the blocking task
    let prove_info = tokio::task::spawn_blocking(move || {
        // Create a new runtime for async operations within the blocking task
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Execute the async env creation in the new runtime
        let env = rt.block_on(get_proof_data_zkvm_env(
            user,
            market,
            chain_id,
            trusted_hash,
        ));

        // Perform the proving
        default_prover().prove(env, GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_ELF)
    })
    .await?;

    prove_info
}

/// Executes a proof data query without generating a proof.
///
/// Useful for testing and debugging proof data queries before generating proofs.
///
/// # Arguments
///
/// * `user` - The user's Ethereum address
/// * `market` - The market contract address to query
/// * `chain_id` - The target chain identifier
/// * `trusted_hash` - The trusted beacon chain block hash to anchor verification from
///
/// # Returns
///
/// Returns a `Result` containing the execution `SessionInfo` or an error
pub async fn get_proof_data_exec(
    user: Address,
    market: Address,
    chain_id: u64,
    trusted_hash: B256,
) -> Result<SessionInfo, Error> {
    let env = get_proof_data_zkvm_env(user, market, chain_id, trusted_hash).await;
    default_executor().execute(env, GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_ELF)
}

/// Creates a RISC Zero executor environment for proof data queries.
///
/// This function:
/// 1. Fetches and validates beacon chain consensus data
/// 2. Retrieves necessary block headers for reorg protection
/// 3. Prepares the proof data query call data
/// 4. Builds a complete environment for zero-knowledge proof generation
///
/// # Arguments
///
/// * `user` - The user's Ethereum address
/// * `market` - The market contract address to query
/// * `chain_id` - The target chain identifier
/// * `trusted_hash` - The trusted beacon chain block hash to anchor verification from
///
/// # Returns
///
/// Returns an `ExecutorEnv` configured for generating proof data query proofs
///
/// # Panics
///
/// Panics if an unsupported chain ID is provided
pub async fn get_proof_data_zkvm_env(
    user: Address,
    market: Address,
    chain_id: u64,
    trusted_hash: B256,
) -> ExecutorEnv<'static> {
    let (rpc_url, rpc_url_beacon) = match chain_id {
        ETHEREUM_CHAIN_ID => (rpc_url_ethereum(), rpc_url_beacon()),
        _ => panic!("Invalid chain ID"),
    };

    let beacon_rpc = NimbusRpc::new(rpc_url_beacon);
    let beacon_root = OldB256::from(trusted_hash.0);
    let bootstrap: Bootstrap = beacon_rpc.get_bootstrap(beacon_root).await.unwrap();
    let current_period = calc_sync_period(bootstrap.header.beacon.slot);

    let updates: Vec<Update> = beacon_rpc.get_updates(current_period, 10).await.unwrap();
    let finality_update = beacon_rpc.get_optimistic_update().await.unwrap();

    // let current_beacon_root = finality_update.attested_header.tree_root_hash();
    let beacon_block_slot = finality_update.attested_header.beacon.slot;
    let beacon_block = beacon_rpc.get_block(beacon_block_slot).await.unwrap();
    let block = beacon_block.body.execution_payload().block_number().clone();

    let linking_blocks = get_linking_blocks(chain_id, rpc_url, block).await;
    let proof_data_call_input =
        get_proof_data_call_input(chain_id, rpc_url, block, user, market).await;

    let beacon_proof_data_input = get_proof_data_call_input(
        chain_id,
        rpc_url,
        block + REORG_PROTECTION_DEPTH_ETHEREUM,
        user,
        market,
    )
    .await;

    build_l1_chain_builder_environment(
        proof_data_call_input,
        chain_id,
        user,
        market,
        None,
        None,
        linking_blocks,
        bootstrap,
        beacon_root,
        updates,
        finality_update,
        beacon_proof_data_input,
    )
}

/// Constructs an EVM input for a proof data query.
///
/// Prepares the encoded EVM call data for querying an ERC20 token's getProofData function,
/// taking into account chain-specific reorg protection depths.
///
/// # Arguments
///
/// * `chain_id` - The target chain identifier
/// * `chain_url` - RPC endpoint URL for the target chain
/// * `block` - Block number to query at
/// * `user` - Address of the user to query
/// * `market` - Token contract address to query
///
/// # Returns
///
/// Returns an `EvmInput` containing the encoded proof data call and block header data
pub async fn get_proof_data_call_input(
    chain_id: u64,
    chain_url: &str,
    block: u64,
    user: Address,
    market: Address,
) -> EvmInput<RlpHeader<Header>> {
    let reorg_protection_depth = match chain_id {
        OPTIMISM_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM,
        BASE_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE,
        LINEA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA,
        ETHEREUM_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM,
        SCROLL_CHAIN_ID => REORG_PROTECTION_DEPTH_SCROLL,
        OPTIMISM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
        BASE_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
        LINEA_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
        ETHEREUM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
        SCROLL_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_SCROLL_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let block_reorg_protected = block - reorg_protection_depth;

    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(chain_url).unwrap())
        .block_number_or_tag(BlockNumberOrTag::Number(block_reorg_protected))
        .beacon_api(Url::parse(rpc_url_beacon()).unwrap())
        .build()
        .await
        .unwrap();

    let call = IMaldaMarket::getProofDataCall {
        account: user,
        dstChainId: chain_id as u32,
    };

    let mut contract = Contract::preflight(market, &mut env);
    let _returns = contract.call_builder(&call).call().await.unwrap();

    env.into_input().await.unwrap()
}

/// Fetches a sequence of Ethereum blocks for reorg protection.
///
/// Retrieves a continuous sequence of block headers starting from a given block,
/// going back by the chain-specific reorg protection depth. This ensures the
/// balance proof remains valid even if a chain reorganization occurs.
///
/// # Arguments
///
/// * `chain_id` - The target chain identifier
/// * `rpc_url` - RPC endpoint URL for the target chain
/// * `current_block` - The latest block number to start from
///
/// # Returns
///
/// Returns a vector of block headers covering the reorg protection window
///
/// # Panics
///
/// Panics if an unsupported chain ID is provided
pub async fn get_linking_blocks(
    chain_id: u64,
    rpc_url: &str,
    current_block: u64,
) -> Vec<RlpHeader<Header>> {
    let reorg_protection_depth = match chain_id {
        OPTIMISM_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM,
        BASE_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE,
        LINEA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA,
        ETHEREUM_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM,
        SCROLL_CHAIN_ID => REORG_PROTECTION_DEPTH_SCROLL,
        OPTIMISM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
        BASE_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
        LINEA_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
        ETHEREUM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
        SCROLL_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_SCROLL_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let mut linking_blocks = vec![];

    let start_block = current_block - reorg_protection_depth + 1;

    for block_nr in (start_block)..=(current_block) {
        let env = EthEvmEnv::builder()
            .rpc(Url::parse(rpc_url).unwrap())
            .block_number_or_tag(BlockNumberOrTag::Number(block_nr))
            .build()
            .await
            .unwrap();
        let header = env.header().inner().clone();
        linking_blocks.push(header);
    }
    linking_blocks
}

/// Builds a complete RISC Zero environment for L1 chain verification.
///
/// Assembles all necessary components for verifying L1 data, including:
/// - View call inputs and chain identification
/// - User and asset addresses
/// - Sequencer commitments (for L2 chains)
/// - Block headers for reorg protection
/// - Beacon chain consensus data
/// - Additional verification data for the beacon chain
///
/// This environment enables zero-knowledge proofs that demonstrate valid
/// token balance queries while ensuring consensus-layer security.
pub fn build_l1_chain_builder_environment(
    view_call_input: EvmInput<RlpHeader<Header>>,
    chain_id: u64,
    user: Address,
    market: Address,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    linking_blocks: Vec<RlpHeader<Header>>,
    bootstrap: Bootstrap,
    checkpoint: OldB256,
    updates: Vec<Update>,
    finality_update: OptimisticUpdate,
    beacon_input: EvmInput<RlpHeader<Header>>,
) -> risc0_zkvm::ExecutorEnv<'static> {
    let mut env = risc0_zkvm::ExecutorEnv::builder();
    env.write(&view_call_input)
        .unwrap()
        .write(&chain_id)
        .unwrap()
        .write(&user)
        .unwrap()
        .write(&market)
        .unwrap()
        .write(&sequencer_commitment)
        .unwrap()
        .write(&env_op_input)
        .unwrap()
        .write(&linking_blocks)
        .unwrap()
        .write(&bootstrap.header)
        .unwrap()
        .write(&bootstrap.current_sync_committee)
        .unwrap()
        .write(&bootstrap.current_sync_committee_branch)
        .unwrap()
        .write(&checkpoint)
        .unwrap()
        .write(&finality_update.attested_header)
        .unwrap()
        .write(&finality_update.sync_aggregate)
        .unwrap()
        .write(&finality_update.signature_slot)
        .unwrap()
        .write(&updates.len())
        .unwrap();

    for update in updates {
        env.write(&update.attested_header).unwrap();
        env.write(&update.next_sync_committee).unwrap();
        env.write(&update.next_sync_committee_branch).unwrap();
        env.write(&update.finalized_header).unwrap();
        env.write(&update.finality_branch).unwrap();
        env.write(&update.sync_aggregate).unwrap();
        env.write(&update.signature_slot).unwrap();
    }

    env.write(&beacon_input).unwrap();

    env.build().unwrap()
}
