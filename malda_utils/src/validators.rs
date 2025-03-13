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
use alloy_primitives::{Address, Bytes, B256, U256, address};
use alloy_sol_types::SolValue;
use risc0_steel::{ethereum::EthEvmInput, serde::RlpHeader, Contract};

/// Validates and executes proof data queries across multiple accounts and tokens
///
/// # Arguments
/// * `chain_id` - The chain ID to validate against
/// * `accounts` - Vector of account addresses to query
/// * `assets` - Vector of token contract addresses to query
/// * `target_chain_ids` - Vector of target chain IDs for each account
/// * `env_input` - EVM environment input for the chain
/// * `sequencer_commitment` - Optional sequencer commitment for L2 chains
/// * `op_env_input` - Optional Optimism environment input for L1 validation
/// * `linking_blocks` - Vector of blocks for reorg protection
/// * `output` - Output vector for proof data results
///
/// # Panics
/// * If chain ID is invalid
/// * If environment validation fails
/// * If chain length is insufficient
/// * If block hashes don't match
pub fn validate_get_proof_data_call(
    chain_id: u64,
    account: Vec<Address>,
    asset: Vec<Address>,
    target_chain_ids: Vec<u64>,
    env_input: EthEvmInput,
    sequencer_commitment: Option<SequencerCommitment>,
    env_op_input: Option<EthEvmInput>,
    linking_blocks: Vec<RlpHeader<Header>>,
    output: &mut Vec<Bytes>,
    env_eth_input: Option<EthEvmInput>,
) {
    let validate_l1_inclusion = env_eth_input.is_some();
    let env = env_input.into_env();

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

    let last_block = if linking_blocks.is_empty() {
        env.header().inner().clone()
    } else {
        linking_blocks[linking_blocks.len() - 1].clone()
    };

    let validated_block_hash = if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
        validate_linea_env(chain_id, last_block.clone());
        last_block.hash_slow()
    } else if chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
    {
        if validate_l1_inclusion {
            let last_block_hash = last_block.hash_slow();
            let env_state_root = env.header().inner().inner().clone().state_root;
            let ethereum_hash = get_ethereum_block_hash_via_opstack(
                sequencer_commitment.unwrap(),
                env_op_input.unwrap(),
                chain_id,
            );
            let env_eth = env_eth_input.unwrap().into_env();
            let eth_hash = env_eth.header().seal();
            let env_eth_timestamp = env_eth.header().inner().inner().timestamp;

            assert_eq!(ethereum_hash, eth_hash, "last block hash mismatch");

            let game_count_call = IDisputeGameFactory::gameCountCall {};

            let contract = Contract::new(DISPUTE_GAME_FACTORY_OPTIMISM_SEPOLIA, &env_eth);
            let returns = contract
                .call_builder(&game_count_call)
                // .gas_price(U256::from(gas_price))
                // .from(Address::ZERO)
                .call();
        
            let latest_game_index = returns._0 - U256::from(1);
        
            let game_call = IDisputeGameFactory::gameAtIndexCall { index: latest_game_index };
        
            let contract = Contract::new(DISPUTE_GAME_FACTORY_OPTIMISM_SEPOLIA, &env_eth);
            let returns = contract
                .call_builder(&game_call)
                .call();
        
            let game_type = returns._0;
            let created_at = returns._1;
            let game_address = returns._2;
        
            let root_claim_call = IDisputeGame::rootClaimCall {};
        
            let contract = Contract::new(game_address, &env_eth);
            let returns = contract
                .call_builder(&root_claim_call)
                .call();
        
            let root_claim = returns._0;
        
            let l2_block_number_challenged_call = IDisputeGame::l2BlockNumberChallengedCall {};
        
            let contract = Contract::new(game_address, &env_eth);
            let returns = contract
                .call_builder(&l2_block_number_challenged_call)
                .call();

            let l2_block_number_challenged = returns._0;

            // assert_eq!(root_claim, env_state_root, "root claim mismatch");
            assert_eq!(l2_block_number_challenged, false, "This L2 block has been challenged");
            assert!(U256::from(env_eth_timestamp) > created_at + U256::from(300), "Not enough time passed to challenge the claim");



            last_block_hash
        } else {
            let last_block_hash = last_block.hash_slow();
            validate_opstack_env(chain_id, &sequencer_commitment.unwrap(), last_block_hash);
            last_block_hash
        }
    } else if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID {
        let ethereum_hash = get_ethereum_block_hash_via_opstack(
            sequencer_commitment.unwrap(),
            env_op_input.unwrap(),
            chain_id,
        );
        ethereum_hash
    } else {
        panic!("invalid chain id");
    };

    validate_chain_length(
        chain_id,
        env.header().seal(),
        linking_blocks,
        validated_block_hash,
    );

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
///
/// # Arguments
/// * `commitment` - The Optimism sequencer commitment
/// * `input_op` - The Optimism EVM input containing environment data
///
/// # Returns
/// * `B256` - The validated Ethereum block hash
///
/// # Panics
/// * If OpStack environment validation fails
/// * If L1Block contract call fails
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
    let l1_block = Contract::new(L1_BLOCK_ADDRESS_OPTIMISM, &env_op);
    let call = IL1Block::hashCall {};
    l1_block.call_builder(&call).call()._0
}

/// Validates block chain length and hash linking for reorg protection
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
/// * If chain ID is invalid
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
