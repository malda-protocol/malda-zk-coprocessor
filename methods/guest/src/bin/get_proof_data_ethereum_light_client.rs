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

use malda_utils::{validators_ethereum_light_client::validate_get_proof_data_call as validate_get_proof_data_call_ethereum_light_client, types::SequencerCommitment};
use alloy_primitives::Address;
use risc0_steel::{ethereum::EthEvmInput, serde::RlpHeader};
use risc0_zkvm::guest::env;
use alloy_consensus::Header;

fn main() {
    // Read the input data for this application.
    let env_input: EthEvmInput = env::read();
    let chain_id: u64 = env::read();
    let account: Address = env::read();
    let asset: Address = env::read();
    let sequencer_commitment: Option<SequencerCommitment> = env::read();
    let env_op_input: Option<EthEvmInput> = env::read();
    let linking_blocks: Vec<RlpHeader<Header>> = env::read();

    validate_get_proof_data_call_ethereum_light_client(chain_id, account, asset, env_input, sequencer_commitment, env_op_input, linking_blocks);
} 