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
//! Validator functions for Ethereum light client verification.
//!
//! This module provides validation utilities for Ethereum light client proofs including:
//! - Light client store management
//! - Beacon chain bootstrapping and updates
//! - Sync committee verification
//! - Proof data validation using light client proofs

use consensus_core::{
    apply_bootstrap, apply_optimistic_update, apply_update, verify_bootstrap,
    verify_optimistic_update, verify_update,
};

pub use consensus_core::types::{
    Bootstrap, Forks, LightClientHeader, LightClientStore, OptimisticUpdate, Update,
};

use alloy_primitives::{b256, B256};
pub use alloy_primitives_old::{fixed_bytes as old_fixed_bytes, B256 as OldB256};
use alloy_sol_types::sol;
use eyre::Result;
use tree_hash::TreeHash;

use alloy_primitives::Address;
use risc0_steel::ethereum::EthEvmInput;
use risc0_zkvm::guest::env;

use consensus_core::types::{SyncAggregate, SyncCommittee};

use crate::constants::*;
use crate::types::*;
use alloy_consensus::Header as ConsensusHeader;
use alloy_sol_types::SolValue;
use risc0_steel::{serde::RlpHeader, Contract};

/// Builder for managing Ethereum L1 light client state.
///
/// Maintains the light client store and handles beacon chain updates through
/// bootstrap, sync committee updates, and optimistic updates.
#[derive(Debug)]
pub struct L1ChainBuilder {
    pub store: LightClientStore,
    pub last_checkpoint: Option<B256>,
    pub genesis_time: u64,
    pub genesis_root: B256,
    pub forks: Forks,
}

impl L1ChainBuilder {
    /// Creates a new L1ChainBuilder with default settings for mainnet.
    ///
    /// Initializes with:
    /// - Empty light client store
    /// - Deneb fork configuration
    /// - Mainnet genesis parameters
    pub fn new() -> Self {
        let store = LightClientStore::default();

        let mut forks = Forks::default();
        forks.deneb.epoch = 269568;
        forks.deneb.fork_version = old_fixed_bytes!("04000000");
        let genesis_root =
            b256!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95");
        let genesis_time = 1606824023;

        L1ChainBuilder {
            store,
            last_checkpoint: None,
            genesis_root,
            forks,
            genesis_time,
        }
    }

    /// Builds a beacon chain from bootstrap data and updates.
    ///
    /// # Arguments
    /// * `bootstrap` - Initial bootstrap data
    /// * `checkpoint` - Trust checkpoint hash
    /// * `updates` - Vector of light client updates
    /// * `optimistic_update` - Latest optimistic update
    ///
    /// # Returns
    /// * Latest beacon chain root after applying all updates
    pub fn build_beacon_chain(
        &mut self,
        bootstrap: Bootstrap,
        checkpoint: OldB256,
        updates: Vec<Update>,
        optimistic_update: OptimisticUpdate,
    ) -> Result<B256> {
        self.bootstrap(bootstrap, checkpoint)?;
        self.advance_updates(updates)?;
        self.advance_optimistic_update(optimistic_update)?;
        let latest_beacon_root = self.store.optimistic_header.beacon.tree_hash_root();
        Ok(B256::new(latest_beacon_root.0))
    }

    /// Bootstraps the light client with initial data.
    ///
    /// # Arguments
    /// * `bootstrap` - Bootstrap data containing initial header and sync committee
    /// * `checkpoint` - Trust checkpoint to verify against
    pub fn bootstrap(&mut self, bootstrap: Bootstrap, checkpoint: OldB256) -> Result<()> {
        verify_bootstrap(&bootstrap, checkpoint, &self.forks).unwrap();
        apply_bootstrap(&mut self.store, &bootstrap);
        Ok(())
    }

    /// Processes a sequence of light client updates.
    ///
    /// # Arguments
    /// * `updates` - Vector of updates to apply
    pub fn advance_updates(&mut self, updates: Vec<Update>) -> Result<()> {
        for update in updates {
            let res = self.verify_update(&update);
            if res.is_ok() {
                self.apply_update(&update);
            }
        }

        Ok(())
    }

    /// Processes an optimistic update.
    ///
    /// # Arguments
    /// * `update` - Optimistic update to apply
    pub fn advance_optimistic_update(&mut self, update: OptimisticUpdate) -> Result<()> {
        let res = self.verify_optimistic_update(&update);
        if res.is_ok() {
            self.apply_optimistic_update(&update);
        }
        Ok(())
    }

    /// Verifies a light client update.
    ///
    /// # Arguments
    /// * `update` - Update to verify
    pub fn verify_update(&self, update: &Update) -> Result<()> {
        verify_update(
            update,
            update.signature_slot,
            &self.store,
            OldB256::from(self.genesis_root.0),
            &self.forks,
        )
    }

    /// Verifies an optimistic update.
    ///
    /// # Arguments
    /// * `update` - Optimistic update to verify
    fn verify_optimistic_update(&self, update: &OptimisticUpdate) -> Result<()> {
        verify_optimistic_update(
            update,
            update.signature_slot,
            &self.store,
            OldB256::from(self.genesis_root.0),
            &self.forks,
        )
    }

    /// Applies a verified update to the light client store.
    ///
    /// # Arguments
    /// * `update` - Verified update to apply
    pub fn apply_update(&mut self, update: &Update) {
        let new_checkpoint = apply_update(&mut self.store, update);
        if new_checkpoint.is_some() {
            self.last_checkpoint = Some(B256::new(new_checkpoint.unwrap().0));
        }
    }

    /// Applies a verified optimistic update to the light client store.
    ///
    /// # Arguments
    /// * `update` - Verified optimistic update to apply
    fn apply_optimistic_update(&mut self, update: &OptimisticUpdate) {
        let new_checkpoint = apply_optimistic_update(&mut self.store, update);
        if new_checkpoint.is_some() {
            self.last_checkpoint = Some(B256::new(new_checkpoint.unwrap().0));
        }
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
    Bootstrap,
    OldB256,
    Vec<Update>,
    OptimisticUpdate,
    EthEvmInput,
) {
    let bootstrap_header: LightClientHeader = env::read();
    let bootstrap_current_sync_committee: SyncCommittee = env::read();
    let bootstrap_current_sync_committee_branch: Vec<OldB256> = env::read();

    let checkpoint: OldB256 = env::read();

    let finality_update_attested_header: LightClientHeader = env::read();
    let finality_update_sync_aggregate: SyncAggregate = env::read();
    let finality_update_signature_slot: u64 = env::read();

    let update_len: usize = env::read();
    let mut updates: Vec<Update> = Vec::new();
    for _ in 0..update_len {
        let update_attested_header: LightClientHeader = env::read();
        let update_next_sync_committee: SyncCommittee = env::read();
        let update_next_sync_committee_branch: Vec<OldB256> = env::read();
        let update_finalized_header: LightClientHeader = env::read();
        let update_finality_branch: Vec<OldB256> = env::read();
        let update_sync_aggregate: SyncAggregate = env::read();
        let update_signature_slot: u64 = env::read();

        let update = Update {
            attested_header: update_attested_header,
            next_sync_committee: update_next_sync_committee,
            next_sync_committee_branch: update_next_sync_committee_branch,
            finalized_header: update_finalized_header,
            finality_branch: update_finality_branch,
            sync_aggregate: update_sync_aggregate,
            signature_slot: update_signature_slot,
        };
        updates.push(update);
    }

    let bootstrap = Bootstrap {
        header: bootstrap_header,
        current_sync_committee: bootstrap_current_sync_committee,
        current_sync_committee_branch: bootstrap_current_sync_committee_branch,
    };

    let finality_update = OptimisticUpdate {
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

sol! {
    struct Journal {
        /// The proof data bytes
        bytes proof_data;
        /// The user's address
        address account;
        /// The asset's contract address
        address asset;
        /// trusted beacon root
        bytes32 checkpoint;
        /// slot of the last update
        uint64 slot_last_update;
        /// new checkpoint
        bytes32 new_checkpoint;
    }
}

/// Validates a proof data query using light client proofs.
///
/// # Arguments
/// * `chain_id` - The chain ID to validate against
/// * `account` - Account address to query
/// * `asset` - Contract address to query
/// * `env_input` - Ethereum environment input
/// * `_sequencer_commitment` - Optional sequencer commitment
/// * `_op_env_input` - Optional optimistic environment input
/// * `linking_blocks` - Chain of blocks for verification
///
/// # Details
///
/// Performs the following validations:
/// 1. Verifies the light client chain via sync committee
/// 2. Validates block linking and chain length
/// 3. Verifies beacon chain commitments
/// 4. Executes and validates the proof data query
///
/// Commits the results including proof data and checkpoints to the guest environment.
pub fn validate_get_proof_data_call(
    chain_id: u64,
    account: Address,
    asset: Address,
    env_input: EthEvmInput,
    _sequencer_commitment: Option<SequencerCommitment>,
    _op_env_input: Option<EthEvmInput>,
    linking_blocks: Vec<RlpHeader<ConsensusHeader>>,
) {
    let env = env_input.into_env();

    let contract = Contract::new(asset, &env);

    let call = IMaldaMarket::getProofDataCall {
        account: account,
        dstChainId: chain_id as u32,
    };
    let proof_data = contract.call_builder(&call).call()._0;

    let last_block = if linking_blocks.is_empty() {
        env.header().inner().clone()
    } else {
        linking_blocks[linking_blocks.len() - 1].clone()
    };

    let (bootstrap, checkpoint, updates, finality_update, beacon_input) =
        read_l1_chain_builder_input();

    let slot_last_update = finality_update.attested_header.beacon.slot;

    let (current_beacon_hash, new_checkpoint) =
        validate_ethereum_env_via_sync_committee(bootstrap, checkpoint, updates, finality_update);

    validate_chain_length(
        chain_id,
        env.header().seal(),
        linking_blocks,
        last_block.hash_slow(),
    );

    let env = beacon_input.into_env();
    let exec_commit = env.header().seal();
    let beacon_commit = env.commitment().digest;

    assert_eq!(
        beacon_commit, current_beacon_hash,
        "beacon commit doesnt correspond to current beacon hash"
    );
    assert_eq!(
        exec_commit,
        last_block.hash_slow(),
        "exec commit doesnt correspond to last block hash"
    );

    let journal = Journal {
        proof_data,
        account,
        asset,
        checkpoint: B256::new(checkpoint.0),
        slot_last_update,
        new_checkpoint,
    };
    env::commit_slice(&journal.abi_encode());
}

/// Validates Ethereum environment using sync committee proofs.
///
/// # Arguments
/// * `bootstrap` - Initial bootstrap data
/// * `checkpoint` - Trust checkpoint
/// * `updates` - Sequence of light client updates
/// * `optimistic_update` - Latest optimistic update
///
/// # Returns
/// Tuple of (current beacon root, new checkpoint)
pub fn validate_ethereum_env_via_sync_committee(
    bootstrap: Bootstrap,
    checkpoint: OldB256,
    updates: Vec<Update>,
    optimistic_update: OptimisticUpdate,
) -> (B256, B256) {
    let mut l1_chain_builder = L1ChainBuilder::new();
    let verified_root = l1_chain_builder
        .build_beacon_chain(bootstrap, checkpoint, updates, optimistic_update)
        .unwrap();

    let verified_root = B256::new(verified_root.0);

    let new_checkpoint = l1_chain_builder
        .last_checkpoint
        .map_or_else(|| B256::from(checkpoint.0), |last| B256::new(last.0));

    (verified_root, new_checkpoint)
}

/// Validates chain length and block linking.
///
/// # Arguments
/// * `chain_id` - Chain ID to determine reorg protection depth
/// * `historical_hash` - Starting block hash
/// * `linking_blocks` - Chain of blocks to verify
/// * `current_hash` - Expected final block hash
///
/// # Panics
/// * If chain length is insufficient for reorg protection
/// * If blocks are not properly linked
/// * If final hash doesn't match expected hash
pub fn validate_chain_length(
    chain_id: u64,
    historical_hash: B256,
    linking_blocks: Vec<RlpHeader<ConsensusHeader>>,
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
