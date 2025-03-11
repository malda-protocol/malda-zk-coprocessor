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