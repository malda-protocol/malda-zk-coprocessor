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

use crate::constants::*;
use crate::elfs_ids::*;
use crate::types::*;
use crate::types::{Call3, IDisputeGame, IDisputeGameFactory, IL1MessageService, IMulticall3};
use crate::types::{ExecutionPayload, IL1Block, SequencerCommitment};
use core::panic;

use risc0_op_steel::optimism::OpEvmInput;
use risc0_steel::{
    ethereum::EthEvmEnv, host::BlockNumberOrTag, serde::RlpHeader, Contract, EvmInput,
};
use risc0_zkvm::{
    default_executor, default_prover, ExecutorEnv, ProveInfo, ProverOpts, SessionInfo,
};

use risc0_op_steel::{optimism::OpEvmEnv, DisputeGameIndex};

use alloy::primitives::{Address, U256, U64};
use alloy_consensus::Header;

use anyhow::{Error, Result};
use bonsai_sdk;
use futures::future::join_all;
use tokio;
use url::Url;

use std::time::Duration;

use bonsai_sdk::blocking::Client;
use risc0_zkvm::Receipt;
use tracing::info;

use dotenvy;

#[derive(Debug, Clone)]
pub struct MaldaSessionStats {
    pub segments: usize,
    pub total_cycles: u64,
    pub user_cycles: u64,
    pub paging_cycles: u64,
    pub reserved_cycles: u64,
}

#[derive(Debug)]
pub struct MaldaProveInfo {
    pub receipt: Receipt,
    pub stats: MaldaSessionStats,
    pub uuid: String,
    pub stark_time: u64,
    pub snark_time: u64,
}

/// Runs a Bonsai ZK proof session with the provided input data.
///
/// # Arguments
/// * `input_data` - The serialized input data for the ZKVM session.
///
/// # Returns
/// * `Result<MaldaProveInfo, anyhow::Error>` - Proof information and statistics if successful, or an error.
///
/// # Errors
/// Returns an error if:
/// - The Bonsai client fails to initialize.
/// - The input upload, session creation, or polling fails.
/// - The SNARK proof or receipt download fails.
/// - The receipt cannot be deserialized.
///
/// # Panics
/// Panics if the required environment variable `IMAGE_ID_BONSAI` is not set.
fn run_bonsai(input_data: Vec<u8>) -> Result<MaldaProveInfo, anyhow::Error> {

    let client = Client::from_env(risc0_zkvm::VERSION)?;

    let image_id_hex: String = dotenvy::var("IMAGE_ID_BONSAI")
        .expect("IMAGE_ID_BONSAI must be set in environment");

    let input_id = client.upload_input(input_data)?;

    let assumptions: Vec<String> = vec![];
    let execute_only = false;

    let session = client.create_session(image_id_hex, input_id, assumptions, execute_only)?;

    let polling_interval = Duration::from_millis(500);

    let stark_time = std::time::Instant::now();
    let succinct_stats = loop {
        let res = session.status(&client)?;
        if res.status == "RUNNING" {
            std::thread::sleep(polling_interval);
            continue;
        }
        if res.status == "SUCCEEDED" {

            let stats = res
                .stats
                .expect("Missing stats object on Bonsai status res");
            tracing::debug!(
                "Bonsai usage: cycles: {} total_cycles: {}",
                stats.cycles,
                stats.total_cycles
            );

            break MaldaSessionStats {
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
    let stark_time = stark_time.elapsed();
    let snark_session = client.create_snark(session.uuid.clone())?;

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

    let snark_time = start.elapsed();

    let receipt_buf = client.download(&snark_receipt_url)?;
    let groth16_receipt: Receipt = bincode::deserialize(&receipt_buf)?;


    Ok(MaldaProveInfo {
        receipt: groth16_receipt,
        stats: succinct_stats,
        uuid: session.uuid,
        stark_time: stark_time.as_secs(),
        snark_time: snark_time.as_secs(),
    })
}

/// Executes proof data queries across multiple chains in parallel.
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain.
/// * `markets` - Vector of market contract address vectors, one per chain.
/// * `target_chain_id` - Vector of target chain IDs to query (vector of vectors).
/// * `chain_ids` - Vector of chain IDs to query.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `Result<SessionInfo, Error>` - Session info from the ZKVM execution.
///
/// # Errors
/// Returns an error if:
/// - Array lengths don't match.
/// - RPC calls fail.
/// - ZKVM execution fails.
pub async fn get_proof_data_exec(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_id: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Result<SessionInfo, Error> {

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
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id, l1_inclusion)
                    .await
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

/// Creates the executor environment with proof data from multiple chains.
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain.
/// * `markets` - Vector of market contract address vectors, one per chain.
/// * `target_chain_ids` - Vector of target chain IDs to query (vector of vectors).
/// * `chain_ids` - Vector of chain IDs to query.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `ExecutorEnv<'static>` - Environment configured with proof data inputs.
///
/// # Panics
/// Panics if:
/// - Array lengths don't match.
async fn get_proof_data_env(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> ExecutorEnv<'static> {

    assert_eq!(users.len(), markets.len());
    assert_eq!(users.len(), chain_ids.len());

    let futures: Vec<_> = (0..chain_ids.len())
        .map(|i| {
            let users = users[i].clone();
            let markets = markets[i].clone();
            let chain_id = chain_ids[i];
            let target_chain_id = target_chain_ids[i].clone();
            tokio::spawn(async move {
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id, l1_inclusion)
                    .await
            })
        })
        .collect();

    let results = join_all(futures).await;
    let all_inputs = results
        .into_iter()
        .filter_map(|r| r.ok())
        .flat_map(|input| input)
        .collect::<Vec<_>>();

    ExecutorEnv::builder()
        .write(&(chain_ids.len() as u64))
        .unwrap()
        .write_slice(&all_inputs)
        .build()
        .unwrap()
}

/// Prepares input data for the ZKVM for multiple chains' proof data queries.
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain.
/// * `markets` - Vector of market contract address vectors, one per chain.
/// * `target_chain_ids` - Vector of target chain IDs to query (vector of vectors).
/// * `chain_ids` - Vector of chain IDs to query.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `Vec<u8>` - Serialized input data for the ZKVM.
///
/// # Panics
/// Panics if:
/// - Array lengths don't match.
async fn get_proof_data_input(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Vec<u8> {

    assert_eq!(users.len(), markets.len());
    assert_eq!(users.len(), chain_ids.len());

    let futures: Vec<_> = (0..chain_ids.len())
        .map(|i| {
            let users = users[i].clone();
            let markets = markets[i].clone();
            let chain_id = chain_ids[i];
            let target_chain_id = target_chain_ids[i].clone();
            tokio::spawn(async move {
                get_proof_data_zkvm_input(users, markets, target_chain_id, chain_id, l1_inclusion)
                    .await
            })
        })
        .collect();

    let results = join_all(futures).await;
    let all_inputs = results
        .into_iter()
        .filter_map(|r| r.ok())
        .flat_map(|input| input)
        .collect::<Vec<_>>();

    let input: Vec<u8> = bytemuck::pod_collect_to_vec(
        &risc0_zkvm::serde::to_vec(&(chain_ids.len() as u64)).unwrap(),
    );

    [input, all_inputs].concat()
}

/// Generates ZK proofs for proof data queries across multiple chains.
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain.
/// * `markets` - Vector of market contract address vectors, one per chain.
/// * `target_chain_ids` - Vector of target chain IDs to query (vector of vectors).
/// * `chain_ids` - Vector of chain IDs to query.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `Result<ProveInfo, Error>` - Proof information from the ZKVM.
///
/// # Errors
/// Returns an error if:
/// - Array lengths don't match.
/// - RPC calls fail.
/// - Proof generation fails.
pub async fn get_proof_data_prove(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Result<ProveInfo, Error> {

    let prove_info = tokio::task::spawn_blocking(move || {

        let rt = tokio::runtime::Runtime::new().unwrap();

        let start_time = std::time::Instant::now();
        let env = rt.block_on(get_proof_data_env(
            users,
            markets,
            target_chain_ids,
            chain_ids,
            l1_inclusion,
        ));
        let duration = start_time.elapsed();
        info!("Env creation time: {:?}", duration);

        let start_time = std::time::Instant::now();
        let proof =
            default_prover().prove_with_opts(env, GET_PROOF_DATA_ELF, &ProverOpts::groth16());
        let duration = start_time.elapsed();
        info!("Bonsai proof time: {:?}", duration);
        proof
    })
    .await?;

    prove_info
}

/// Generates ZK proofs for proof data queries across multiple chains using the Bonsai SDK.
///
/// # Arguments
/// * `users` - Vector of user address vectors, one per chain.
/// * `markets` - Vector of market contract address vectors, one per chain.
/// * `target_chain_ids` - Vector of target chain IDs to query (vector of vectors).
/// * `chain_ids` - Vector of chain IDs to query.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `Result<MaldaProveInfo, Error>` - Proof information from the Bonsai SDK.
///
/// # Errors
/// Returns an error if:
/// - Array lengths don't match.
/// - RPC calls fail.
/// - Proof generation fails.
pub async fn get_proof_data_prove_sdk(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Result<MaldaProveInfo, Error> {

    let prove_info = tokio::task::spawn_blocking(move || {

        let rt = tokio::runtime::Runtime::new().unwrap();

        let start_time = std::time::Instant::now();
        let input = rt.block_on(get_proof_data_input(
            users,
            markets,
            target_chain_ids,
            chain_ids,
            l1_inclusion,
        ));
        let duration = start_time.elapsed();
        info!("Env creation time: {:?}", duration);

        let start_time = std::time::Instant::now();
        let proof = run_bonsai(input);
        let duration = start_time.elapsed();
        info!("Bonsai proof time: {:?}", duration);
        proof
    })
    .await?;

    prove_info
}

/// Prepares input data for the ZKVM for a single chain's proof data queries.
///
/// # Arguments
/// * `users` - Vector of user addresses to query.
/// * `markets` - Vector of market contract addresses to query.
/// * `target_chain_ids` - Vector of target chain IDs to query.
/// * `chain_id` - Chain ID for the queries.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `Vec<u8>` - Serialized input data for the ZKVM.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
/// - RPC calls fail.
pub async fn get_proof_data_zkvm_input(
    users: Vec<Address>,
    markets: Vec<Address>,
    target_chain_ids: Vec<u64>,
    chain_id: u64,
    l1_inclusion: bool,
) -> Vec<u8> {
    let is_sepolia = chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID
        || chain_id == LINEA_SEPOLIA_CHAIN_ID;

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

    let (block, commitment, block_2, commitment_2) =
        get_sequencer_commitments_and_blocks(chain_id, rpc_url, is_sepolia, l1_inclusion).await;

    let (l1_block_call_input_1, ethereum_block_1, l1_block_call_input_2, _ethereum_block_2) =
        get_l1block_call_inputs_and_l1_block_numbers(
            chain_id,
            is_sepolia,
            l1_inclusion,
            block,
            block_2,
        )
        .await;

    let (env_input_l1_inclusion, l2_block_number_on_l1) =
        get_env_input_for_l1_inclusion_and_l2_block_number(
            chain_id,
            is_sepolia,
            l1_inclusion,
            ethereum_block_1,
        )
        .await;

    let block =
        if l1_inclusion && (chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID) {
            l2_block_number_on_l1.unwrap()
        } else if chain_id == ETHEREUM_CHAIN_ID
            || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID
            || (chain_id == OPTIMISM_CHAIN_ID
                || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
                || chain_id == BASE_CHAIN_ID
                || chain_id == BASE_SEPOLIA_CHAIN_ID)
                && l1_inclusion
        {
            ethereum_block_1.unwrap()
        } else {
            block.unwrap()
        };

    let (chaind_id_linking_blocks, rpc_url_linking_blocks) = if (chain_id == OPTIMISM_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID)
        && l1_inclusion
    {
        if chain_id == OPTIMISM_CHAIN_ID || chain_id == BASE_CHAIN_ID {
            (ETHEREUM_CHAIN_ID, rpc_url_ethereum())
        } else {
            (ETHEREUM_SEPOLIA_CHAIN_ID, rpc_url_ethereum_sepolia())
        }
    } else {
        (chain_id, rpc_url)
    };

    let (linking_blocks, (proof_data_call_input, proof_data_call_input_op)) = tokio::join!(
        get_linking_blocks(chaind_id_linking_blocks, rpc_url_linking_blocks, block),
        get_proof_data_call_input(
            chain_id,
            rpc_url,
            block,
            users.clone(),
            markets.clone(),
            target_chain_ids.clone(),
            l1_inclusion
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
            &l1_block_call_input_1,
            &linking_blocks,
            &env_input_l1_inclusion,
            &proof_data_call_input_op,
            &commitment_2,
            &l1_block_call_input_2,
        ))
        .unwrap(),
    );

    input
}

/// Returns the environment input for L1 inclusion and the L2 block number for a given chain.
///
/// # Arguments
/// * `chain_id` - The chain ID to query.
/// * `is_sepolia` - Whether the chain is a Sepolia testnet variant.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
/// * `ethereum_block` - The Ethereum block number (optional).
///
/// # Returns
/// * `(Option<EvmInput<RlpHeader<Header>>>, Option<u64>)` - The environment input and L2 block number, if available.
///
/// # Panics
/// Panics if:
/// - L1 inclusion is requested for an unsupported chain.
pub async fn get_env_input_for_l1_inclusion_and_l2_block_number(
    chain_id: u64,
    is_sepolia: bool,
    l1_inclusion: bool,
    ethereum_block: Option<u64>,
) -> (Option<EvmInput<RlpHeader<Header>>>, Option<u64>) {
    if !l1_inclusion {
        (None, None)
    } else {
        let l1_rpc_url = match is_sepolia {
            true => rpc_url_ethereum_sepolia(),
            false => rpc_url_ethereum(),
        };
        let l1_block = if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
            ethereum_block.unwrap()
        } else {
            if is_sepolia {
                ethereum_block.unwrap() - REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA
            } else if !is_sepolia {
                ethereum_block.unwrap() - REORG_PROTECTION_DEPTH_ETHEREUM
            } else {
                panic!("Invalid chain ID");
            }
        };

        if chain_id == OPTIMISM_CHAIN_ID
            || chain_id == BASE_CHAIN_ID
            || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
            || chain_id == BASE_SEPOLIA_CHAIN_ID
        {
            get_env_input_for_opstack_dispute_game(chain_id, l1_block).await
        } else if chain_id == LINEA_CHAIN_ID || chain_id == LINEA_SEPOLIA_CHAIN_ID {
            get_env_input_for_linea_l1_call(chain_id, l1_rpc_url, l1_block).await
        } else {
            panic!(
                "L1 Inclusion only supported for Optimism, Base, Linea and their Sepolia variants"
            );
        }
    }
}

/// Returns the environment input and L2 block number for Linea L1 call.
///
/// # Arguments
/// * `chain_id` - The chain ID to query.
/// * `l1_rpc_url` - The L1 RPC URL.
/// * `l1_block` - The L1 block number.
///
/// # Returns
/// * `(Option<EvmInput<RlpHeader<Header>>>, Option<u64>)` - The environment input and L2 block number, if available.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
pub async fn get_env_input_for_linea_l1_call(
    chain_id: u64,
    l1_rpc_url: &str,
    l1_block: u64,
) -> (Option<EvmInput<RlpHeader<Header>>>, Option<u64>) {
    let message_service_address = match chain_id {
        LINEA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA,
        LINEA_SEPOLIA_CHAIN_ID => L1_MESSAGE_SERVICE_LINEA_SEPOLIA,
        _ => panic!("Invalid chain ID"),
    };

    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(l1_rpc_url).expect("Failed to parse RPC URL"))
        .block_number_or_tag(BlockNumberOrTag::Number(l1_block))
        .build()
        .await
        .expect("Failed to build EVM environment");

    // Make single multicall
    let current_l2_block_number_call = IL1MessageService::currentL2BlockNumberCall {};

    let mut contract = Contract::preflight(message_service_address, &mut env);
    let returns = contract
        .call_builder(&current_l2_block_number_call)
        .call()
        .await
        .expect("Failed to execute current l2 block number call");

    let l2_block_number: u64 = U64::from(returns._0).try_into().unwrap();

    (
        Some(
            env.into_input()
                .await
                .expect("Failed to convert environment to input"),
        ),
        Some(l2_block_number),
    )
}

/// Returns the environment input for OpStack dispute game and a dummy L2 block number.
///
/// # Arguments
/// * `chain_id` - The chain ID to query.
/// * `l1_block` - The L1 block number.
///
/// # Returns
/// * `(Option<EvmInput<RlpHeader<Header>>>, Option<u64>)` - The environment input and a dummy L2 block number.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
pub async fn get_env_input_for_opstack_dispute_game(
    chain_id: u64,
    l1_block: u64,
) -> (Option<EvmInput<RlpHeader<Header>>>, Option<u64>) {
    let (l1_rpc_url, optimism_portal, l2_rpc_url) = match chain_id {
        OPTIMISM_CHAIN_ID => (rpc_url_ethereum(), OPTIMISM_PORTAL, rpc_url_optimism()),
        OPTIMISM_SEPOLIA_CHAIN_ID => (
            rpc_url_ethereum_sepolia(),
            OPTIMISM_SEPOLIA_PORTAL,
            rpc_url_optimism_sepolia(),
        ),
        BASE_CHAIN_ID => (rpc_url_ethereum(), BASE_PORTAL, rpc_url_base()),
        BASE_SEPOLIA_CHAIN_ID => (
            rpc_url_ethereum_sepolia(),
            BASE_SEPOLIA_PORTAL,
            rpc_url_base_sepolia(),
        ),
        _ => panic!("Invalid chain ID"),
    };

    let mut env = EthEvmEnv::builder()
        .rpc(Url::parse(l1_rpc_url).expect("Failed to parse RPC URL"))
        .block_number_or_tag(BlockNumberOrTag::Number(l1_block))
        .build()
        .await
        .expect("Failed to build EVM environment");
    let builder = OpEvmEnv::builder()
        .dispute_game_from_rpc(
            optimism_portal,
            Url::parse(l1_rpc_url).expect("Failed to parse RPC URL"),
        )
        .game_index(DisputeGameIndex::Finalized);
    let mut op_env = builder
        .rpc(Url::parse(l2_rpc_url).expect("Failed to parse RPC URL"))
        .build()
        .await
        .expect("Failed to build OP-EVM environment");

    // This is just an arbitrary simple call needed in order to do into_env to get the game_index
    let mut contract = Contract::preflight(L1_BLOCK_ADDRESS_OPSTACK, &mut op_env);
    let block_hash_call = IL1Block::hashCall {};
    let _returns = contract
        .call_builder(&block_hash_call)
        .call()
        .await
        .expect("Failed to execute factory call");

    let input = op_env
        .into_input()
        .await
        .expect("Failed to convert environment to input");
    let op_env_commitment = input.clone().into_env().into_commitment();

    let (game_index, _version) = op_env_commitment.decode_id();

    let root_claim = op_env_commitment.digest;

    let portal_adress = match chain_id {
        OPTIMISM_SEPOLIA_CHAIN_ID => OPTIMISM_SEPOLIA_PORTAL,
        BASE_SEPOLIA_CHAIN_ID => BASE_SEPOLIA_PORTAL,
        OPTIMISM_CHAIN_ID => OPTIMISM_PORTAL,
        BASE_CHAIN_ID => BASE_PORTAL,
        _ => panic!("invalid chain id"),
    };

    // Get the portal contract for additional checks
    let mut contract = Contract::preflight(portal_adress, &mut env);

    // Get factory address from portal
    let factory_call = IOptimismPortal::disputeGameFactoryCall {};
    let returns = contract
        .call_builder(&factory_call)
        .call()
        .await
        .expect("Failed to execute factory call");
    let factory_address = returns._0;

    let game_call = IDisputeGameFactory::gameAtIndexCall { index: game_index };

    let mut contract = Contract::preflight(factory_address, &mut env);
    let returns = contract
        .call_builder(&game_call)
        .call()
        .await
        .expect("Failed to execute game at index call");

    let game_type = returns._0;
    assert_eq!(game_type, U256::from(0), "game type not respected game");

    let created_at = returns._1;
    let game_address = returns._2;

    // Check if game was created after respected game type update
    let mut contract = Contract::preflight(portal_adress, &mut env);
    let respected_game_type_updated_at_call = IOptimismPortal::respectedGameTypeUpdatedAtCall {};
    let returns = contract
        .call_builder(&respected_game_type_updated_at_call)
        .call()
        .await
        .expect("Failed to execute respected game type updated at call");
    assert!(
        created_at >= returns._0,
        "game created before respected game type update"
    );

    // Get game contract for status checks
    let mut contract = Contract::preflight(game_address, &mut env);

    // Check game status
    let status_call = IDisputeGame::statusCall {};
    let returns = contract
        .call_builder(&status_call)
        .call()
        .await
        .expect("Failed to execute status call");
    assert_eq!(
        returns._0,
        GameStatus::DEFENDER_WINS,
        "game status not DEFENDER_WINS"
    );

    // Check if game is blacklisted
    let mut contract = Contract::preflight(portal_adress, &mut env);
    let blacklist_call = IOptimismPortal::disputeGameBlacklistCall { game: game_address };
    let returns = contract
        .call_builder(&blacklist_call)
        .call()
        .await
        .expect("Failed to execute blacklist call");
    assert!(!returns._0, "game is blacklisted");

    // Check game resolution time
    let mut contract = Contract::preflight(game_address, &mut env);
    let resolved_at_call = IDisputeGame::resolvedAtCall {};
    let returns = contract
        .call_builder(&resolved_at_call)
        .call()
        .await
        .expect("Failed to execute resolved at call");
    let resolved_at = returns._0;

    let mut contract = Contract::preflight(portal_adress, &mut env);
    let proof_maturity_delay_call = IOptimismPortal::proofMaturityDelaySecondsCall {};
    let returns = contract
        .call_builder(&proof_maturity_delay_call)
        .call()
        .await
        .expect("Failed to execute proof maturity delay call");
    let proof_maturity_delay = returns._0;

    let current_timestamp = env.header().inner().inner().timestamp;
    assert!(
        U256::from(current_timestamp) - U256::from(resolved_at)
            > proof_maturity_delay - U256::from(300),
        "insufficient time passed since game resolution"
    );

    // Finally verify root claim matches
    let mut contract = Contract::preflight(game_address, &mut env);
    let root_claim_call = IDisputeGame::rootClaimCall {};
    let returns = contract
        .call_builder(&root_claim_call)
        .call()
        .await
        .expect("Failed to execute root claim call");

    assert_eq!(returns._0, root_claim, "root claim not respected");

    (
        Some(
            env.into_input()
                .await
                .expect("Failed to convert environment to input"),
        ),
        // irrelevant for l1 inclusion on opstack
        Some(1),
    )
}

/// Returns L1 block call inputs and L1 block numbers for a given chain.
///
/// # Arguments
/// * `chain_id` - The chain ID to query.
/// * `is_sepolia` - Whether the chain is a Sepolia testnet variant.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
/// * `block` - The block number (optional).
/// * `_block_2` - The second block number (optional, unused).
///
/// # Returns
/// * Tuple of optional L1 block call inputs and block numbers.
///
/// # Panics
/// Panics if:
/// - Block number is not provided when required.
pub async fn get_l1block_call_inputs_and_l1_block_numbers(
    chain_id: u64,
    is_sepolia: bool,
    l1_inclusion: bool,
    block: Option<u64>,
    _block_2: Option<u64>,
) -> (
    Option<EvmInput<RlpHeader<Header>>>,
    Option<u64>,
    Option<EvmInput<RlpHeader<Header>>>,
    Option<u64>,
) {
    if chain_id == ETHEREUM_CHAIN_ID || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID || l1_inclusion {
        let (chain_id_1, _chain_id_2) = match is_sepolia {
            true => (OPTIMISM_SEPOLIA_CHAIN_ID, BASE_SEPOLIA_CHAIN_ID),
            false => (OPTIMISM_CHAIN_ID, BASE_CHAIN_ID),
        };
        let (l1_block_call_input_1, ethereum_block_1) =
            get_l1block_call_input(BlockNumberOrTag::Number(block.unwrap()), chain_id_1).await;
        // let (l1_block_call_input_2, ethereum_block_2) =
        //     get_l1block_call_input(BlockNumberOrTag::Number(block_2.unwrap()), chain_id_2).await;

        (
            Some(l1_block_call_input_1),
            Some(ethereum_block_1),
            None::<EvmInput<RlpHeader<Header>>>,
            None::<u64>,
        )
        // (Some(l1_block_call_input_1), Some(ethereum_block_1), Some(l1_block_call_input_2), Some(ethereum_block_2))
    } else {
        (None, None, None, None)
    }
}

/// Prepares multicall input for batch proof data checking.
///
/// # Arguments
/// * `chain_id` - Chain ID for the queries.
/// * `chain_url` - RPC URL for the chain.
/// * `block` - Block number to query at.
/// * `users` - Vector of user addresses.
/// * `markets` - Vector of market contract addresses.
/// * `target_chain_ids` - Vector of target chain IDs to query.
/// * `validate_l1_inclusion` - Whether to validate L1 inclusion for OpStack chains.
///
/// # Returns
/// * `(Option<EvmInput<RlpHeader<Header>>>, Option<OpEvmInput>)` - Formatted EVM input for the multicall and optional OpEvmInput.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
/// - RPC connection fails.
pub async fn get_proof_data_call_input(
    chain_id: u64,
    chain_url: &str,
    block: u64,
    users: Vec<Address>,
    markets: Vec<Address>,
    target_chain_ids: Vec<u64>,
    validate_l1_inclusion: bool,
) -> (Option<EvmInput<RlpHeader<Header>>>, Option<OpEvmInput>) {
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

    // Use separate code paths for each environment type
    if (chain_id == OPTIMISM_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID)
        && validate_l1_inclusion
    {
        // Build an environment based on the state of the latest finalized fault dispute game
        let (l1_rpc_url, optimism_portal) = match chain_id {
            OPTIMISM_CHAIN_ID => (rpc_url_ethereum(), OPTIMISM_PORTAL),
            OPTIMISM_SEPOLIA_CHAIN_ID => (rpc_url_ethereum_sepolia(), OPTIMISM_SEPOLIA_PORTAL),
            BASE_CHAIN_ID => (rpc_url_ethereum(), BASE_PORTAL),
            BASE_SEPOLIA_CHAIN_ID => (rpc_url_ethereum_sepolia(), BASE_SEPOLIA_PORTAL),
            _ => panic!("Invalid chain ID"),
        };
        let builder = OpEvmEnv::builder()
            .dispute_game_from_rpc(
                optimism_portal,
                Url::parse(l1_rpc_url).expect("Failed to parse RPC URL"),
            )
            .game_index(DisputeGameIndex::Finalized);
        let mut env = builder
            .rpc(Url::parse(chain_url).expect("Failed to parse RPC URL"))
            .build()
            .await
            .expect("Failed to build OP-EVM environment");

        let mut contract = Contract::preflight(MULTICALL, &mut env);
        let _returns = contract
            .call_builder(&multicall)
            // .gas_price(U256::from(gas_price))
            // .from(Address::ZERO)
            .call()
            .await
            .expect("Failed to execute multicall");

        (
            None,
            Some(
                env.into_input()
                    .await
                    .expect("Failed to convert environment to input"),
            ),
        )
    } else {
        let mut env = EthEvmEnv::builder()
            .rpc(Url::parse(chain_url).expect("Failed to parse RPC URL"))
            .block_number_or_tag(BlockNumberOrTag::Number(block_reorg_protected))
            .build()
            .await
            .expect("Failed to build EVM environment");

        let mut contract = Contract::preflight(MULTICALL, &mut env);
        let _returns = contract
            .call_builder(&multicall)
            // .gas_price(U256::from(gas_price))
            // .from(Address::ZERO)
            .call()
            .await
            .expect("Failed to execute multicall");

        (
            Some(
                env.into_input()
                    .await
                    .expect("Failed to convert environment to input"),
            ),
            None,
        )
    }
}

/// Fetches sequencer commitments and block numbers for a given chain, handling L1 inclusion and Sepolia/mainnet variants.
///
/// # Arguments
/// * `chain_id` - The chain ID to query.
/// * `rpc_url` - The RPC URL for the chain.
/// * `is_sepolia` - Whether the chain is a Sepolia testnet variant.
/// * `l1_inclusion` - Whether to include L1 data in the proof.
///
/// # Returns
/// * `(Option<u64>, Option<SequencerCommitment>, Option<u64>, Option<SequencerCommitment>)` -
///   Tuple of (block, commitment, block_2, commitment_2), where the second pair is only relevant for some Sepolia/mainnet cases.
///
/// # Panics
/// Panics if:
/// - An invalid chain ID is provided.
/// - RPC calls fail.
pub async fn get_sequencer_commitments_and_blocks(
    chain_id: u64,
    rpc_url: &str,
    is_sepolia: bool,
    l1_inclusion: bool,
) -> (
    Option<u64>,
    Option<SequencerCommitment>,
    Option<u64>,
    Option<SequencerCommitment>,
) {
    if chain_id == OPTIMISM_CHAIN_ID
        || chain_id == BASE_CHAIN_ID
        || chain_id == ETHEREUM_CHAIN_ID
        || chain_id == OPTIMISM_CHAIN_ID
        || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
        || chain_id == BASE_SEPOLIA_CHAIN_ID
        || chain_id == ETHEREUM_SEPOLIA_CHAIN_ID
        || (chain_id == LINEA_CHAIN_ID && l1_inclusion)
        || (chain_id == LINEA_SEPOLIA_CHAIN_ID && l1_inclusion)
    {
        if !l1_inclusion
            && (chain_id == OPTIMISM_CHAIN_ID
                || chain_id == BASE_CHAIN_ID
                || chain_id == OPTIMISM_SEPOLIA_CHAIN_ID
                || chain_id == BASE_SEPOLIA_CHAIN_ID)
        {
            let (commitment, block) = get_current_sequencer_commitment(chain_id).await;
            (
                Some(block),
                Some(commitment),
                None::<u64>,
                None::<SequencerCommitment>,
            )
        } else if is_sepolia {
            let (commitment, block) =
                get_current_sequencer_commitment(OPTIMISM_SEPOLIA_CHAIN_ID).await;
            // let (commitment_2, block_2) = get_current_sequencer_commitment(BASE_SEPOLIA_CHAIN_ID).await;
            (Some(block), Some(commitment), None, None)
            // (Some(block), Some(commitment), Some(block_2), Some(commitment_2))
        } else if !is_sepolia {
            let (commitment, block) = get_current_sequencer_commitment(OPTIMISM_CHAIN_ID).await;
            // let (commitment_2, block_2) = get_current_sequencer_commitment(BASE_CHAIN_ID).await;
            (Some(block), Some(commitment), None, None)
            // (Some(block), Some(commitment), Some(block_2), Some(commitment_2))
        } else {
            panic!("Invalid chain ID");
        }
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
        (Some(block), None, None, None)
    } else {
        panic!("Invalid chain ID");
    }
}
/// Fetches the current sequencer commitment for L2 chains.
///
/// # Arguments
/// * `chain_id` - Chain ID (Optimism, Base, or their Sepolia variants).
///
/// # Returns
/// * `(SequencerCommitment, u64)` - Tuple of sequencer commitment and block number.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
/// - Sequencer API request fails.
pub async fn get_current_sequencer_commitment(chain_id: u64) -> (SequencerCommitment, u64) {
    let req = match chain_id {
        BASE_CHAIN_ID => sequencer_request_base(),
        OPTIMISM_CHAIN_ID => sequencer_request_optimism(),
        OPTIMISM_SEPOLIA_CHAIN_ID => sequencer_request_optimism_sepolia(),
        BASE_SEPOLIA_CHAIN_ID => sequencer_request_base_sepolia(),
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

/// Retrieves L1 block information for L2 chains.
///
/// # Arguments
/// * `block` - Block number or tag to query.
/// * `chain_id` - Chain ID (Optimism, Base, or their Sepolia variants).
///
/// # Returns
/// * `(EvmInput<RlpHeader<Header>>, u64)` - Tuple of L1 block input and block number.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
/// - RPC calls fail.
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
    let mut contract = Contract::preflight(L1_BLOCK_ADDRESS_OPSTACK, &mut env);
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
    let mut contract = Contract::preflight(L1_BLOCK_ADDRESS_OPSTACK, &mut env);
    let l1_block = contract
        .call_builder(&call)
        .call()
        .await
        .expect("Failed to call L1Block number")
        ._0;

    (view_call_input_l1_block, l1_block)
}

/// Fetches a sequence of blocks for reorg protection.
///
/// # Arguments
/// * `chain_id` - Chain ID to query.
/// * `rpc_url` - RPC URL for the chain.
/// * `current_block` - Latest block number to start from.
///
/// # Returns
/// * `Vec<RlpHeader<Header>>` - Vector of block headers within the reorg protection window.
///
/// # Panics
/// Panics if:
/// - Invalid chain ID is provided.
/// - RPC calls fail.
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
