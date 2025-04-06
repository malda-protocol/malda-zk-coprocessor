// Copyright notice...

use malda_utils::{validators::validate_get_proof_data_call, types::SequencerCommitment};
use alloy_primitives::{Address, Bytes};
use risc0_steel::{ethereum::EthEvmInput, serde::RlpHeader};
use risc0_op_steel::optimism::OpEvmInput;
use risc0_zkvm::guest::env;
use alloy_consensus::Header;
use alloy_sol_types::SolValue;

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

        validate_get_proof_data_call(chain_id, account, asset, target_chain_ids, env_input, sequencer_commitment, env_op_input, &linking_blocks, &mut output, &env_eth_input, op_evm_input);
    }
    env::commit_slice(&output.abi_encode());
} 