//! Integration tests for malda-rs validation and view call functionality.
//!
//! This module tests:
//! - Linea environment validation
//! - OpStack (Optimism/Base) environment validation
//! - Chain length validation for reorg protection
//! - Cross-chain balance query inputs

#[cfg(test)]
mod tests {

    use alloy::{
        eips::BlockNumberOrTag,
        providers::{Provider, ProviderBuilder},
        transports::http::reqwest::Url,
    };
    use malda_rs::{constants::*, types::*, validators::*, viewcalls::*};
    use risc0_steel::{ethereum::EthEvmEnv, host::BlockNumberOrTag as BlockRisc0};

    /// Tests Linea header verification with correct input parameters
    ///
    /// # Test Steps
    /// 1. Fetches latest block from Linea
    /// 2. Fetches beacon data for the block
    /// 3. Verifies the header using linea_block_verifier
    ///
    /// # Expected Outcome
    /// - No panic occurs with valid input
    #[tokio::test]
    async fn test_validate_linea_env_correct_input() {
        let env = EthEvmEnv::builder()
            .rpc(Url::parse(get_rpc_url("LINEA", false, false)).unwrap())
            .block_number_or_tag(BlockRisc0::Latest)
            .chain_spec(&LINEA_MAINNET_CHAIN_SPEC)
            .build()
            .await
            .unwrap();

        let block_number = env.header().inner().inner().number;
        let untrusted_header = env.header().inner().inner();

        // Fetch beacon data for verification
        let beacon_url = get_beacon_api_url("LINEA", false, false);
        let network =
            linea_block_verifier::core::constants::LineaNetwork::try_from(LINEA_CHAIN_ID).unwrap();
        let beacon_client = linea_block_verifier::host::BeaconClient::new(beacon_url, network);
        let beacon_data = beacon_client.fetch_beacon_data(block_number).await.unwrap();

        // Verify the header
        let result =
            linea_block_verifier::core::verify_header(untrusted_header, &beacon_data, network);
        assert!(result.is_ok());
    }

    /// Tests Linea header verification with wrong chain input
    ///
    /// # Test Steps
    /// 1. Fetches latest block from Optimism (wrong chain)
    /// 2. Fetches Linea beacon data
    /// 3. Attempts to verify Optimism header as Linea
    ///
    /// # Expected Outcome
    /// - Verification fails due to chain mismatch
    #[tokio::test]
    async fn test_validate_linea_env_input_of_wrong_chain_panics() {
        // Fetch a block from Optimism (wrong chain)
        let env = EthEvmEnv::builder()
            .rpc(Url::parse(get_rpc_url("OPTIMISM", false, false)).unwrap())
            .block_number_or_tag(BlockRisc0::Latest)
            .chain_spec(&LINEA_MAINNET_CHAIN_SPEC)
            .build()
            .await
            .unwrap();

        let untrusted_header = env.header().inner().inner();

        // Fetch Linea beacon data (using a recent Linea block)
        let linea_env = EthEvmEnv::builder()
            .rpc(Url::parse(get_rpc_url("LINEA", false, false)).unwrap())
            .block_number_or_tag(BlockRisc0::Latest)
            .chain_spec(&LINEA_MAINNET_CHAIN_SPEC)
            .build()
            .await
            .unwrap();
        let linea_block_number = linea_env.header().inner().inner().number;

        let beacon_url = get_beacon_api_url("LINEA", false, false);
        let network =
            linea_block_verifier::core::constants::LineaNetwork::try_from(LINEA_CHAIN_ID).unwrap();
        let beacon_client = linea_block_verifier::host::BeaconClient::new(beacon_url, network);
        let beacon_data = beacon_client
            .fetch_beacon_data(linea_block_number)
            .await
            .unwrap();

        // Verify should fail - Optimism header doesn't match Linea beacon data
        let result =
            linea_block_verifier::core::verify_header(untrusted_header, &beacon_data, network);
        assert!(result.is_err());
    }

    /// Tests Linea header verification with manipulated block data
    ///
    /// # Test Steps
    /// 1. Fetches latest block from Linea
    /// 2. Manipulates block number
    /// 3. Attempts verification
    ///
    /// # Expected Outcome
    /// - Verification fails due to block manipulation
    #[tokio::test]
    async fn test_validate_linea_env_input_manipulated_panics() {
        let env = EthEvmEnv::builder()
            .rpc(Url::parse(get_rpc_url("LINEA", false, false)).unwrap())
            .block_number_or_tag(BlockRisc0::Latest)
            .chain_spec(&LINEA_MAINNET_CHAIN_SPEC)
            .build()
            .await
            .unwrap();

        let block_number = env.header().inner().inner().number;

        // Fetch beacon data for the original block
        let beacon_url = get_beacon_api_url("LINEA", false, false);
        let network =
            linea_block_verifier::core::constants::LineaNetwork::try_from(LINEA_CHAIN_ID).unwrap();
        let beacon_client = linea_block_verifier::host::BeaconClient::new(beacon_url, network);
        let beacon_data = beacon_client.fetch_beacon_data(block_number).await.unwrap();

        // Manipulate the header
        let mut manipulated_header = env.header().inner().inner().clone();
        manipulated_header.number = 1;

        // Verify should fail - manipulated header doesn't match beacon data
        let result = linea_block_verifier::core::verify_header(
            &manipulated_header,
            &beacon_data,
            network,
        );
        assert!(result.is_err());
    }

    /// Tests OpStack environment validation with correct input
    ///
    /// # Test Steps
    /// 1. Fetches current sequencer commitment
    /// 2. Gets corresponding block hash
    /// 3. Validates OpStack environment
    ///
    /// # Expected Outcome
    /// - No panic occurs with valid input
    #[tokio::test]
    async fn test_validate_optimism_env_correct_input() {
        let (sequencer_commitment, block) =
            get_current_sequencer_commitment(OPTIMISM_CHAIN_ID, false).await;

        let http_url: Url = get_rpc_url("OPTIMISM", false, false).parse().unwrap();

        let provider = ProviderBuilder::new().connect_http(http_url);
        let correct_hash = provider
            .get_block_by_number(BlockNumberOrTag::Number(block))
            .await
            .unwrap()
            .unwrap()
            .header
            .hash;

        validate_opstack_env(OPTIMISM_CHAIN_ID, &sequencer_commitment, correct_hash);
    }

    /// Tests OpStack environment validation with incorrect block hash
    ///
    /// # Test Steps
    /// 1. Fetches current sequencer commitment
    /// 2. Gets hash from wrong block
    /// 3. Attempts validation
    ///
    /// # Expected Outcome
    /// - Panics due to hash mismatch
    #[tokio::test]
    async fn test_validate_optimism_env_wrong_hash_panics() {
        let (sequencer_commitment, block) =
            get_current_sequencer_commitment(OPTIMISM_CHAIN_ID, false).await;

        let http_url: Url = get_rpc_url("OPTIMISM", false, false).parse().unwrap();

        let provider = ProviderBuilder::new().connect_http(http_url);

        // get hash of previous block here
        let wrong_hash = provider
            .get_block_by_number(BlockNumberOrTag::Number(block - 1))
            .await
            .unwrap()
            .unwrap()
            .header
            .hash;

        assert!(std::panic::catch_unwind(|| {
            validate_opstack_env(OPTIMISM_CHAIN_ID, &sequencer_commitment, wrong_hash);
        })
        .is_err());
    }

    /// Tests OpStack environment validation with incorrect chain ID
    ///
    /// # Test Steps
    /// 1. Fetches current sequencer commitment
    /// 2. Gets correct block hash
    /// 3. Attempts validation with wrong chain ID
    ///
    /// # Expected Outcome
    /// - Panics due to chain ID mismatch
    #[tokio::test]
    async fn test_validate_optimism_env_wrong_chain_id_panics() {
        let (sequencer_commitment, block) =
            get_current_sequencer_commitment(OPTIMISM_CHAIN_ID, false).await;

        let http_url: Url = get_rpc_url("OPTIMISM", false, false).parse().unwrap();

        let provider = ProviderBuilder::new().connect_http(http_url);

        // get hash of previous block here
        let correct_hash = provider
            .get_block_by_number(BlockNumberOrTag::Number(block))
            .await
            .unwrap()
            .unwrap()
            .header
            .hash;

        assert!(std::panic::catch_unwind(|| {
            validate_opstack_env(OPTIMISM_CHAIN_ID + 1, &sequencer_commitment, correct_hash);
        })
        .is_err());
    }

    /// Tests OpStack environment validation with wrong commitment
    ///
    /// # Test Steps
    /// 1. Fetches Base commitment instead of Optimism
    /// 2. Gets correct block hash
    /// 3. Attempts validation
    ///
    /// # Expected Outcome
    /// - Panics due to commitment mismatch
    #[tokio::test]
    async fn test_validate_optimism_env_wrong_commitment_panics() {
        // get commitment from base chain here
        let (sequencer_commitment, block) =
            get_current_sequencer_commitment(BASE_CHAIN_ID, false).await;

        let http_url: Url = get_rpc_url("OPTIMISM", false, false).parse().unwrap();

        let provider = ProviderBuilder::new().connect_http(http_url);

        // get hash of previous block here
        let correct_hash = provider
            .get_block_by_number(BlockNumberOrTag::Number(block))
            .await
            .unwrap()
            .unwrap()
            .header
            .hash;

        assert!(std::panic::catch_unwind(|| {
            validate_opstack_env(OPTIMISM_CHAIN_ID, &sequencer_commitment, correct_hash);
        })
        .is_err());
    }

    /// Tests OpStack environment validation with manipulated commitment
    ///
    /// # Test Steps
    /// 1. Fetches commitments from both Optimism and Base
    /// 2. Creates manipulated commitments by mixing data
    /// 3. Attempts validation with manipulated data
    ///
    /// # Expected Outcome
    /// - Panics for both signature and data manipulation
    #[tokio::test]
    async fn test_validate_optimism_env_manipulated_commitment_panics() {
        let (sequencer_commitment, _block) =
            get_current_sequencer_commitment(OPTIMISM_CHAIN_ID, false).await;

        let (wrong_sequencer_commitment, block) =
            get_current_sequencer_commitment(BASE_CHAIN_ID, false).await;

        let mut manipulated_commitment_signature = sequencer_commitment.clone();
        manipulated_commitment_signature.signature = wrong_sequencer_commitment.signature;

        let mut manipulated_commitment_data = sequencer_commitment.clone();
        manipulated_commitment_data.data = wrong_sequencer_commitment.data;

        let http_url: Url = get_rpc_url("OPTIMISM", false, false).parse().unwrap();

        let provider = ProviderBuilder::new().connect_http(http_url);

        // get hash of previous block here
        let correct_hash = provider
            .get_block_by_number(BlockNumberOrTag::Number(block))
            .await
            .unwrap()
            .unwrap()
            .header
            .hash;

        // fails when either signature or data has been modified
        assert!(std::panic::catch_unwind(|| {
            validate_opstack_env(
                OPTIMISM_CHAIN_ID,
                &manipulated_commitment_signature,
                correct_hash,
            );
        })
        .is_err());

        assert!(std::panic::catch_unwind(|| {
            validate_opstack_env(
                OPTIMISM_CHAIN_ID,
                &manipulated_commitment_data,
                correct_hash,
            );
        })
        .is_err());
    }

    /// Tests chain length validation with correct input
    ///
    /// # Test Steps
    /// 1. Gets linking blocks for specific block number
    /// 2. Validates chain length with correct parameters
    ///
    /// # Expected Outcome
    /// - No panic occurs with valid input
    #[tokio::test]
    async fn test_validate_chain_length_input_correct() {
        let block_number = 21193475;
        let linking_blocks = get_linking_blocks(
            ETHEREUM_CHAIN_ID,
            get_rpc_url("ETHEREUM", false, false),
            block_number,
        )
        .await;
        if linking_blocks.is_empty() {
            // No linking blocks needed when reorg protection is zero
            return;
        }
        let historical_hash = linking_blocks[0].inner().parent_hash;
        let current_hash = linking_blocks[linking_blocks.len() - 1].hash_slow();
        validate_chain_length(
            ETHEREUM_CHAIN_ID,
            historical_hash,
            &linking_blocks,
            current_hash,
        );
    }

    /// Tests chain length validation with insufficient blocks
    ///
    /// # Test Steps
    /// 1. Gets linking blocks
    /// 2. Removes blocks to make chain too short
    /// 3. Attempts validation
    ///
    /// # Expected Outcome
    /// - Panics due to insufficient chain length
    #[tokio::test]
    async fn test_validate_chain_length_panics_if_chain_too_short() {
        let block_number = 21193475;
        let linking_blocks = get_linking_blocks(
            ETHEREUM_CHAIN_ID,
            get_rpc_url("ETHEREUM", false, false),
            block_number,
        )
        .await;
        if linking_blocks.is_empty() {
            // No linking blocks needed when reorg protection is zero
            return;
        }
        let historical_hash = linking_blocks[0].inner().parent_hash;
        let current_hash = linking_blocks[linking_blocks.len() - 1].hash_slow();

        assert!(std::panic::catch_unwind(|| {
            validate_chain_length(
                ETHEREUM_CHAIN_ID,
                historical_hash,
                &linking_blocks[0..linking_blocks.len() - 2].to_vec(),
                current_hash,
            );
        })
        .is_err());
    }

    /// Tests chain length validation with mismatched hashes
    ///
    /// # Test Steps
    /// 1. Gets linking blocks
    /// 2. Uses wrong hash for validation
    /// 3. Attempts validation
    ///
    /// # Expected Outcome
    /// - Panics due to hash mismatch
    #[tokio::test]
    async fn test_validate_chain_length_panics_if_hash_doesnt_match() {
        let block_number = 21193475;
        let linking_blocks = get_linking_blocks(
            ETHEREUM_CHAIN_ID,
            get_rpc_url("ETHEREUM", false, false),
            block_number,
        )
        .await;
        if linking_blocks.is_empty() {
            // No linking blocks needed when reorg protection is zero
            return;
        }
        let historical_hash = linking_blocks[0].inner().parent_hash;

        assert!(std::panic::catch_unwind(|| {
            validate_chain_length(
                ETHEREUM_CHAIN_ID,
                historical_hash,
                &linking_blocks[0..linking_blocks.len() - 2].to_vec(),
                historical_hash,
            );
        })
        .is_err());
    }
}
