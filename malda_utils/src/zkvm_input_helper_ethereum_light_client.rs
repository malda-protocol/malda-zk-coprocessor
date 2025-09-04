//! ZKVM input helper functions for Ethereum light client verification.
//!
//! This module provides helper functions for reading Ethereum light client data
//! from the RISC Zero zkVM environment, including:
//! - BeaconBlockHeader deserialization
//! - LightClientHeader deserialization
//! - ExecutionPayloadHeader deserialization
//! - Light client input data reading

use consensus_core::types::{Bootstrap, LightClientHeader, OptimisticUpdate, Update, UpdateElectra, BootstrapElectra};
use consensus_core::consensus_spec::MainnetConsensusSpec;
use ssz_types::{FixedVector, typenum};
use alloy_primitives::{Address, B256};
use risc0_steel::ethereum::EthEvmInput;
use risc0_zkvm::guest::env;

/// Helper function to read BeaconBlockHeader by deconstructing its fields
pub fn read_beacon_block_header() -> consensus_core::types::BeaconBlockHeader {
    let slot: u64 = env::read();
    let proposer_index: u64 = env::read();
    let parent_root: B256 = env::read();
    let state_root: B256 = env::read();
    let body_root: B256 = env::read();
    
    consensus_core::types::BeaconBlockHeader {
        slot,
        proposer_index,
        parent_root,
        state_root,
        body_root,
    }
}

/// Helper function to read LightClientHeader by deconstructing its fields
pub fn read_light_client_header() -> LightClientHeader {
    let variant: u8 = env::read(); // 0 = Bellatrix, 1 = Capella, 2 = Deneb, 3 = Electra
    
    match variant {
        0 => {
            // Bellatrix variant - only has beacon field
            let beacon = read_beacon_block_header();
            LightClientHeader::Bellatrix(consensus_core::types::LightClientHeaderBellatrix {
                beacon,
            })
        }
        1 => {
            // Capella variant - has beacon, execution, and execution_branch
            let beacon = read_beacon_block_header();
            let execution = read_execution_payload_header();
            let execution_branch: Vec<B256> = env::read();
            LightClientHeader::Capella(consensus_core::types::LightClientHeaderCapella {
                beacon,
                execution,
                execution_branch: FixedVector::from(execution_branch),
            })
        }
        2 => {
            // Deneb variant
            let beacon = read_beacon_block_header();
            let execution = read_execution_payload_header();
            let execution_branch: Vec<B256> = env::read();
            LightClientHeader::Deneb(consensus_core::types::LightClientHeaderDeneb {
                beacon,
                execution,
                execution_branch: FixedVector::from(execution_branch),
            })
        }
        3 => {
            // Electra variant
            let beacon = read_beacon_block_header();
            let execution = read_execution_payload_header();
            let execution_branch: Vec<B256> = env::read();
            LightClientHeader::Electra(consensus_core::types::LightClientHeaderElectra {
                beacon,
                execution,
                execution_branch: FixedVector::from(execution_branch),
            })
        }
        _ => panic!("Invalid LightClientHeader variant: {}", variant),
    }
}

/// Helper function to read ExecutionPayloadHeader by deconstructing its fields
pub fn read_execution_payload_header() -> consensus_core::types::ExecutionPayloadHeader {
    let variant: u8 = env::read(); // 0 = Bellatrix, 1 = Capella, 2 = Deneb, 3 = Electra
    
    match variant {
        0 => {
            // Bellatrix variant - read all fields
            let parent_hash: B256 = env::read();
            let fee_recipient: Address = env::read();
            let state_root: B256 = env::read();
            let receipts_root: B256 = env::read();
            let logs_bloom: consensus_core::types::LogsBloom = env::read();
            let prev_randao: B256 = env::read();
            let block_number: u64 = env::read();
            let gas_limit: u64 = env::read();
            let gas_used: u64 = env::read();
            let timestamp: u64 = env::read();
            let extra_data: consensus_core::types::bytes::ByteList<typenum::U32> = env::read();
            let base_fee_per_gas: alloy_primitives::U256 = env::read();
            let block_hash: B256 = env::read();
            let transactions_root: B256 = env::read();
            
            consensus_core::types::ExecutionPayloadHeader::Bellatrix(
                consensus_core::types::ExecutionPayloadHeaderBellatrix {
                    parent_hash,
                    fee_recipient,
                    state_root,
                    receipts_root,
                    logs_bloom,
                    prev_randao,
                    block_number,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    base_fee_per_gas,
                    block_hash,
                    transactions_root,
                }
            )
        }
        1 => {
            // Capella variant - read all fields
            let parent_hash: B256 = env::read();
            let fee_recipient: Address = env::read();
            let state_root: B256 = env::read();
            let receipts_root: B256 = env::read();
            let logs_bloom: consensus_core::types::LogsBloom = env::read();
            let prev_randao: B256 = env::read();
            let block_number: u64 = env::read();
            let gas_limit: u64 = env::read();
            let gas_used: u64 = env::read();
            let timestamp: u64 = env::read();
            let extra_data: consensus_core::types::bytes::ByteList<typenum::U32> = env::read();
            let base_fee_per_gas: alloy_primitives::U256 = env::read();
            let block_hash: B256 = env::read();
            let transactions_root: B256 = env::read();
            let withdrawals_root: B256 = env::read();
            
            consensus_core::types::ExecutionPayloadHeader::Capella(
                consensus_core::types::ExecutionPayloadHeaderCapella {
                    parent_hash,
                    fee_recipient,
                    state_root,
                    receipts_root,
                    logs_bloom,
                    prev_randao,
                    block_number,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    base_fee_per_gas,
                    block_hash,
                    transactions_root,
                    withdrawals_root,
                }
            )
        }
        2 => {
            // Deneb variant - read all fields
            let parent_hash: B256 = env::read();
            let fee_recipient: Address = env::read();
            let state_root: B256 = env::read();
            let receipts_root: B256 = env::read();
            let logs_bloom: consensus_core::types::LogsBloom = env::read();
            let prev_randao: B256 = env::read();
            let block_number: u64 = env::read();
            let gas_limit: u64 = env::read();
            let gas_used: u64 = env::read();
            let timestamp: u64 = env::read();
            let extra_data: consensus_core::types::bytes::ByteList<typenum::U32> = env::read();
            let base_fee_per_gas: alloy_primitives::U256 = env::read();
            let block_hash: B256 = env::read();
            let transactions_root: B256 = env::read();
            let withdrawals_root: B256 = env::read();
            let blob_gas_used: u64 = env::read();
            let excess_blob_gas: u64 = env::read();
            
            consensus_core::types::ExecutionPayloadHeader::Deneb(
                consensus_core::types::ExecutionPayloadHeaderDeneb {
                    parent_hash,
                    fee_recipient,
                    state_root,
                    receipts_root,
                    logs_bloom,
                    prev_randao,
                    block_number,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    base_fee_per_gas,
                    block_hash,
                    transactions_root,
                    withdrawals_root,
                    blob_gas_used,
                    excess_blob_gas,
                }
            )
        }
        3 => {
            // Electra variant - read all fields
            let parent_hash: B256 = env::read();
            let fee_recipient: Address = env::read();
            let state_root: B256 = env::read();
            let receipts_root: B256 = env::read();
            let logs_bloom: consensus_core::types::LogsBloom = env::read();
            let prev_randao: B256 = env::read();
            let block_number: u64 = env::read();
            let gas_limit: u64 = env::read();
            let gas_used: u64 = env::read();
            let timestamp: u64 = env::read();
            let extra_data: consensus_core::types::bytes::ByteList<typenum::U32> = env::read();
            let base_fee_per_gas: alloy_primitives::U256 = env::read();
            let block_hash: B256 = env::read();
            let transactions_root: B256 = env::read();
            let withdrawals_root: B256 = env::read();
            let blob_gas_used: u64 = env::read();
            let excess_blob_gas: u64 = env::read();
            
            consensus_core::types::ExecutionPayloadHeader::Electra(
                consensus_core::types::ExecutionPayloadHeaderElectra {
                    parent_hash,
                    fee_recipient,
                    state_root,
                    receipts_root,
                    logs_bloom,
                    prev_randao,
                    block_number,
                    gas_limit,
                    gas_used,
                    timestamp,
                    extra_data,
                    base_fee_per_gas,
                    block_hash,
                    transactions_root,
                    withdrawals_root,
                    blob_gas_used,
                    excess_blob_gas,
                }
            )
        }
        _ => panic!("Invalid ExecutionPayloadHeader variant: {}", variant),
    }
}

/// Reads light client input data from the guest environment.
///
/// Deserializes the following data:
/// - Bootstrap data (header, sync committee, proof)
/// - Trust checkpoint
/// - Update sequence
/// - Finality update
/// - Ethereum environment input
///
/// # Returns
/// Tuple containing all deserialized components needed for light client verification
pub fn read_l1_chain_builder_input() -> (
    Bootstrap<MainnetConsensusSpec>,
    B256,
    Vec<Update<MainnetConsensusSpec>>,
    OptimisticUpdate<MainnetConsensusSpec>,
    EthEvmInput,
) {

    let bootstrap_header = read_light_client_header();

    let bootstrap_current_sync_committee: consensus_core::types::SyncCommittee<MainnetConsensusSpec> = env::read();
    let bootstrap_current_sync_committee_branch: Vec<B256> = env::read();

    let checkpoint: B256 = env::read();

    let finality_update_attested_header = read_light_client_header();
    let finality_update_sync_aggregate: consensus_core::types::SyncAggregate<MainnetConsensusSpec> = env::read();
    let finality_update_signature_slot: u64 = env::read();

    let update_len: usize = env::read();
    let mut updates: Vec<Update<MainnetConsensusSpec>> = Vec::new();
    for _ in 0..update_len {
        let update_attested_header = read_light_client_header();
        let update_next_sync_committee: consensus_core::types::SyncCommittee<MainnetConsensusSpec> = env::read();
        let update_next_sync_committee_branch: Vec<B256> = env::read();
        let update_finalized_header = read_light_client_header();
        let update_finality_branch: Vec<B256> = env::read();
        let update_sync_aggregate: consensus_core::types::SyncAggregate<MainnetConsensusSpec> = env::read();
        let update_signature_slot: u64 = env::read();

        let update = Update::Electra(UpdateElectra {
            attested_header: update_attested_header,
            next_sync_committee: update_next_sync_committee,
            next_sync_committee_branch: FixedVector::from(update_next_sync_committee_branch),
            finalized_header: update_finalized_header,
            finality_branch: FixedVector::from(update_finality_branch),
            sync_aggregate: update_sync_aggregate,
            signature_slot: update_signature_slot,
        });
        updates.push(update);
    }


    // Create the Bootstrap::Electra variant
    let bootstrap = Bootstrap::<MainnetConsensusSpec>::Electra(BootstrapElectra {
        header: bootstrap_header,
        current_sync_committee: bootstrap_current_sync_committee,
        current_sync_committee_branch: FixedVector::from(bootstrap_current_sync_committee_branch),
    });


    let finality_update = OptimisticUpdate::<MainnetConsensusSpec> {
        attested_header: finality_update_attested_header,
        sync_aggregate: finality_update_sync_aggregate,
        signature_slot: finality_update_signature_slot,
    };

    let beacon_input: EthEvmInput = env::read();

    (
        bootstrap,
        checkpoint,
        updates,
        finality_update,
        beacon_input,
    )
}
