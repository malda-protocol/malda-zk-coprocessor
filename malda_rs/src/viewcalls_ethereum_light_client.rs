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

use helios_ethereum::rpc::http_rpc::HttpRpc;
use helios_ethereum::rpc::ConsensusRpc;
use consensus_core::{
    calc_sync_period,
    consensus_spec::MainnetConsensusSpec,
    types::{Bootstrap, Update, OptimisticUpdate, BeaconBlock, LightClientHeader},
};
use ssz_types::FixedVector;

use risc0_steel::{
    ethereum::{EthEvmEnv, EthEvmInput, EthEvmFactory, ETH_MAINNET_CHAIN_SPEC},
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

/// Helper function to write LightClientHeader by deconstructing its fields
fn write_light_client_header(env: &mut risc0_zkvm::ExecutorEnvBuilder, header: &LightClientHeader) {
    match header {
        LightClientHeader::Bellatrix(bellatrix) => {
            println!("Writing LightClientHeader variant: 0 (Bellatrix)");
            env.write(&0u8).unwrap(); // Variant discriminant
            env.write(&bellatrix.beacon).unwrap();
        }
        LightClientHeader::Capella(capella) => {
            println!("Writing LightClientHeader variant: 1 (Capella)");
            env.write(&1u8).unwrap(); // Variant discriminant
            env.write(&capella.beacon).unwrap();
            write_execution_payload_header(env, &capella.execution);
            env.write(&capella.execution_branch).unwrap();
        }
        LightClientHeader::Deneb(deneb) => {
            println!("Writing LightClientHeader variant: 2 (Deneb)");
            env.write(&2u8).unwrap(); // Variant discriminant
            env.write(&deneb.beacon).unwrap();
            write_execution_payload_header(env, &deneb.execution);
            env.write(&deneb.execution_branch).unwrap();
        }
        LightClientHeader::Electra(electra) => {
            println!("Writing LightClientHeader variant: 3 (Electra)");
            env.write(&3u8).unwrap(); // Variant discriminant
            env.write(&electra.beacon).unwrap();
            write_execution_payload_header(env, &electra.execution);
            env.write(&electra.execution_branch).unwrap();
        }
    }
}

/// Helper function to write ExecutionPayloadHeader by deconstructing its fields
fn write_execution_payload_header(env: &mut risc0_zkvm::ExecutorEnvBuilder, header: &consensus_core::types::ExecutionPayloadHeader) {
    match header {
        consensus_core::types::ExecutionPayloadHeader::Bellatrix(bellatrix) => {
            println!("Writing ExecutionPayloadHeader variant: 0 (Bellatrix)");
            env.write(&0u8).unwrap(); // Variant discriminant
            env.write(&bellatrix.parent_hash).unwrap();
            env.write(&bellatrix.fee_recipient).unwrap();
            env.write(&bellatrix.state_root).unwrap();
            env.write(&bellatrix.receipts_root).unwrap();
            env.write(&bellatrix.logs_bloom).unwrap();
            env.write(&bellatrix.prev_randao).unwrap();
            env.write(&bellatrix.block_number).unwrap();
            env.write(&bellatrix.gas_limit).unwrap();
            env.write(&bellatrix.gas_used).unwrap();
            env.write(&bellatrix.timestamp).unwrap();
            env.write(&bellatrix.extra_data).unwrap();
            env.write(&bellatrix.base_fee_per_gas).unwrap();
            env.write(&bellatrix.block_hash).unwrap();
            env.write(&bellatrix.transactions_root).unwrap();
        }
        consensus_core::types::ExecutionPayloadHeader::Capella(capella) => {
            println!("Writing ExecutionPayloadHeader variant: 1 (Capella)");
            env.write(&1u8).unwrap(); // Variant discriminant
            env.write(&capella.parent_hash).unwrap();
            env.write(&capella.fee_recipient).unwrap();
            env.write(&capella.state_root).unwrap();
            env.write(&capella.receipts_root).unwrap();
            env.write(&capella.logs_bloom).unwrap();
            env.write(&capella.prev_randao).unwrap();
            env.write(&capella.block_number).unwrap();
            env.write(&capella.gas_limit).unwrap();
            env.write(&capella.gas_used).unwrap();
            env.write(&capella.timestamp).unwrap();
            env.write(&capella.extra_data).unwrap();
            env.write(&capella.base_fee_per_gas).unwrap();
            env.write(&capella.block_hash).unwrap();
            env.write(&capella.transactions_root).unwrap();
            env.write(&capella.withdrawals_root).unwrap();
        }
        consensus_core::types::ExecutionPayloadHeader::Deneb(deneb) => {
            println!("Writing ExecutionPayloadHeader variant: 2 (Deneb)");
            env.write(&2u8).unwrap(); // Variant discriminant
            env.write(&deneb.parent_hash).unwrap();
            env.write(&deneb.fee_recipient).unwrap();
            env.write(&deneb.state_root).unwrap();
            env.write(&deneb.receipts_root).unwrap();
            env.write(&deneb.logs_bloom).unwrap();
            env.write(&deneb.prev_randao).unwrap();
            env.write(&deneb.block_number).unwrap();
            env.write(&deneb.gas_limit).unwrap();
            env.write(&deneb.gas_used).unwrap();
            env.write(&deneb.timestamp).unwrap();
            env.write(&deneb.extra_data).unwrap();
            env.write(&deneb.base_fee_per_gas).unwrap();
            env.write(&deneb.block_hash).unwrap();
            env.write(&deneb.transactions_root).unwrap();
            env.write(&deneb.withdrawals_root).unwrap();
            env.write(&deneb.blob_gas_used).unwrap();
            env.write(&deneb.excess_blob_gas).unwrap();
        }
        consensus_core::types::ExecutionPayloadHeader::Electra(electra) => {
            println!("Writing ExecutionPayloadHeader variant: 3 (Electra)");
            env.write(&3u8).unwrap(); // Variant discriminant
            env.write(&electra.parent_hash).unwrap();
            env.write(&electra.fee_recipient).unwrap();
            env.write(&electra.state_root).unwrap();
            env.write(&electra.receipts_root).unwrap();
            env.write(&electra.logs_bloom).unwrap();
            env.write(&electra.prev_randao).unwrap();
            env.write(&electra.block_number).unwrap();
            env.write(&electra.gas_limit).unwrap();
            env.write(&electra.gas_used).unwrap();
            env.write(&electra.timestamp).unwrap();
            env.write(&electra.extra_data).unwrap();
            env.write(&electra.base_fee_per_gas).unwrap();
            env.write(&electra.block_hash).unwrap();
            env.write(&electra.transactions_root).unwrap();
            env.write(&electra.withdrawals_root).unwrap();
            env.write(&electra.blob_gas_used).unwrap();
            env.write(&electra.excess_blob_gas).unwrap();
        }
    }
}

// Add RPC URL functions
fn rpc_url_ethereum() -> &'static str {
    get_rpc_url("ETHEREUM", false, false)
}

fn rpc_url_beacon() -> &'static str {
    get_env_var("RPC_URL_BEACON")
}

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

    let beacon_rpc = <HttpRpc as ConsensusRpc<MainnetConsensusSpec>>::new(rpc_url_beacon);
    let beacon_root = trusted_hash;
    let bootstrap: Bootstrap<MainnetConsensusSpec> = beacon_rpc.get_bootstrap(beacon_root).await.unwrap();
    let current_period = calc_sync_period::<MainnetConsensusSpec>(bootstrap.header().beacon().slot);

    let updates: Vec<Update<MainnetConsensusSpec>> = beacon_rpc.get_updates(current_period, 10).await.unwrap();
    let finality_update = beacon_rpc.get_optimistic_update().await.unwrap();

    // let current_beacon_root = finality_update.attested_header.tree_root_hash();
    let beacon_block_slot = finality_update.attested_header.beacon().slot;
    let beacon_block: BeaconBlock<MainnetConsensusSpec> = beacon_rpc.get_block(beacon_block_slot).await.unwrap();
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
) -> EvmInput<EthEvmFactory> {
    let reorg_protection_depth = match chain_id {
        OPTIMISM_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM,
        BASE_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE,
        LINEA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA,
        ETHEREUM_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM,
        OPTIMISM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
        BASE_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
        LINEA_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
        ETHEREUM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let block_reorg_protected = block - reorg_protection_depth;

    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(chain_url).unwrap())
        .block_number_or_tag(BlockNumberOrTag::Number(block_reorg_protected))
        .beacon_api(Url::parse(rpc_url_beacon()).unwrap())
        .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
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
        OPTIMISM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
        BASE_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
        LINEA_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
        ETHEREUM_SEPOLIA_CHAIN_ID => REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let mut linking_blocks = vec![];

    let start_block = current_block - reorg_protection_depth + 1;

    for block_nr in (start_block)..=(current_block) {
        let env = EthEvmEnv::builder()
            .rpc(Url::parse(rpc_url).unwrap())
            .block_number_or_tag(BlockNumberOrTag::Number(block_nr))
            .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
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
    view_call_input: EvmInput<EthEvmFactory>,
    chain_id: u64,
    user: Address,
    market: Address,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    linking_blocks: Vec<RlpHeader<Header>>,
    bootstrap: Bootstrap<MainnetConsensusSpec>,
    checkpoint: B256,
    updates: Vec<Update<MainnetConsensusSpec>>,
    finality_update: OptimisticUpdate<MainnetConsensusSpec>,
    beacon_input: EvmInput<EthEvmFactory>,
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
        .unwrap();
    
    write_light_client_header(&mut env, &bootstrap.header());
    env.write(&bootstrap.current_sync_committee())
        .unwrap()
        .write(&bootstrap.current_sync_committee_branch())
        .unwrap()
        .write(&checkpoint)
        .unwrap();
    
    write_light_client_header(&mut env, &finality_update.attested_header);
    env.write(&finality_update.sync_aggregate)
        .unwrap()
        .write(&finality_update.signature_slot)
        .unwrap()
        .write(&updates.len())
        .unwrap();

    for update in updates {
        write_light_client_header(&mut env, &update.attested_header());
        env.write(&update.next_sync_committee()).unwrap();
        env.write(&update.next_sync_committee_branch()).unwrap();
        write_light_client_header(&mut env, &update.finalized_header());
        env.write(&update.finality_branch()).unwrap();
        env.write(&update.sync_aggregate()).unwrap();
        env.write(&update.signature_slot()).unwrap();
    }

    env.write(&beacon_input).unwrap();

    env.build().unwrap()
}
