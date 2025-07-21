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
//! Validator functions for verifying blockchain environments and commitments.
//!
//! This module provides validation utilities for:
//! - Proof data queries across multiple EVM chains
//! - Linea block validation through sequencer signatures
//! - OpStack (Optimism/Base) validation through sequencer commitments
//! - Ethereum L1 block validation through OpStack L2s
//! - Chain length validation for reorg protection
//!
//! Supported networks include:
//! - Ethereum (L1) - Mainnet and Sepolia
//! - Optimism - Mainnet and Sepolia
//! - Base - Mainnet and Sepolia
//! - Linea - Mainnet and Sepolia

use crate::constants::*;
use crate::cryptography::{recover_signer, signature_from_bytes};
use crate::types::*;
use alloy_consensus::Header;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_sol_types::SolValue;
use risc0_op_steel::optimism::{OpEvmFactory, OpEvmInput, OP_MAINNET_CHAIN_SPEC};
use risc0_steel::EvmFactory;
use risc0_steel::{
    ethereum::{EthEvmFactory, EthEvmInput, ETH_MAINNET_CHAIN_SPEC},
    serde::RlpHeader,
    Commitment, Contract, EvmEnv, StateDb,
};

/// Validates and executes proof data queries across multiple accounts and tokens using multicall.
///
/// This function orchestrates the validation of proof data queries for multiple accounts and assets across different EVM chains. It sorts and verifies the relevant parameters, validates block hashes and chain length for reorg protection, and executes a batch multicall to retrieve proof data.
///
/// # Arguments
/// * `chain_id` - The chain ID to validate against.
/// * `account` - Vector of account addresses to query.
/// * `asset` - Vector of token contract addresses to query.
/// * `target_chain_ids` - Vector of target chain IDs for each account.
/// * `env_input_for_viewcall` - Optional EVM environment input for the chain.
/// * `sequencer_commitment_opstack` - Optional sequencer commitment for L2 chains.
/// * `env_input_opstack_for_l1_block_call` - Optional Optimism environment input for L1 validation.
/// * `linking_blocks` - Vector of blocks for reorg protection.
/// * `output` - Output vector for proof data results.
/// * `env_input_eth_for_l1_inclusion` - Optional Ethereum environment input for L1 inclusion.
/// * `env_input_opstack_for_viewcall_with_l1_inclusion` - Optional OpStack environment input for L1 inclusion.
/// * `sequencer_commitment_opstack_2` - Optional second sequencer commitment for L2 chains.
/// * `env_input_opstack_for_l1_block_call_2` - Optional second Optimism environment input for L1 validation.
///
/// # Panics
/// Panics if:
/// * Chain ID is invalid
/// * Environment validation fails
/// * Chain length is insufficient
/// * Block hashes don't match
/// * Multicall execution fails
/// * Return data decoding fails
pub fn validate_get_proof_data_call(
    chain_id: u64,
    account: Vec<Address>,
    asset: Vec<Address>,
    target_chain_ids: Vec<u64>,
    env_input_for_viewcall: Option<EthEvmInput>,
    sequencer_commitment_opstack: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call: Option<EthEvmInput>,
    linking_blocks: &Vec<RlpHeader<Header>>,
    output: &mut Vec<Bytes>,
    env_input_eth_for_l1_inclusion: &Option<EthEvmInput>,
    env_input_opstack_for_viewcall_with_l1_inclusion: Option<OpEvmInput>,
    sequencer_commitment_opstack_2: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call_2: Option<EthEvmInput>,
) {
    // Sort and verify all relevant parameters for the proof data call, including environment and block headers.
    let (
        env_for_viewcall,
        block_header_to_validate,
        env_header_hash_to_validate,
        env_header_to_validate,
        op_env_for_viewcall_with_l1_inclusion,
        op_env_commitment,
        chain_id_for_length_validation,
        validate_l1_inclusion,
    ) = sort_and_verify_relevant_params(
        chain_id,
        env_input_for_viewcall,
        linking_blocks,
        env_input_eth_for_l1_inclusion,
        env_input_opstack_for_viewcall_with_l1_inclusion,
    );

    // Validate the block hash for the given chain and environment.
    let validated_block_hash = get_validated_block_hash(
        chain_id,
        env_header_to_validate,
        sequencer_commitment_opstack,
        env_input_opstack_for_l1_block_call,
        env_input_eth_for_l1_inclusion,
        block_header_to_validate,
        validate_l1_inclusion,
        op_env_commitment.as_ref(),
        sequencer_commitment_opstack_2,
        env_input_opstack_for_l1_block_call_2,
    );

    // Ensure the chain length and hash linking are valid for reorg protection.
    validate_chain_length(
        chain_id_for_length_validation,
        env_header_hash_to_validate,
        linking_blocks,
        validated_block_hash,
    );

    // Execute the batch multicall to retrieve proof data, using the appropriate environment.
    if op_env_for_viewcall_with_l1_inclusion.is_some() {
        batch_call_get_proof_data(
            chain_id,
            account,
            asset,
            target_chain_ids,
            op_env_for_viewcall_with_l1_inclusion.unwrap(),
            validate_l1_inclusion,
            output,
        )
    } else {
        batch_call_get_proof_data(
            chain_id,
            account,
            asset,
            target_chain_ids,
            env_for_viewcall,
            validate_l1_inclusion,
            output,
        );
    }
}

/// Sorts and verifies relevant parameters for proof data validation.
///
/// This function processes and validates input parameters for different chain types,
/// handling both L1 and L2 validation scenarios. It determines which environment and block header
/// should be used for subsequent validation, based on the chain type and inclusion requirements.
///
/// # Arguments
/// * `chain_id` - The chain ID to determine validation strategy.
/// * `env_input_for_viewcall` - Optional EVM input for view calls (used for L1 or Linea chains).
/// * `linking_blocks` - Vector of blocks for reorg protection.
/// * `env_input_eth_for_l1_inclusion` - Optional Ethereum input for L1 inclusion (used for OpStack/Linea L2s).
/// * `env_input_opstack_for_viewcall_with_l1_inclusion` - Optional OpStack input for L1 inclusion (used for OpStack L2s).
///
/// # Returns
/// Returns a tuple containing:
/// * `EvmEnv` - The validated EVM environment for the view call.
/// * `RlpHeader<Header>` - The block header to validate (from the environment or linking blocks).
/// * `B256` - The header hash to validate.
/// * `Header` - The inner header to validate.
/// * `Option<EvmEnv>` - Optional OpStack EVM environment (for L1 inclusion).
/// * `Option<Commitment>` - Optional commitment (for OpStack L1 inclusion).
/// * `u64` - Chain ID for length validation.
/// * `bool` - Whether to validate L1 inclusion.
///
/// # Panics
/// Panics if:
/// * Chain ID is invalid.
/// * Required environment inputs are missing.
/// * Parameter validation fails.
pub fn sort_and_verify_relevant_params(
    chain_id: u64,
    env_input_for_viewcall: Option<EthEvmInput>,
    linking_blocks: &Vec<RlpHeader<Header>>,
    env_input_eth_for_l1_inclusion: &Option<EthEvmInput>,
    env_input_opstack_for_viewcall_with_l1_inclusion: Option<OpEvmInput>,
) -> (
    EvmEnv<StateDb, EthEvmFactory, Commitment>,
    RlpHeader<Header>,
    B256,
    Header,
    Option<EvmEnv<StateDb, OpEvmFactory, Commitment>>,
    Option<Commitment>,
    u64,
    bool,
) {
    let validate_l1_inclusion = env_input_eth_for_l1_inclusion.is_some();

    // Determine which environment and parameters to use based on chain type and inclusion requirements.
    let (
        env_for_viewcall,
        op_env_for_viewcall_with_l1_inclusion,
        op_env_commitment,
        chain_id_for_length_validation,
    ) = if (chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID)
        && validate_l1_inclusion
    {
        // For OpStack L2s with L1 inclusion, use the L1 environment and OpStack environment for inclusion.
        let env_for_viewcall = env_input_eth_for_l1_inclusion
            .as_ref()
            .expect("env_eth_input is None")
            .clone()
            .into_env(&ETH_MAINNET_CHAIN_SPEC);
        let op_env_for_viewcall_with_l1_inclusion =
            env_input_opstack_for_viewcall_with_l1_inclusion
                .expect("op_evm_input is None")
                .into_env(&OP_MAINNET_CHAIN_SPEC);
        let op_env_commitment = op_env_for_viewcall_with_l1_inclusion.commitment().clone();
        let chain_id_for_length_validation = match chain_id {
            OPTIMISM_CHAIN_ID | BASE_CHAIN_ID => ETHEREUM_CHAIN_ID,
            OPTIMISM_SEPOLIA_CHAIN_ID | BASE_SEPOLIA_CHAIN_ID => ETHEREUM_SEPOLIA_CHAIN_ID,
            _ => panic!("invalid chain id"),
        };
        (
            env_for_viewcall,
            Some(op_env_for_viewcall_with_l1_inclusion),
            Some(op_env_commitment),
            chain_id_for_length_validation,
        )
    } else {
        // For L1 or Linea chains, use the provided environment input.
        let chain_spec = match chain_id {
            LINEA_CHAIN_ID => &LINEA_MAINNET_CHAIN_SPEC,
            LINEA_SEPOLIA_CHAIN_ID => &LINEA_MAINNET_CHAIN_SPEC,
            _ => &ETH_MAINNET_CHAIN_SPEC,
        };

        (
            env_input_for_viewcall
                .expect("env_input is None")
                .into_env(&chain_spec),
            None,
            None,
            chain_id,
        )
    };

    // Select the block header to validate: use the last linking block if present, otherwise use the environment's header.
    let block_header_to_validate = if linking_blocks.is_empty() {
        env_for_viewcall.header().inner().clone()
    } else {
        linking_blocks[linking_blocks.len() - 1].clone()
    };

    let env_header_hash_to_validate = env_for_viewcall.header().seal();
    let env_header_to_validate = env_for_viewcall.header().inner().inner().clone();

    (
        env_for_viewcall,
        block_header_to_validate,
        env_header_hash_to_validate,
        env_header_to_validate,
        op_env_for_viewcall_with_l1_inclusion,
        op_env_commitment,
        chain_id_for_length_validation,
        validate_l1_inclusion,
    )
}

/// Validates an OpStack dispute game commitment.
///
/// This function verifies the dispute game state and commitment for OpStack chains,
/// ensuring the game is valid and properly resolved. It checks the game type, creation time,
/// status, blacklist status, resolution time, and root claim.
///
/// # Arguments
/// * `chain_id` - The OpStack chain ID.
/// * `eth_env` - The Ethereum EVM environment.
/// * `op_env_commitment` - The OpStack commitment to validate.
///
/// # Panics
/// Panics if:
/// * Chain ID is invalid.
/// * Game type is not respected.
/// * Game was created before respected game type update.
/// * Game status is not DEFENDER_WINS.
/// * Game is blacklisted.
/// * Insufficient time has passed since game resolution.
/// * Root claim doesn't match.
pub fn validate_opstack_dispute_game_commitment(
    chain_id: u64,
    eth_env: EvmEnv<StateDb, EthEvmFactory, Commitment>,
    op_env_commitment: &Commitment,
) {
    // Decode the game index and root claim from the commitment.
    let (game_index, _version) = op_env_commitment.decode_id();
    let root_claim = op_env_commitment.digest;

    // Select the correct portal address for the given chain.
    let portal_adress = match chain_id {
        OPTIMISM_SEPOLIA_CHAIN_ID => OPTIMISM_SEPOLIA_PORTAL,
        BASE_SEPOLIA_CHAIN_ID => BASE_SEPOLIA_PORTAL,
        OPTIMISM_CHAIN_ID => OPTIMISM_PORTAL,
        BASE_CHAIN_ID => BASE_PORTAL,
        _ => panic!("invalid chain id"),
    };

    // Get the portal contract for additional checks.
    let portal_contract = Contract::new(portal_adress, &eth_env);

    // Get factory address from portal.
    let factory_call = IOptimismPortal::disputeGameFactoryCall {};
    let returns = portal_contract.call_builder(&factory_call).call();
    let factory_address = returns;

    // Query the dispute game at the given index.
    let game_call = IDisputeGameFactory::gameAtIndexCall { index: game_index };
    let contract = Contract::new(factory_address, &eth_env);
    let returns = contract.call_builder(&game_call).call();

    let game_type = returns._0;
    let created_at = returns._1;
    let game_address = returns._2;

    // Ensure the game type is respected (must be 0).
    assert_eq!(game_type, U256::from(0), "game type not respected game");

    // Check if game was created after respected game type update.
    let respected_game_type_updated_at_call = IOptimismPortal::respectedGameTypeUpdatedAtCall {};
    let updated_at = portal_contract
        .call_builder(&respected_game_type_updated_at_call)
        .call();
    assert!(
        created_at >= updated_at,
        "game created before respected game type update"
    );

    // Get game contract for status checks.
    let game_contract = Contract::new(game_address, &eth_env);

    // Check game status.
    let status_call = IDisputeGame::statusCall {};
    let status = game_contract.call_builder(&status_call).call();
    assert_eq!(
        status,
        GameStatus::DEFENDER_WINS,
        "game status not DEFENDER_WINS"
    );

    // Check if game is blacklisted.
    let blacklist_call = IOptimismPortal::disputeGameBlacklistCall { game: game_address };
    let is_blacklisted = portal_contract.call_builder(&blacklist_call).call();
    assert!(!is_blacklisted, "game is blacklisted");

    // Check game resolution time.
    let resolved_at_call = IDisputeGame::resolvedAtCall {};
    let resolved_at = game_contract.call_builder(&resolved_at_call).call();

    let proof_maturity_delay_call = IOptimismPortal::proofMaturityDelaySecondsCall {};
    let proof_maturity_delay = portal_contract
        .call_builder(&proof_maturity_delay_call)
        .call();

    let current_timestamp = eth_env.header().inner().inner().timestamp;
    assert!(
        U256::from(current_timestamp) - U256::from(resolved_at)
            > proof_maturity_delay - U256::from(300),
        "insufficient time passed since game resolution"
    );

    // Finally verify root claim matches.
    let root_claim_call = IDisputeGame::rootClaimCall {};
    let root_claim_return = game_contract.call_builder(&root_claim_call).call();
    assert_eq!(root_claim_return, root_claim, "root claim mismatch");
}

/// Retrieves validated block hash based on chain type and validation requirements.
///
/// This function dispatches to the appropriate block hash validation logic depending on the chain type.
///
/// # Arguments
/// * `chain_id` - The chain ID to determine validation strategy.
/// * `env_header_to_validate` - The block header to validate.
/// * `sequencer_commitment_opstack` - Optional sequencer commitment for L2 chains.
/// * `env_input_opstack_for_l1_block_call` - Optional Optimism environment input for L1 validation.
/// * `env_input_eth_for_l1_inclusion` - Optional Ethereum environment input for L1 inclusion validation.
/// * `block_header_to_validate` - Last block in the chain for hash validation.
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion.
/// * `op_env_commitment` - Optional storage hash for L1 inclusion validation.
/// * `sequencer_commitment_opstack_2` - Optional second sequencer commitment for L2 chains.
/// * `env_input_opstack_for_l1_block_call_2` - Optional second Optimism environment input for L1 validation.
///
/// # Returns
/// * `B256` - The validated block hash.
///
/// # Panics
/// Panics if:
/// * Chain ID is invalid or unsupported.
/// * Validation fails for the specific chain type.
pub fn get_validated_block_hash(
    chain_id: u64,
    env_header_to_validate: Header,
    sequencer_commitment_opstack: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call: Option<EthEvmInput>,
    env_input_eth_for_l1_inclusion: &Option<EthEvmInput>,
    block_header_to_validate: RlpHeader<Header>,
    validate_l1_inclusion: bool,
    op_env_commitment: Option<&Commitment>,
    sequencer_commitment_opstack_2: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call_2: Option<EthEvmInput>,
) -> B256 {
    // Dispatch to the correct validation logic based on chain type.
    if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
        get_validated_block_hash_linea(
            chain_id,
            env_header_to_validate,
            sequencer_commitment_opstack,
            env_input_opstack_for_l1_block_call,
            env_input_eth_for_l1_inclusion,
            block_header_to_validate,
            validate_l1_inclusion,
            sequencer_commitment_opstack_2,
            env_input_opstack_for_l1_block_call_2,
        )
    } else if chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
    {
        get_validated_block_hash_opstack(
            chain_id,
            sequencer_commitment_opstack,
            env_input_opstack_for_l1_block_call,
            env_input_eth_for_l1_inclusion,
            block_header_to_validate,
            validate_l1_inclusion,
            op_env_commitment,
            sequencer_commitment_opstack_2,
            env_input_opstack_for_l1_block_call_2,
        )
    } else if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID {
        get_validated_ethereum_block_hash_via_opstack(
            sequencer_commitment_opstack.as_ref(),
            env_input_opstack_for_l1_block_call,
            chain_id,
            sequencer_commitment_opstack_2.as_ref(),
            env_input_opstack_for_l1_block_call_2,
        )
    } else {
        panic!("invalid chain id");
    }
}

/// Validates OpStack block hash with optional L1 inclusion verification.
///
/// This function validates the block hash for OpStack chains (Optimism/Base), optionally verifying L1 inclusion if requested.
///
/// # Arguments
/// * `chain_id` - The OpStack chain ID (Optimism/Base).
/// * `sequencer_commitment` - Optional sequencer commitment.
/// * `env_input_opstack_for_l1_block_call` - Optional Optimism environment input.
/// * `env_input_eth_for_l1_inclusion` - Optional Ethereum environment input.
/// * `block_header_to_validate` - Last block for hash validation.
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion.
/// * `op_env_commitment` - Optional storage hash for L1 validation.
/// * `sequencer_commitment_opstack_2` - Optional second sequencer commitment.
/// * `env_input_opstack_for_l1_block_call_2` - Optional second Optimism environment input.
///
/// # Returns
/// * `B256` - The validated block hash.
///
/// # Panics
/// Panics if:
/// * Validation fails for OpStack environment.
/// * L1 inclusion validation fails when requested.
pub fn get_validated_block_hash_opstack(
    chain_id: u64,
    sequencer_commitment: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call: Option<EthEvmInput>,
    env_input_eth_for_l1_inclusion: &Option<EthEvmInput>,
    block_header_to_validate: RlpHeader<Header>,
    validate_l1_inclusion: bool,
    op_env_commitment: Option<&Commitment>,
    sequencer_commitment_opstack_2: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call_2: Option<EthEvmInput>,
) -> B256 {
    // Compute the hash of the block header to validate.
    let validated_hash = block_header_to_validate.hash_slow();
    if validate_l1_inclusion {
        // For L1 inclusion, determine the correct Ethereum chain ID.
        let ethereum_chain_id = match chain_id {
            OPTIMISM_CHAIN_ID | BASE_CHAIN_ID => ETHEREUM_CHAIN_ID,
            OPTIMISM_SEPOLIA_CHAIN_ID | BASE_SEPOLIA_CHAIN_ID => ETHEREUM_SEPOLIA_CHAIN_ID,
            _ => panic!("invalid chain id"),
        };

        // Validate the Ethereum block hash via OpStack.
        let ethereum_hash = get_validated_ethereum_block_hash_via_opstack(
            sequencer_commitment.as_ref(),
            env_input_opstack_for_l1_block_call,
            ethereum_chain_id,
            sequencer_commitment_opstack_2.as_ref(),
            env_input_opstack_for_l1_block_call_2,
        );

        // Ensure the hashes match.
        assert_eq!(ethereum_hash, validated_hash, "hash mismatch  opstack");
        // Validate the OpStack dispute game commitment.
        validate_opstack_dispute_game_commitment(
            chain_id,
            env_input_eth_for_l1_inclusion
                .as_ref()
                .unwrap()
                .clone()
                .into_env(&ETH_MAINNET_CHAIN_SPEC),
            op_env_commitment.unwrap(),
        )
    } else {
        // For non-L1 inclusion, validate the OpStack environment directly.
        validate_opstack_env(chain_id, &sequencer_commitment.unwrap(), validated_hash);
    }
    validated_hash
}

/// Validates Linea block hash with optional L1 inclusion verification.
///
/// This function validates the block hash for Linea chains, optionally verifying L1 inclusion if requested.
///
/// # Arguments
/// * `chain_id` - The Linea chain ID.
/// * `env_header_to_validate` - The block header to validate.
/// * `sequencer_commitment_opstack` - Optional sequencer commitment.
/// * `env_input_opstack_for_l1_block_call` - Optional Optimism environment input.
/// * `env_input_eth_for_l1_inclusion` - Optional Ethereum environment input.
/// * `block_header_to_validate` - Last block for hash validation.
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion.
/// * `sequencer_commitment_opstack_2` - Optional second sequencer commitment.
/// * `env_input_opstack_for_l1_block_call_2` - Optional second Optimism environment input.
///
/// # Returns
/// * `B256` - The validated block hash.
///
/// # Panics
/// Panics if:
/// * Validation fails for Linea environment.
/// * L1 inclusion validation fails when requested.
pub fn get_validated_block_hash_linea(
    chain_id: u64,
    env_header_to_validate: Header,
    sequencer_commitment_opstack: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call: Option<EthEvmInput>,
    env_input_eth_for_l1_inclusion: &Option<EthEvmInput>,
    block_header_to_validate: RlpHeader<Header>,
    validate_l1_inclusion: bool,
    sequencer_commitment_opstack_2: Option<SequencerCommitment>,
    env_input_opstack_for_l1_block_call_2: Option<EthEvmInput>,
) -> B256 {
    if validate_l1_inclusion {
        // For L1 inclusion, determine the correct Ethereum chain ID.
        let ethereum_chain_id = match chain_id {
            LINEA_CHAIN_ID => ETHEREUM_CHAIN_ID,
            LINEA_SEPOLIA_CHAIN_ID => ETHEREUM_SEPOLIA_CHAIN_ID,
            _ => panic!("invalid chain id"),
        };
        // Validate the Ethereum block hash via OpStack.
        let ethereum_hash = get_validated_ethereum_block_hash_via_opstack(
            sequencer_commitment_opstack.as_ref(),
            env_input_opstack_for_l1_block_call,
            ethereum_chain_id,
            sequencer_commitment_opstack_2.as_ref(),
            env_input_opstack_for_l1_block_call_2,
        );
        // Validate the Linea environment with L1 inclusion (block number only, not hash).
        validate_linea_env_with_l1_inclusion(
            chain_id,
            env_header_to_validate.number,
            env_input_eth_for_l1_inclusion.as_ref().unwrap(),
            ethereum_hash,
        );
    }
    // Always validate the Linea environment (signature check).
    validate_linea_env(chain_id, &block_header_to_validate);
    block_header_to_validate.hash_slow()
}

/// Executes batch multicall for proof data queries.
///
/// This function constructs and executes a batch multicall to retrieve proof data for multiple accounts and assets.
/// It encodes the call data for each query, performs the multicall, decodes the results, and pushes the encoded output to the provided vector.
///
/// # Arguments
/// * `chain_id` - The chain ID for validation.
/// * `account` - Vector of account addresses to query.
/// * `asset` - Vector of token contract addresses.
/// * `target_chain_ids` - Vector of target chain IDs.
/// * `env` - EVM environment for contract calls.
/// * `validate_l1_inclusion` - Whether L1 inclusion is being validated.
/// * `output` - Output vector for proof data results.
///
/// # Panics
/// Panics if:
/// * Multicall execution fails.
/// * Return data decoding fails.
/// * Parameters are mismatched.
pub fn batch_call_get_proof_data<H>(
    chain_id: u64,
    account: Vec<Address>,
    asset: Vec<Address>,
    target_chain_ids: Vec<u64>,
    env: EvmEnv<StateDb, H, Commitment>,
    validate_l1_inclusion: bool,
    output: &mut Vec<Bytes>,
) where
    H: Clone + std::fmt::Debug + EvmFactory,
{
    // Create array of Call3 structs for each proof data check.
    let mut calls = Vec::with_capacity(account.len());
    let batch_params = account
        .iter()
        .zip(asset.iter())
        .zip(target_chain_ids.iter());
    for ((user, market), target_chain_id) in batch_params {
        let user_bytes: [u8; 32] = user.into_word().into();
        let chain_id_bytes: [u8; 32] = U256::from(*target_chain_id).to_be_bytes();

        // Create calldata by concatenating selector, encoded address, and chain ID.
        let mut call_data = Vec::with_capacity(68); // 4 bytes selector + 32 bytes address + 32 bytes chain ID
        call_data.extend_from_slice(&SELECTOR_MALDA_GET_PROOF_DATA);
        call_data.extend_from_slice(&user_bytes);
        call_data.extend_from_slice(&chain_id_bytes);

        calls.push(Call3 {
            target: *market,
            allowFailure: false,
            callData: call_data.into(),
        });
    }

    let multicall_contract = Contract::new(MULTICALL, &env);

    // Make single multicall.
    let multicall = IMulticall3::aggregate3Call { calls };

    let returns = multicall_contract.call_builder(&multicall).call();

    // Create a new iterator for the batch parameters to avoid cloning.
    let batch_params = account
        .iter()
        .zip(asset.iter())
        .zip(target_chain_ids.iter());

    // Zip the batch parameters with returns for parallel iteration.
    batch_params
        .zip(returns.iter())
        .for_each(|(((user, market), target_chain_id), result)| {
            // Decode the returned data as a tuple of (amountIn, amountOut).
            let amounts = <(U256, U256)>::abi_decode(&result.returnData)
                .expect("Failed to decode return data");

            let input = vec![
                SolidityDataType::Address(*user),
                SolidityDataType::Address(*market),
                SolidityDataType::Number(amounts.0), // amountIn
                SolidityDataType::Number(amounts.1), // amountOut
                SolidityDataType::NumberWithShift(U256::from(chain_id), TakeLastXBytes(32)),
                SolidityDataType::NumberWithShift(U256::from(*target_chain_id), TakeLastXBytes(32)),
                SolidityDataType::Bool(validate_l1_inclusion),
            ];

            let (bytes, _hash) = abi::encode_packed(&input);
            output.push(bytes.into());
        });
}

/// Validates Linea environment with L1 inclusion verification.
///
/// This function verifies that a Linea block is properly included in the L1 chain
/// by checking block numbers. This is not sufficient to check L1 inclusion which would require checking the hash. This is currently not possible
/// and is therefore omitted.
///
/// # Arguments
/// * `chain_id` - The Linea chain ID.
/// * `env_block_number` - The block number to validate.
/// * `env_eth_input` - The Ethereum EVM input for L1 validation.
/// * `ethereum_hash` - The Ethereum block hash to validate against.
///
/// # Panics
/// Panics if:
/// * Chain ID is invalid.
/// * Ethereum hash doesn't match.
/// * Block number is higher than the last one posted to L1.
pub fn validate_linea_env_with_l1_inclusion(
    chain_id: u64,
    env_block_number: u64,
    env_eth_input: &EthEvmInput,
    ethereum_hash: B256,
) {
    // Select the correct message service address for the given chain.
    let msg_service_address = match chain_id {
        LINEA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA,
        LINEA_SEPOLIA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let env_eth = env_eth_input.clone().into_env(&ETH_MAINNET_CHAIN_SPEC);

    let eth_hash = env_eth.header().seal();

    // Ensure the Ethereum hash matches.
    assert_eq!(ethereum_hash, eth_hash, "Ethereum hash mismatch linea");

    let current_l2_block_number_call = IL1MessageService::currentL2BlockNumberCall {};

    let contract = Contract::new(msg_service_address, &env_eth);
    let returns = contract.call_builder(&current_l2_block_number_call).call();

    let l2_block_number = returns;

    // Ensure the L2 block number is at least as high as the environment block number.
    assert!(
        l2_block_number >= U256::from(env_block_number),
        "Block number must be lower than or equal to the last one posted to L1"
    );
}

/// Validates a Linea block header by verifying the sequencer signature.
///
/// This function checks that the block is signed by the official Linea sequencer by extracting the signature from the extra data,
/// recovering the signer, and comparing it to the expected sequencer address for the given chain.
///
/// # Arguments
/// * `chain_id` - The chain ID (Linea mainnet or Sepolia).
/// * `block_header_to_validate` - The Linea block header to validate.
///
/// # Panics
/// Panics if:
/// * Chain ID is not a Linea chain.
/// * Block is not signed by the official Linea sequencer.
/// * Signature recovery fails.
/// * Extra data format is invalid.
pub fn validate_linea_env(chain_id: u64, block_header_to_validate: &RlpHeader<Header>) {
    // Extract the extra data and split into prefix and signature.
    let extra_data = block_header_to_validate.inner().extra_data.clone();

    let length = extra_data.len();
    let prefix = extra_data.slice(0..length - 65);
    let signature_bytes = extra_data.slice(length - 65..length);

    let sig = signature_from_bytes(
        &signature_bytes
            .try_into()
            .expect("Failed to convert signature bytes to fixed array"),
    );

    // Remove the signature from the header for sighash calculation.
    let mut header = block_header_to_validate.inner().clone();
    header.extra_data = prefix;

    let sighash: [u8; 32] = header
        .hash_slow()
        .to_vec()
        .try_into()
        .expect("Failed to convert header hash to fixed array");
    let sighash = B256::new(sighash);

    // Recover the sequencer address from the signature and sighash.
    let sequencer =
        recover_signer(sig, sighash).expect("Failed to recover sequencer address from signature");

    // Determine the expected sequencer address for the given chain.
    let expected_sequencer = match chain_id {
        LINEA_CHAIN_ID => LINEA_SEQUENCER,
        LINEA_SEPOLIA_CHAIN_ID => LINEA_SEPOLIA_SEQUENCER,
        _ => panic!("invalid chain id"),
    };

    // Ensure the recovered sequencer matches the expected address.
    if sequencer != expected_sequencer {
        panic!("Block not signed by linea sequencer");
    }
}

/// Validates an OpStack (Optimism/Base) environment through sequencer commitments.
///
/// This function verifies the sequencer commitment for OpStack chains, checks the signature, and ensures the block hash matches.
///
/// # Arguments
/// * `chain_id` - The chain ID (Optimism or Base, mainnet or Sepolia).
/// * `commitment` - The sequencer commitment to verify.
/// * `env_block_hash` - The block hash to validate against.
///
/// # Panics
/// Panics if:
/// * Chain ID is not an OpStack chain.
/// * Commitment verification fails.
/// * Block hash doesn't match commitment.
/// * Sequencer signature is invalid.
/// * Execution payload conversion fails.
pub fn validate_opstack_env(chain_id: u64, commitment: &SequencerCommitment, env_block_hash: B256) {
    // Verify the sequencer commitment for the correct chain and sequencer address.
    match chain_id {
        OPTIMISM_CHAIN_ID => commitment
            .verify(OPTIMISM_SEQUENCER, OPTIMISM_CHAIN_ID)
            .expect("Failed to verify Optimism sequencer commitment"),
        BASE_CHAIN_ID => commitment
            .verify(BASE_SEQUENCER, BASE_CHAIN_ID)
            .expect("Failed to verify Base sequencer commitment"),
        OPTIMISM_SEPOLIA_CHAIN_ID => commitment
            .verify(OPTIMISM_SEPOLIA_SEQUENCER, OPTIMISM_SEPOLIA_CHAIN_ID)
            .expect("Failed to verify Optimism Sepolia sequencer commitment"),
        BASE_SEPOLIA_CHAIN_ID => commitment
            .verify(BASE_SEPOLIA_SEQUENCER, BASE_SEPOLIA_CHAIN_ID)
            .expect("Failed to verify Base Sepolia sequencer commitment"),
        _ => panic!("invalid chain id"),
    }
    // Convert the commitment to an execution payload and check the block hash.
    let payload = ExecutionPayload::try_from(commitment)
        .expect("Failed to convert sequencer commitment to execution payload");
    assert_eq!(payload.block_hash, env_block_hash, "block hash mismatch");
}

/// Retrieves and validates Ethereum L1 block hash through OpStack L2.
///
/// Uses Optimism's L1Block contract to fetch and verify the L1 block hash.
/// This provides a secure way to verify L1 block hashes through L2 commitments.
///
/// # Arguments
/// * `sequencer_commitment_opstack_1` - The Optimism sequencer commitment.
/// * `env_input_opstack_for_l1_block_call_1` - The Optimism EVM input containing environment data.
/// * `chain_id` - The Ethereum chain ID (mainnet or Sepolia).
/// * `_sequencer_commitment_opstack_2` - (Unused) Optional second sequencer commitment.
/// * `_env_input_opstack_for_l1_block_call_2` - (Unused) Optional second Optimism EVM input.
///
/// # Returns
/// * `B256` - The validated Ethereum block hash.
///
/// # Panics
/// Panics if:
/// * OpStack environment validation fails.
/// * L1Block contract call fails.
/// * Chain ID is not an Ethereum chain.
pub fn get_validated_ethereum_block_hash_via_opstack(
    sequencer_commitment_opstack_1: Option<&SequencerCommitment>,
    env_input_opstack_for_l1_block_call_1: Option<EthEvmInput>,
    chain_id: u64,
    _sequencer_commitment_opstack_2: Option<&SequencerCommitment>,
    _env_input_opstack_for_l1_block_call_2: Option<EthEvmInput>,
) -> B256 {
    // Convert the provided EVM input to an environment.
    let env_op = env_input_opstack_for_l1_block_call_1
        .expect("env_input_opstack_for_l1_block_call_1 is None")
        .into_env(&ETH_MAINNET_CHAIN_SPEC);

    // Determine which OpStack chain to use for validation.
    let (verify_via_chain_1, _verify_via_chain_2) = if chain_id == ETHEREUM_CHAIN_ID {
        (OPTIMISM_CHAIN_ID, BASE_CHAIN_ID)
    } else {
        (OPTIMISM_SEPOLIA_CHAIN_ID, BASE_SEPOLIA_CHAIN_ID)
    };

    // Validate the OpStack environment and commitment.
    validate_opstack_env(
        verify_via_chain_1,
        sequencer_commitment_opstack_1.unwrap(),
        env_op.commitment().digest,
    );

    // Query the L1 block hash from the L1Block contract.
    let l1_block = Contract::new(L1_BLOCK_ADDRESS_OPSTACK, &env_op);
    let call = IL1Block::hashCall {};
    let l1_hash_1 = l1_block.call_builder(&call).call();

    // (Optional) Could validate via a second chain, but currently omitted.
    // let env_op_2 = env_input_opstack_for_l1_block_call_2.expect("env_input_opstack_for_l1_block_call_2 is None").into_env();
    // validate_opstack_env(verify_via_chain_2, sequencer_commitment_opstack_2.unwrap(), env_op_2.commitment().digest);
    // let l1_block = Contract::new(L1_BLOCK_ADDRESS_OPSTACK, &env_op_2);
    // let call = IL1Block::hashCall {};
    // let l1_hash_2 = l1_block.call_builder(&call).call().0;
    // assert_eq!(l1_hash_1, l1_hash_2, "L1 hash 1 and 2 mismatch");

    l1_hash_1
}

/// Validates block chain length and hash linking for reorg protection.
///
/// Ensures sufficient block confirmations and proper hash linking between blocks
/// to prevent reorganization attacks. Checks that the chain is long enough, that each block is hash-linked to its parent,
/// and that the final hash matches the expected current hash.
///
/// # Arguments
/// * `chain_id` - The chain ID to determine reorg protection depth.
/// * `historical_hash` - The hash of the historical block.
/// * `linking_blocks` - Vector of blocks linking historical to current.
/// * `current_hash` - The expected current block hash.
///
/// # Panics
/// Panics if:
/// * Chain length is less than required reorg protection depth.
/// * Blocks are not properly hash-linked.
/// * Final hash doesn't match current hash.
/// * Chain ID is invalid or unsupported.
pub fn validate_chain_length(
    chain_id: u64,
    historical_hash: B256,
    linking_blocks: &Vec<RlpHeader<Header>>,
    current_hash: B256,
) {
    // Determine the required reorg protection depth for the given chain.
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
    let chain_length = linking_blocks.len() as u64;
    // Ensure the chain is long enough for reorg protection.
    assert!(
        chain_length >= reorg_protection_depth,
        "chain length is less than reorg protection"
    );
    let mut previous_hash = historical_hash;
    // Check that each block is hash-linked to its parent.
    for header in linking_blocks.iter() {
        let parent_hash = header.parent_hash;
        assert_eq!(parent_hash, previous_hash, "blocks not hashlinked");
        previous_hash = header.hash_slow();
    }
    // Ensure the final hash matches the expected current hash.
    assert_eq!(
        previous_hash, current_hash,
        "last hash doesnt correspond to verified hash"
    );
}
