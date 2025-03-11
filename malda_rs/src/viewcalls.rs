//! Ethereum view call utilities for cross-chain view call proof.
//!
//! This module provides functionality to:
//! - Execute and prove user proof data queries across multiple EVM chains
//! - Handle sequencer commitments for L2 chains (Optimism, Base)
//! - Process L1 block verification for L2 chains
//! - Manage linking blocks for reorg protection
//! - Support parallel processing of multi-chain proof data queries
//!
//! The module supports both mainnet and testnet (Sepolia) environments for:
//! - Ethereum (L1)
//! - Optimism
//! - Base
//! - Linea

use core::panic;

use crate::constants::*;
use crate::elfs_ids::*;
use crate::types::{Call3, IMulticall3};
use crate::types::{ExecutionPayload, IL1Block, SequencerCommitment};

use risc0_steel::{
    ethereum::EthEvmEnv, host::BlockNumberOrTag, serde::RlpHeader, Contract, EvmInput,
};
use risc0_zkvm::{
    default_executor, default_prover, ExecutorEnv, ProveInfo, ProverOpts, SessionInfo,
};

use alloy::primitives::Address;
use alloy_consensus::Header;

use anyhow::{Error, Result};
use bonsai_sdk;
use futures::future::join_all;
use tokio;
use url::Url;

use std::time::Duration;

use bonsai_sdk::blocking::Client;
use risc0_zkvm::{Receipt, SessionStats};

fn run_bonsai(input_data: Vec<u8>) -> Result<ProveInfo, anyhow::Error> {
    let start_total = std::time::Instant::now();

    let start = std::time::Instant::now();
    let client = Client::from_env(risc0_zkvm::VERSION)?;
    println!("Client creation time: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    // Convert Vec<u32> to Vec<u8> before encoding
    let id_bytes: Vec<u8> = GET_PROOF_DATA_ID
        .iter()
        .flat_map(|&x| x.to_le_bytes())
        .collect();
    let _image_id_hex = hex::encode(id_bytes);
    let image_id_hex: String =
        "2df73778d83db383511ada84662a60342c5106a66cfb3ecb09e2b45a97887863".to_string();
    println!("Image ID: {}", image_id_hex);
    println!("Image read time: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    let input_id = client.upload_input(input_data)?;
    println!("Input upload time: {:?}", start.elapsed());

    let assumptions: Vec<String> = vec![];
    let execute_only = false;

    let start = std::time::Instant::now();
    let session = client.create_session(image_id_hex, input_id, assumptions, execute_only)?;

    tracing::debug!("Bonsai proving SessionID: {}", session.uuid);
    println!("Bonsai proving SessionID: {}", session.uuid);

    let polling_interval = Duration::from_millis(100);

    let succinct_stats = loop {
        let res = session.status(&client)?;
        if res.status == "RUNNING" {
            std::thread::sleep(polling_interval);
            continue;
        }
        if res.status == "SUCCEEDED" {
            println!("Proving time: {:?}", start.elapsed());

            let stats = res
                .stats
                .expect("Missing stats object on Bonsai status res");
            tracing::debug!(
                "Bonsai usage: cycles: {} total_cycles: {}",
                stats.cycles,
                stats.total_cycles
            );

            break SessionStats {
                segments: stats.segments,
                total_cycles: stats.total_cycles,
                user_cycles: stats.cycles,
                paging_cycles: 0,
                reserved_cycles: 0,
            };
        } else {
            return Err(anyhow::Error::msg(format!(
                "Bonsai prover workflow [{}] exited: {} err: {}",
                session.uuid,
                res.status,
                res.error_msg
                    .unwrap_or("Bonsai workflow missing error_msg".into())
            )));
        }
    };

    let start = std::time::Instant::now();
    let snark_session = client.create_snark(session.uuid)?;
    println!("Snark session creation time: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    let snark_receipt_url = loop {
        let res = snark_session.status(&client)?;
        match res.status.as_str() {
            "RUNNING" => {
                std::thread::sleep(polling_interval);
                continue;
            }
            "SUCCEEDED" => {
                break res.output.ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "Bonsai prover workflow [{}] reported success, but provided no receipt",
                        snark_session.uuid
                    ))
                })?;
            }
            _ => {
                return Err(anyhow::Error::msg(format!(
                    "Bonsai prover workflow [{}] exited: {} err: {}",
                    snark_session.uuid,
                    res.status,
                    res.error_msg
                        .unwrap_or("Bonsai workflow missing error_msg".into())
                )));
            }
        }
    };
    println!("Snark proving time: {:?}", start.elapsed());

    let start = std::time::Instant::now();
    let receipt_buf = client.download(&snark_receipt_url)?;
    let groth16_receipt: Receipt = bincode::deserialize(&receipt_buf)?;
    println!("Receipt download time: {:?}", start.elapsed());

    println!("Total time (groth16): {:?}", start_total.elapsed());

    Ok(ProveInfo {
        receipt: groth16_receipt,
        stats: succinct_stats,
    })
}

/// Executes proof data queries across multiple chains in parallel
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain
/// * `markets` - Vector of market contract address vectors, one per chain
/// * `target_chain_ids` - Vector of target chain IDs to query
/// * `chain_ids` - Vector of chain IDs to query
///
/// # Returns
/// * `Result<SessionInfo, Error>` - Session info from the ZKVM execution
///
/// # Errors
/// Returns an error if:
/// - Array lengths don't match
/// - RPC calls fail
/// - ZKVM execution fails
pub async fn get_proof_data_exec(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_id: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
) -> Result<SessionInfo, Error> {
    // Verify outer arrays have same length
    assert_eq!(
        users.len(),
        markets.len(),
        "Users and markets array lengths must match"
    );
    assert_eq!(
        users.len(),
        chain_ids.len(),
        "Users and chain_ids array lengths must match"
    );

    let futures: Vec<_> = (0..chain_ids.len())
        .map(|i| {
            let users = users[i].clone();
            let markets = markets[i].clone();
            let target_chain_id = target_chain_id[i].clone();
            let chain_id = chain_ids[i];
            tokio::spawn(async move {
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id).await
            })
        })
        .collect();

    let results = join_all(futures).await;
    let all_inputs = results
        .into_iter()
        .map(|r| r.expect("Failed to join parallel execution task"))
        .flatten()
        .collect::<Vec<u8>>();

    let env = ExecutorEnv::builder()
        .write(&(chain_ids.len() as u64))
        .expect("Failed to write chain count to executor environment")
        .write_slice(&all_inputs)
        .build()
        .expect("Failed to build executor environment");

    Ok(default_executor()
        .execute(env, GET_PROOF_DATA_ELF)
        .expect("Failed to execute ZKVM"))
}

/// Creates the executor environment with proof data from multiple chains
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain
/// * `markets` - Vector of market contract address vectors, one per chain
/// * `target_chain_ids` - Vector of target chain IDs to query
/// * `chain_ids` - Vector of chain IDs to query
///
/// # Returns
/// * `ExecutorEnv` - Environment configured with proof data inputs
async fn get_proof_data_env(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
) -> ExecutorEnv<'static> {
    // Verify outer arrays have same length
    assert_eq!(users.len(), markets.len());
    assert_eq!(users.len(), chain_ids.len());

    // Create futures using tokio::spawn for true parallelism
    let futures: Vec<_> = (0..chain_ids.len())
        .map(|i| {
            let users = users[i].clone();
            let markets = markets[i].clone();
            let chain_id = chain_ids[i];
            let target_chain_id = target_chain_ids[i].clone();
            tokio::spawn(async move {
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id).await
            })
        })
        .collect();

    // Execute all futures in parallel and collect results
    let results = join_all(futures).await;
    let all_inputs = results
        .into_iter()
        .filter_map(|r| r.ok()) // Handle any JoinError
        .flat_map(|input| input)
        .collect::<Vec<_>>();

    // Create environment with inputs
    ExecutorEnv::builder()
        .write(&(chain_ids.len() as u64))
        .unwrap()
        .write_slice(&all_inputs)
        .build()
        .unwrap()
}

async fn get_proof_data_input(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
) -> Vec<u8> {
    // Verify outer arrays have same length
    assert_eq!(users.len(), markets.len());
    assert_eq!(users.len(), chain_ids.len());

    // Create futures using tokio::spawn for true parallelism
    let futures: Vec<_> = (0..chain_ids.len())
        .map(|i| {
            let users = users[i].clone();
            let markets = markets[i].clone();
            let chain_id = chain_ids[i];
            let target_chain_id = target_chain_ids[i].clone();
            tokio::spawn(async move {
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id).await
            })
        })
        .collect();

    // Execute all futures in parallel and collect results
    let results = join_all(futures).await;
    let all_inputs = results
        .into_iter()
        .filter_map(|r| r.ok()) // Handle any JoinError
        .flat_map(|input| input)
        .collect::<Vec<_>>();

    // Create input with chain count
    let input: Vec<u8> = bytemuck::pod_collect_to_vec(
        &risc0_zkvm::serde::to_vec(&(chain_ids.len() as u64)).unwrap(),
    );

    // Concatenate the chain count input with all the chain-specific inputs
    [input, all_inputs].concat()
}

/// Generates ZK proofs for proof data queries across multiple chains
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain
/// * `markets` - Vector of market contract address vectors, one per chain
/// * `target_chain_ids` - Vector of target chain IDs to query
/// * `chain_ids` - Vector of chain IDs to query
///
/// # Returns
/// * `Result<ProveInfo, Error>` - Proof information from the ZKVM
///
/// # Errors
/// Returns an error if:
/// - Array lengths don't match
/// - RPC calls fail
/// - Proof generation fails
pub async fn get_proof_data_prove(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
) -> Result<ProveInfo, Error> {
    // Move all the work including env creation into the blocking task
    let prove_info = tokio::task::spawn_blocking(move || {
        // Create a new runtime for async operations within the blocking task
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Execute all async operations in the new runtime
        let start_time = std::time::Instant::now();
        let env = rt.block_on(get_proof_data_env(
            users,
            markets,
            target_chain_ids,
            chain_ids,
        ));
        let duration = start_time.elapsed();
        println!("Env creation time: {:?}", duration);

        let start_time = std::time::Instant::now();
        let proof =
            default_prover().prove_with_opts(env, GET_PROOF_DATA_ELF, &ProverOpts::groth16());
        let duration = start_time.elapsed();
        println!("Bonsai proof time: {:?}", duration);
        proof
    })
    .await?;

    prove_info
}

pub async fn get_proof_data_prove_sdk(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
) -> Result<ProveInfo, Error> {
    // Move all the work including env creation into the blocking task
    let prove_info = tokio::task::spawn_blocking(move || {
        // Create a new runtime for async operations within the blocking task
        let rt = tokio::runtime::Runtime::new().unwrap();

        // Execute all async operations in the new runtime
        let start_time = std::time::Instant::now();
        let input = rt.block_on(get_proof_data_input(
            users,
            markets,
            target_chain_ids,
            chain_ids,
        ));
        let duration = start_time.elapsed();
        println!("Env creation time: {:?}", duration);

        let start_time = std::time::Instant::now();
        let proof = run_bonsai(input);
        let duration = start_time.elapsed();
        println!("Bonsai proof time: {:?}", duration);
        proof
    })
    .await?;

    prove_info
}

/// Prepares input data for the ZKVM for a single chain's proof data queries
///
/// # Arguments
/// * `users` - Vector of user addresses to query
/// * `markets` - Vector of market contract addresses to query
/// * `target_chain_ids` - Vector of target chain IDs to query
/// * `chain_id` - Chain ID for the queries
///
/// # Returns
/// * `Vec<u8>` - Serialized input data for the ZKVM
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided
/// - RPC calls fail
pub async fn get_proof_data_zkvm_input(
    users: Vec<Address>,
    markets: Vec<Address>,
    target_chain_ids: Vec<u64>,
    chain_id: u64,
) -> Vec<u8> {
    let rpc_url = match chain_id {
        BASE_CHAIN_ID => rpc_url_base(),
        OPTIMISM_CHAIN_ID => rpc_url_optimism(),
        LINEA_CHAIN_ID => rpc_url_linea(),
        ETHEREUM_CHAIN_ID => rpc_url_ethereum(),
        OPTIMISM_SEPOLIA_CHAIN_ID => rpc_url_optimism_sepolia(),
        BASE_SEPOLIA_CHAIN_ID => rpc_url_base_sepolia(),
        LINEA_SEPOLIA_CHAIN_ID => rpc_url_linea_sepolia(),
        ETHEREUM_SEPOLIA_CHAIN_ID => rpc_url_ethereum_sepolia(),
        _ => panic!("Invalid chain ID"),
    };

    let (block, commitment) = if chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == ETHEREUM_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID
    {
        let (commitment, block) = get_current_sequencer_commitment(chain_id).await;
        (Some(block), Some(commitment))
    } else if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
        let block = EthEvmEnv::builder()
            .rpc(Url::parse(rpc_url).unwrap())
            .block_number_or_tag(BlockNumberOrTag::Latest)
            .build()
            .await
            .unwrap()
            .header()
            .inner()
            .inner()
            .number;
        (Some(block), None)
    } else {
        panic!("Invalid chain ID");
    };

    let (l1_block_call_input, ethereum_block) =
        if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID {
            let chain_id = if chain_id == ETHEREUM_CHAIN_ID {
                OPTIMISM_CHAIN_ID
            } else {
                OPTIMISM_SEPOLIA_CHAIN_ID
            };
            let (l1_block_call_input, ethereum_block) =
                get_l1block_call_input(BlockNumberOrTag::Number(block.unwrap()), chain_id).await;

            (Some(l1_block_call_input), Some(ethereum_block))
        } else {
            (None, None)
        };

    let block = match chain_id {
        BASE_CHAIN_ID => block.unwrap(),
        OPTIMISM_CHAIN_ID => block.unwrap(),
        LINEA_CHAIN_ID => block.unwrap(),
        ETHEREUM_CHAIN_ID => ethereum_block.unwrap(),
        ETHEREUM_SEPOLIA_CHAIN_ID => ethereum_block.unwrap(),
        BASE_SEPOLIA_CHAIN_ID => block.unwrap(),
        OPTIMISM_SEPOLIA_CHAIN_ID => block.unwrap(),
        LINEA_SEPOLIA_CHAIN_ID => block.unwrap(),
        _ => panic!("Invalid chain ID"),
    };

    let (linking_blocks, proof_data_call_input) = tokio::join!(
        get_linking_blocks(chain_id, rpc_url, block),
        get_proof_data_call_input(
            chain_id,
            rpc_url,
            block,
            users.clone(),
            markets.clone(),
            target_chain_ids.clone()
        )
    );

    let input: Vec<u8> = bytemuck::pod_collect_to_vec(
        &risc0_zkvm::serde::to_vec(&(
            &proof_data_call_input,
            &chain_id,
            &users,
            &markets,
            &target_chain_ids,
            &commitment,
            &l1_block_call_input,
            &linking_blocks,
        ))
        .unwrap(),
    );

    input
}

/// Prepares multicall input for batch proof data checking
///
/// # Arguments
/// * `chain_id` - Chain ID for the queries
/// * `chain_url` - RPC URL for the chain
/// * `block` - Block number to query at
/// * `users` - Vector of user addresses
/// * `markets` - Vector of market contract addresses
/// * `target_chain_ids` - Vector of target chain IDs to query
///
/// # Returns
/// * `EvmInput<RlpHeader<Header>>` - Formatted EVM input for the multicall
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided
/// - RPC connection fails
pub async fn get_proof_data_call_input(
    chain_id: u64,
    chain_url: &str,
    block: u64,
    users: Vec<Address>,
    markets: Vec<Address>,
    target_chain_ids: Vec<u64>,
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
        .rpc(Url::parse(chain_url).expect("Failed to parse RPC URL"))
        .block_number_or_tag(BlockNumberOrTag::Number(block_reorg_protected))
        .build()
        .await
        .expect("Failed to build EVM environment");

    // Create array of Call3 structs for each proof data check
    let mut calls = Vec::with_capacity(users.len());

    for ((user, market), target_chain_id) in users
        .iter()
        .zip(markets.iter())
        .zip(target_chain_ids.iter())
    {
        // Selector for getProofData(address,uint32)
        let selector = [0x07, 0xd9, 0x23, 0xe9];
        let user_bytes: [u8; 32] = user.into_word().into();
        // Convert chain_id to 4 bytes
        let chain_id_bytes = (*target_chain_id as u32).to_be_bytes();

        // Create calldata by concatenating selector, encoded address, and chain ID
        let mut call_data = Vec::with_capacity(68); // 4 bytes selector + 32 bytes address + 4 bytes chain ID
        call_data.extend_from_slice(&selector);
        call_data.extend_from_slice(&user_bytes);
        call_data.extend_from_slice(&[0u8; 28]); // pad chain id to 32 bytes
        call_data.extend_from_slice(&chain_id_bytes);

        calls.push(Call3 {
            target: *market,
            allowFailure: false,
            callData: call_data.into(),
        });
    }

    // Make single multicall
    let multicall = IMulticall3::aggregate3Call { calls };

    // let gas_price = if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID {
    //     50000000000u64
    // } else {
    //     10000000000u64
    // };

    let mut contract = Contract::preflight(MULTICALL, &mut env);
    let _returns = contract
        .call_builder(&multicall)
        // .gas_price(U256::from(gas_price))
        // .from(Address::ZERO)
        .call()
        .await
        .expect("Failed to execute multicall");

    env.into_input()
        .await
        .expect("Failed to convert environment to input")
}

/// Fetches the current sequencer commitment for L2 chains
///
/// # Arguments
/// * `chain_id` - Chain ID (Optimism, Base, or their Sepolia variants)
///
/// # Returns
/// * `(SequencerCommitment, u64)` - Tuple of sequencer commitment and block number
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided
/// - Sequencer API request fails
pub async fn get_current_sequencer_commitment(chain_id: u64) -> (SequencerCommitment, u64) {
    let req = match chain_id {
        BASE_CHAIN_ID => sequencer_request_base(),
        OPTIMISM_CHAIN_ID => sequencer_request_optimism(),
        ETHEREUM_CHAIN_ID => sequencer_request_optimism(),
        OPTIMISM_SEPOLIA_CHAIN_ID => sequencer_request_optimism_sepolia(),
        BASE_SEPOLIA_CHAIN_ID => sequencer_request_base_sepolia(),
        ETHEREUM_SEPOLIA_CHAIN_ID => sequencer_request_optimism_sepolia(),
        _ => panic!("Invalid chain ID: {}", chain_id),
    };

    let commitment = reqwest::get(req)
        .await
        .expect("Failed to fetch sequencer commitment")
        .json::<SequencerCommitment>()
        .await
        .expect("Failed to parse sequencer commitment JSON");

    let block = ExecutionPayload::try_from(&commitment)
        .expect("Failed to convert commitment to execution payload")
        .block_number;

    (commitment, block)
}

/// Retrieves L1 block information for L2 chains
///
/// # Arguments
/// * `block` - Block number or tag to query
/// * `chain_id` - Chain ID (Optimism, Base, or their Sepolia variants)
///
/// # Returns
/// * `(EvmInput<RlpHeader<Header>>, u64)` - Tuple of L1 block input and block number
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided
/// - RPC calls fail
pub async fn get_l1block_call_input(
    block: BlockNumberOrTag,
    chain_id: u64,
) -> (EvmInput<RlpHeader<Header>>, u64) {
    let rpc_url = match chain_id {
        BASE_CHAIN_ID => rpc_url_base(),
        OPTIMISM_CHAIN_ID => rpc_url_optimism(),
        BASE_SEPOLIA_CHAIN_ID => rpc_url_base_sepolia(),
        OPTIMISM_SEPOLIA_CHAIN_ID => rpc_url_optimism_sepolia(),
        _ => panic!("Invalid chain ID for L1 block call: {}", chain_id),
    };
    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(rpc_url).expect("Failed to parse RPC URL"))
        .block_number_or_tag(block)
        .build()
        .await
        .expect("Failed to build EVM environment");

    let call = IL1Block::hashCall {};
    let mut contract = Contract::preflight(L1_BLOCK_ADDRESS_OPTIMISM, &mut env);
    contract
        .call_builder(&call)
        .call()
        .await
        .expect("Failed to call L1Block hash");

    let view_call_input_l1_block = env
        .into_input()
        .await
        .expect("Failed to convert environment to input");

    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(rpc_url).expect("Failed to parse RPC URL"))
        .block_number_or_tag(block)
        .build()
        .await
        .expect("Failed to build EVM environment");

    let call = IL1Block::numberCall {};
    let mut contract = Contract::preflight(L1_BLOCK_ADDRESS_OPTIMISM, &mut env);
    let l1_block = contract
        .call_builder(&call)
        .call()
        .await
        .expect("Failed to call L1Block number")
        ._0;

    (view_call_input_l1_block, l1_block)
}

/// Fetches a sequence of blocks for reorg protection
///
/// # Arguments
/// * `chain_id` - Chain ID to query
/// * `rpc_url` - RPC URL for the chain
/// * `current_block` - Latest block number to start from
///
/// # Returns
/// * `Vec<RlpHeader<Header>>` - Vector of block headers within the reorg protection window
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided
/// - RPC calls fail
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
        _ => panic!("Invalid chain ID: {}", chain_id),
    };

    let start_block = current_block - reorg_protection_depth + 1;

    // Create futures for parallel block fetching
    let futures: Vec<_> = (start_block..=current_block)
        .map(|block_nr| {
            let rpc_url = rpc_url.to_string();
            tokio::spawn(async move {
                let env = EthEvmEnv::builder()
                    .rpc(Url::parse(&rpc_url).expect("Failed to parse RPC URL"))
                    .block_number_or_tag(BlockNumberOrTag::Number(block_nr))
                    .build()
                    .await
                    .expect("Failed to build EVM environment");
                env.header().inner().clone()
            })
        })
        .collect();

    // Execute all futures in parallel and collect results
    join_all(futures)
        .await
        .into_iter()
        .map(|r| r.expect("Failed to join block fetch task"))
        .collect()
}
