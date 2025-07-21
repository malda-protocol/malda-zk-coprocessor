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

use malda_utils::{validators::validate_get_proof_data_call, types::SequencerCommitment};
use alloy_primitives::{Address, Bytes};
use risc0_steel::{ethereum::EthEvmInput, serde::RlpHeader};
use risc0_op_steel::optimism::OpEvmInput;
use risc0_zkvm::guest::env;
use alloy_consensus::Header;
use alloy_sol_types::SolValue;
use malda_utils::constants::{LINEA_CHAIN_ID, BASE_CHAIN_ID, ETHEREUM_CHAIN_ID, LINEA_SEPOLIA_CHAIN_ID, BASE_SEPOLIA_CHAIN_ID, ETHEREUM_SEPOLIA_CHAIN_ID, OPTIMISM_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID};

fn main() {
    let mut output: Vec<Bytes> = Vec::new();
    let length: u64 = env::read();
    for _i in 0..length {
        // Read the input data for this application.
        let env_input: Option<EthEvmInput> = env::read();
        let chain_id: u64 = env::read();
        let account: Vec<Address> = env::read();
        let asset: Vec<Address> = env::read();
        let target_chain_ids: Vec<u64> = env::read();
        let sequencer_commitment: Option<SequencerCommitment> = env::read();
        let env_op_input: Option<EthEvmInput> = env::read();
        let linking_blocks: Vec<RlpHeader<Header>> = env::read();
        let env_eth_input: Option<EthEvmInput> = env::read();
        let op_evm_input: Option<OpEvmInput> = env::read();
        let sequencer_commitment_opstack_2: Option<SequencerCommitment> = env::read();
        let env_op_input_2: Option<EthEvmInput> = env::read();

        // This makes the guest program only compatible with mainnet chains, remove for testnet and enable the below testnet code
        if chain_id != LINEA_CHAIN_ID && chain_id != BASE_CHAIN_ID && chain_id != ETHEREUM_CHAIN_ID && chain_id != OPTIMISM_CHAIN_ID {
            panic!("Chain ID is not Linea, Base, Ethereum or Optimism");
        }

        // This makes the guest program only compatible with testnet chains, remove for mainnet and enable the above mainnet code
        // if chain_id != LINEA_SEPOLIA_CHAIN_ID && chain_id != BASE_SEPOLIA_CHAIN_ID && chain_id != ETHEREUM_SEPOLIA_CHAIN_ID && chain_id != OPTIMISM_SEPOLIA_CHAIN_ID {
        //     panic!("Chain ID is not Linea Sepolia, Base Sepolia, Ethereum Sepolia or Optimism Sepolia");
        // }
        
        validate_get_proof_data_call(chain_id, account, asset, target_chain_ids, env_input, sequencer_commitment, env_op_input, &linking_blocks, &mut output, &env_eth_input, op_evm_input, sequencer_commitment_opstack_2, env_op_input_2);
    }
    env::commit_slice(&output.abi_encode());
} 
