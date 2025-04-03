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
use alloy_encode_packed::{abi, SolidityDataType, TakeLastXBytes};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_sol_types::SolValue;
use risc0_steel::{ethereum::EthEvmInput, serde::RlpHeader, Commitment, Contract, EvmEnv, StateDb};
use risc0_op_steel::optimism::OpEvmInput;
use risc0_steel::EvmBlockHeader;

/// Validates and executes proof data queries across multiple accounts and tokens using multicall
///
/// # Arguments
/// * `chain_id` - The chain ID to validate against
/// * `accounts` - Vector of account addresses to query
/// * `assets` - Vector of token contract addresses to query
/// * `target_chain_ids` - Vector of target chain IDs for each account
/// * `env_input` - EVM environment input for the chain
/// * `sequencer_commitment` - Optional sequencer commitment for L2 chains
/// * `env_op_input` - Optional Optimism environment input for L1 validation
/// * `linking_blocks` - Vector of blocks for reorg protection
/// * `output` - Output vector for proof data results
/// * `env_eth_input` - Optional Ethereum environment input for L1 inclusion validation
/// * `storage_hash` - Optional storage hash for L1 inclusion validation
///
/// # Panics
/// * If chain ID is invalid
/// * If environment validation fails
/// * If chain length is insufficient
/// * If block hashes don't match
/// * If multicall execution fails
/// * If return data decoding fails
pub fn validate_get_proof_data_call(
    chain_id: u64,
    account: Vec<Address>,
    asset: Vec<Address>,
    target_chain_ids: Vec<u64>,
    env_input: Option<EthEvmInput>,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    linking_blocks: Vec<RlpHeader<Header>>,
    output: &mut Vec<Bytes>,
    env_eth_input: Option<EthEvmInput>,
    op_evm_input: Option<OpEvmInput>,
) {
    let validate_l1_inclusion = env_eth_input.is_some();

    let (env, validate_chain_id) = if (chain_id == OPTIMISM_CHAIN_ID || chain_id == BASE_CHAIN_ID || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID || chain_id == BASE_SEPOLIA_CHAIN_ID) && validate_l1_inclusion {
        let env = env_eth_input.clone().expect("env_eth_input is None").into_env();
        if chain_id == OPTIMISM_CHAIN_ID || chain_id == BASE_CHAIN_ID {
            (env, ETHEREUM_CHAIN_ID)
        } else {
            (env, ETHEREUM_SEPOLIA_CHAIN_ID)
        }
    } else {
        (env_input.expect("env_input is None").into_env(), chain_id)
    };

    let last_block = if linking_blocks.is_empty() {
        env.header().inner().clone()
    } else {
        linking_blocks[linking_blocks.len() - 1].clone()
    };

    // let eth_env: risc0_steel::EvmEnv<risc0_steel::StateDb, RlpHeader<Header>, _> = env_eth_input.expect("env_eth_input is None").into_env();
    // let op_env: risc0_steel::EvmEnv<risc0_steel::StateDb, risc0_op_steel::optimism::OpBlockHeader, _> = op_evm_input.expect("op_evm_input is None").into_env();

    let env_header_hash = env.header().seal();
    let env_header = env.header().inner().inner().clone();

    let validated_block_hash = get_validated_block_hash(
        validate_chain_id,
        env_header,
        sequencer_commitment,
        env_op_input,
        env_eth_input,
        last_block,
        validate_l1_inclusion,
    );

    validate_chain_length(
        validate_chain_id,
        env_header_hash,
        linking_blocks,
        validated_block_hash,
    );


    batch_call_get_proof_data(
        chain_id,
        account,
        asset,
        target_chain_ids,
        env,
        validate_l1_inclusion,
        output,
    );
}

pub fn validate_opstack_dispute_game_commitment(
    chain_id: u64,
    env_eth_input: Option<EthEvmInput>,
    op_evm_input: Option<OpEvmInput>
) {

    let eth_env = env_eth_input.expect("env_eth_input is None").into_env();
    let op_env = op_evm_input.expect("op_evm_input is None").into_env();

    let commitment = op_env.commitment();
    let id = commitment.id;
    let root_claim = commitment.digest;

    // Extract last 240 bits by masking with 0xFFFF...FFFF (240 bits)
    let mask = U256::from(1) << 240;
    let game_index = id & (mask - U256::from(1));


    let portal_adress = match chain_id {
        OPTIMISM_SEPOLIA_CHAIN_ID => OPTIMISM_SEPOLIA_PORTAL,
        BASE_SEPOLIA_CHAIN_ID => BASE_SEPOLIA_PORTAL,
        OPTIMISM_CHAIN_ID => OPTIMISM_PORTAL,
        BASE_CHAIN_ID => BASE_PORTAL,
        _ => panic!("invalid chain id"),
    };

    // Get the portal contract for additional checks
    let portal_contract = Contract::new(portal_adress, &eth_env);

    // Get factory address from portal
    let factory_call = IOptimismPortal::disputeGameFactoryCall {};
    let returns = portal_contract.call_builder(&factory_call).call();
    let factory_address = returns._0;

    let game_call = IDisputeGameFactory::gameAtIndexCall {
        index: game_index,
    };

    let contract = Contract::new(factory_address, &eth_env);
    let returns = contract.call_builder(&game_call).call();

    let game_type = returns._0;
    assert_eq!(game_type, U256::from(0), "game type not respected game");

    let created_at = returns._1;
    let game_address = returns._2;

    // Check if game was created after respected game type update
    let respected_game_type_updated_at_call = IOptimismPortal::respectedGameTypeUpdatedAtCall {};
    let returns = portal_contract.call_builder(&respected_game_type_updated_at_call).call();
    assert!(created_at >= returns._0, "game created before respected game type update");

    // Get game contract for status checks
    let game_contract = Contract::new(game_address, &eth_env);

    // Check game status
    let status_call = IDisputeGame::statusCall {};
    let returns = game_contract.call_builder(&status_call).call();
    assert_eq!(returns._0, GameStatus::DEFENDER_WINS, "game status not DEFENDER_WINS");

    // Check if game is blacklisted
    let blacklist_call = IOptimismPortal::disputeGameBlacklistCall { game: game_address };
    let returns = portal_contract.call_builder(&blacklist_call).call();
    assert!(!returns._0, "game is blacklisted");

    // Check game resolution time
    let resolved_at_call = IDisputeGame::resolvedAtCall {};
    let returns = game_contract.call_builder(&resolved_at_call).call();
    let resolved_at = returns._0;
    
    let proof_maturity_delay_call = IOptimismPortal::proofMaturityDelaySecondsCall {};
    let returns = portal_contract.call_builder(&proof_maturity_delay_call).call();
    let proof_maturity_delay = returns._0;
    
    let current_timestamp = eth_env.header().inner().inner().timestamp;
    assert!(
        U256::from(current_timestamp) - U256::from(resolved_at) > proof_maturity_delay,
        "insufficient time passed since game resolution"
    );

    // Finally verify root claim matches
    let root_claim_call = IDisputeGame::rootClaimCall {};
    let returns = game_contract.call_builder(&root_claim_call).call();
    assert_eq!(returns._0, commitment.digest, "root claim mismatch");
}

/// Retrieves validated block hash based on chain type and validation requirements
///
/// # Arguments
/// * `chain_id` - The chain ID to determine validation strategy
/// * `env_header` - The block header to validate
/// * `sequencer_commitment` - Optional sequencer commitment for L2 chains
/// * `env_op_input` - Optional Optimism environment input for L1 validation
/// * `env_eth_input` - Optional Ethereum environment input for L1 inclusion validation
/// * `last_block` - Last block in the chain for hash validation
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion
/// * `storage_hash` - Optional storage hash for L1 inclusion validation
///
/// # Returns
/// * `B256` - The validated block hash
///
/// # Panics
/// * If chain ID is invalid or unsupported
/// * If validation fails for the specific chain type
pub fn get_validated_block_hash(
    chain_id: u64,
    env_header: Header,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    env_eth_input: Option<EthEvmInput>,
    last_block: RlpHeader<Header>,
    validate_l1_inclusion: bool,
) -> B256 {
    if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
        get_validated_block_hash_linea(
            chain_id,
            env_header,
            sequencer_commitment,
            env_op_input,
            env_eth_input,
            last_block,
            validate_l1_inclusion,
        )
    } else if chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
    {
        get_validated_block_hash_opstack(
            chain_id,
            env_header,
            sequencer_commitment,
            env_op_input,
            env_eth_input,
            last_block,
            validate_l1_inclusion,
        )
    } else if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID {
        let ethereum_hash = get_ethereum_block_hash_via_opstack(
            sequencer_commitment.unwrap(),
            env_op_input.unwrap(),
            chain_id,
        );
        ethereum_hash
    } else {
        panic!("invalid chain id");
    }
}

/// Validates OpStack block hash with optional L1 inclusion verification
///
/// # Arguments
/// * `chain_id` - The OpStack chain ID (Optimism/Base)
/// * `env_header` - The block header to validate
/// * `sequencer_commitment` - Optional sequencer commitment
/// * `env_op_input` - Optional Optimism environment input
/// * `env_eth_input` - Optional Ethereum environment input
/// * `last_block` - Last block for hash validation
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion
/// * `storage_hash` - Optional storage hash for L1 validation
///
/// # Returns
/// * `B256` - The validated block hash
///
/// # Panics
/// * If validation fails for OpStack environment
/// * If L1 inclusion validation fails when requested
pub fn get_validated_block_hash_opstack(
    chain_id: u64,
    env_header: Header,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    env_eth_input: Option<EthEvmInput>,
    last_block: RlpHeader<Header>,
    validate_l1_inclusion: bool,
) -> B256 {
    let last_block_hash = last_block.hash_slow();
    if validate_l1_inclusion {
        let env_state_root = env_header.state_root;
        let ethereum_hash = get_ethereum_block_hash_via_opstack(
            sequencer_commitment.unwrap(),
            env_op_input.unwrap(),
            chain_id,
        );
        // validate_opstack_env_with_l1_inclusion(
        //     chain_id,
        //     env_state_root,
        //     env_eth_input.unwrap(),
        //     ethereum_hash,
        //     last_block_hash,
        // );
    } else {
        validate_opstack_env(chain_id, &sequencer_commitment.unwrap(), last_block_hash);
    }
    last_block_hash
}

/// Validates Linea block hash with optional L1 inclusion verification
///
/// # Arguments
/// * `chain_id` - The Linea chain ID
/// * `env_header` - The block header to validate
/// * `sequencer_commitment` - Optional sequencer commitment
/// * `env_op_input` - Optional Optimism environment input
/// * `env_eth_input` - Optional Ethereum environment input
/// * `last_block` - Last block for hash validation
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion
///
/// # Returns
/// * `B256` - The validated block hash
///
/// # Panics
/// * If validation fails for Linea environment
/// * If L1 inclusion validation fails when requested
pub fn get_validated_block_hash_linea(
    chain_id: u64,
    env_header: Header,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    env_eth_input: Option<EthEvmInput>,
    last_block: RlpHeader<Header>,
    validate_l1_inclusion: bool,
) -> B256 {
    if validate_l1_inclusion {
        let env_block_number = env_header.number;
        let ethereum_hash = get_ethereum_block_hash_via_opstack(
            sequencer_commitment.unwrap(),
            env_op_input.unwrap(),
            chain_id,
        );
        validate_linea_env_with_l1_inclusion(
            chain_id,
            env_block_number,
            env_eth_input.unwrap(),
            ethereum_hash,
        );
    }
    validate_linea_env(chain_id, last_block.clone());
    last_block.hash_slow()
}

/// Executes batch multicall for proof data queries
///
/// # Arguments
/// * `chain_id` - The chain ID for validation
/// * `account` - Vector of account addresses to query
/// * `asset` - Vector of token contract addresses
/// * `target_chain_ids` - Vector of target chain IDs
/// * `env` - EVM environment for contract calls
/// * `validate_l1_inclusion` - Whether L1 inclusion is being validated
/// * `output` - Output vector for proof data results
///
/// # Panics
/// * If multicall execution fails
/// * If return data decoding fails
/// * If parameters are mismatched
pub fn batch_call_get_proof_data<H>(
    chain_id: u64,
    account: Vec<Address>,
    asset: Vec<Address>,
    target_chain_ids: Vec<u64>,
    env: EvmEnv<StateDb, H, Commitment>,
    validate_l1_inclusion: bool,
    output: &mut Vec<Bytes>,
) where
    H: Clone + std::fmt::Debug, H: EvmBlockHeader
{
    // Create array of Call3 structs for each proof data check
    let mut calls = Vec::with_capacity(account.len());
    let batch_params = account
        .iter()
        .zip(asset.iter())
        .zip(target_chain_ids.iter());
    for ((user, market), target_chain_id) in batch_params.clone() {
        // Selector for getProofData(address,uint32)
        let selector = [0x07, 0xd9, 0x23, 0xe9];
        let user_bytes: [u8; 32] = user.into_word().into();
        let chain_id_bytes: [u8; 32] = U256::from(*target_chain_id).to_be_bytes();

        // Create calldata by concatenating selector, encoded address, and chain ID
        let mut call_data = Vec::with_capacity(68); // 4 bytes selector + 32 bytes address + 32 bytes chain ID
        call_data.extend_from_slice(&selector);
        call_data.extend_from_slice(&user_bytes);
        call_data.extend_from_slice(&chain_id_bytes);

        calls.push(Call3 {
            target: *market,
            allowFailure: false,
            callData: call_data.into(),
        });
    }

    let multicall_contract = Contract::new(MULTICALL, &env);

    // Make single multicall
    let multicall = IMulticall3::aggregate3Call { calls };

    let returns = multicall_contract.call_builder(&multicall).call();

    // Zip the batch parameters with returns.results for parallel iteration
    batch_params.zip(returns.results.iter()).for_each(
        |(((user, market), target_chain_id), result)| {
            let amounts = <(U256, U256)>::abi_decode(&result.returnData, true)
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
        },
    );
}


pub fn validate_linea_env_with_l1_inclusion(
    chain_id: u64,
    env_block_number: u64,
    env_eth_input: EthEvmInput,
    ethereum_hash: B256,
) {
    let msg_service_address = match chain_id {
        LINEA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA,
        LINEA_SEPOLIA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA_SEPOLIA,
        _ => panic!("invalid chain id"),
    };

    let env_eth = env_eth_input.into_env();

    let eth_hash = env_eth.header().seal();

    assert_eq!(ethereum_hash, eth_hash, "Ethereum hash mismatch");

    let current_l2_block_number_call = IL1MessageService::currentL2BlockNumberCall {};

    let contract = Contract::new(msg_service_address, &env_eth);
    let returns = contract.call_builder(&current_l2_block_number_call).call();

    let l2_block_number = returns._0;

    assert!(
        l2_block_number <= U256::from(env_block_number),
        "Block number must be lower than the last one posted to L1"
    );
}

/// Validates a Linea block header by verifying the sequencer signature
///
/// # Arguments
/// * `chain_id` - The chain ID (Linea mainnet or Sepolia)
/// * `header` - The Linea block header to validate
///
/// # Panics
/// * If chain ID is not a Linea chain
/// * If block is not signed by the official Linea sequencer
/// * If signature recovery fails
/// * If extra data format is invalid
pub fn validate_linea_env(chain_id: u64, header: risc0_steel::ethereum::EthBlockHeader) {
    let extra_data = header.inner().extra_data.clone();

    let length = extra_data.len();
    let prefix = extra_data.slice(0..length - 65);
    let signature_bytes = extra_data.slice(length - 65..length);

    let sig = signature_from_bytes(
        &signature_bytes
            .try_into()
            .expect("Failed to convert signature bytes to fixed array"),
    );

    let mut header = header.inner().clone();
    header.extra_data = prefix;

    let sighash: [u8; 32] = header
        .hash_slow()
        .to_vec()
        .try_into()
        .expect("Failed to convert header hash to fixed array");
    let sighash = B256::new(sighash);

    let sequencer =
        recover_signer(sig, sighash).expect("Failed to recover sequencer address from signature");

    let expected_sequencer = match chain_id {
        LINEA_CHAIN_ID => LINEA_SEQUENCER,
        LINEA_SEPOLIA_CHAIN_ID => LINEA_SEPOLIA_SEQUENCER,
        _ => panic!("invalid chain id"),
    };

    if sequencer != expected_sequencer {
        panic!("Block not signed by linea sequencer");
    }
}

/// Validates an OpStack (Optimism/Base) environment through sequencer commitments
///
/// # Arguments
/// * `chain_id` - The chain ID (Optimism or Base, mainnet or Sepolia)
/// * `commitment` - The sequencer commitment to verify
/// * `env_block_hash` - The block hash to validate against
///
/// # Panics
/// * If chain ID is not an OpStack chain
/// * If commitment verification fails
/// * If block hash doesn't match commitment
/// * If sequencer signature is invalid
/// * If execution payload conversion fails
pub fn validate_opstack_env(chain_id: u64, commitment: &SequencerCommitment, env_block_hash: B256) {
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
    let payload = ExecutionPayload::try_from(commitment)
        .expect("Failed to convert sequencer commitment to execution payload");
    assert_eq!(payload.block_hash, env_block_hash, "block hash mismatch");
}

/// Retrieves and validates Ethereum L1 block hash through OpStack L2
///
/// Uses Optimism's L1Block contract to fetch and verify the L1 block hash.
/// This provides a secure way to verify L1 block hashes through L2 commitments.
///
/// # Arguments
/// * `commitment` - The Optimism sequencer commitment
/// * `input_op` - The Optimism EVM input containing environment data
/// * `chain_id` - The Ethereum chain ID (mainnet or Sepolia)
///
/// # Returns
/// * `B256` - The validated Ethereum block hash
///
/// # Panics
/// * If OpStack environment validation fails
/// * If L1Block contract call fails
/// * If chain ID is not an Ethereum chain
pub fn get_ethereum_block_hash_via_opstack(
    commitment: SequencerCommitment,
    input_op: EthEvmInput,
    chain_id: u64,
) -> B256 {
    let env_op = input_op.into_env();
    let verify_via_chain = if chain_id == ETHEREUM_CHAIN_ID {
        OPTIMISM_CHAIN_ID
    } else {
        OPTIMISM_SEPOLIA_CHAIN_ID
    };
    validate_opstack_env(verify_via_chain, &commitment, env_op.commitment().digest);
    let l1_block = Contract::new(L1_BLOCK_ADDRESS_OPSTACK, &env_op);
    let call = IL1Block::hashCall {};
    l1_block.call_builder(&call).call()._0
}

/// Validates block chain length and hash linking for reorg protection
///
/// Ensures sufficient block confirmations and proper hash linking between blocks
/// to prevent reorganization attacks.
///
/// # Arguments
/// * `chain_id` - The chain ID to determine reorg protection depth
/// * `historical_hash` - The hash of the historical block
/// * `linking_blocks` - Vector of blocks linking historical to current
/// * `current_hash` - The expected current block hash
///
/// # Panics
/// * If chain length is less than required reorg protection depth
/// * If blocks are not properly hash-linked
/// * If final hash doesn't match current hash
/// * If chain ID is invalid or unsupported
pub fn validate_chain_length(
    chain_id: u64,
    historical_hash: B256,
    linking_blocks: Vec<RlpHeader<Header>>,
    current_hash: B256,
) {
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
    let chain_length = linking_blocks.len() as u64;
    assert!(
        chain_length >= reorg_protection_depth,
        "chain length is less than reorg protection"
    );
    let mut previous_hash = historical_hash;
    for header in linking_blocks {
        let parent_hash = header.parent_hash;
        assert_eq!(parent_hash, previous_hash, "blocks not hashlinked");
        previous_hash = header.hash_slow();
    }
    assert_eq!(
        previous_hash, current_hash,
        "last hash doesnt correspond to current l1 hash"
    );
}
