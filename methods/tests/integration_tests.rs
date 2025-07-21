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

#[cfg(test)]
mod tests {

    use alloy_primitives::{address, Address, Bytes, U256};
    use hex;
    use malda_rs::{
        constants::*, viewcalls::get_proof_data_exec, viewcalls::get_proof_data_prove_boundless,
    };

    use alloy::sol_types::SolValue;

    // HELPER CONSTANTS AND FUNCTIONS
    /////////////////////////////////

    pub const WETH_MARKET_SEPOLIA: Address = address!("D4286cc562b906589f8232335413f79d9aD42f7E");
    pub const WETH_MARKET: Address = address!("C7Bc6bD45Eb84D594f51cED3c5497E6812C7732f");

    pub const TEST_USER: Address = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");

    #[derive(Debug)]
    struct JournalEntry {
        sender: Address,
        market: Address,
        _acc_amount_in: U256,
        _acc_amount_out: U256,
        chain_id: u32,
        dst_chain_id: u32,
        l1_inclusion: bool,
    }

    fn decode_journal(journal_data: &[u8]) -> Result<JournalEntry, &'static str> {
        // Constants for journal entry size (113 bytes total)
        const ENTRY_SIZE: usize = 113;
        const SENDER_OFFSET: usize = 0;
        const SENDER_LENGTH: usize = 20;
        const MARKET_OFFSET: usize = 20;
        const MARKET_LENGTH: usize = 20;
        const ACC_AMOUNT_IN_OFFSET: usize = 40;
        const ACC_AMOUNT_IN_LENGTH: usize = 32;
        const ACC_AMOUNT_OUT_OFFSET: usize = 72;
        const ACC_AMOUNT_OUT_LENGTH: usize = 32;
        const CHAIN_ID_OFFSET: usize = 104;
        const CHAIN_ID_LENGTH: usize = 4;
        const DST_CHAIN_ID_OFFSET: usize = 108;
        const DST_CHAIN_ID_LENGTH: usize = 4;
        const L1_INCLUSION_OFFSET: usize = 112;
        // Check if the journal data has the correct length
        if journal_data.len() != ENTRY_SIZE {
            return Err("Invalid journal data length");
        }

        // Decode sender address (20 bytes)
        let sender_bytes = &journal_data[SENDER_OFFSET..SENDER_OFFSET + SENDER_LENGTH];
        let sender = Address::from_slice(sender_bytes);

        // Decode market address (20 bytes)
        let market_bytes = &journal_data[MARKET_OFFSET..MARKET_OFFSET + MARKET_LENGTH];
        let market = Address::from_slice(market_bytes);

        // Decode acc_amount_in (32 bytes)
        let acc_amount_in_bytes =
            &journal_data[ACC_AMOUNT_IN_OFFSET..ACC_AMOUNT_IN_OFFSET + ACC_AMOUNT_IN_LENGTH];
        let _acc_amount_in = U256::from_be_slice(acc_amount_in_bytes.try_into().unwrap());

        // Decode acc_amount_out (32 bytes)
        let acc_amount_out_bytes =
            &journal_data[ACC_AMOUNT_OUT_OFFSET..ACC_AMOUNT_OUT_OFFSET + ACC_AMOUNT_OUT_LENGTH];
        let _acc_amount_out = U256::from_be_slice(acc_amount_out_bytes.try_into().unwrap());

        // Decode chain_id (4 bytes)
        let chain_id_bytes = &journal_data[CHAIN_ID_OFFSET..CHAIN_ID_OFFSET + CHAIN_ID_LENGTH];
        let chain_id = u32::from_be_bytes(chain_id_bytes.try_into().unwrap());

        // Decode dst_chain_id (4 bytes)
        let dst_chain_id_bytes =
            &journal_data[DST_CHAIN_ID_OFFSET..DST_CHAIN_ID_OFFSET + DST_CHAIN_ID_LENGTH];
        let dst_chain_id = u32::from_be_bytes(dst_chain_id_bytes.try_into().unwrap());

        // Decode l1_inclusion (1 byte)
        let l1_inclusion_byte = journal_data[L1_INCLUSION_OFFSET];
        if l1_inclusion_byte != 0 && l1_inclusion_byte != 1 {
            return Err("Invalid L1 inclusion value");
        }
        let l1_inclusion = l1_inclusion_byte == 1;

        Ok(JournalEntry {
            sender,
            market,
            _acc_amount_in,
            _acc_amount_out,
            chain_id,
            dst_chain_id,
            l1_inclusion,
        })
    }

    async fn test_get_proof_data_with_params(
        users: Vec<Vec<Address>>,
        assets: Vec<Vec<Address>>,
        dst_chain_ids: Vec<Vec<u64>>,
        chain_ids: Vec<u64>,
        l1_inclusion: bool,
        fallback: bool,
        boundless: bool,
        onchain: bool,
    ) {
        // Assert that all input vectors have the same length
        let expected_length = users.len();
        assert_eq!(
            assets.len(),
            expected_length,
            "Assets vector length should match users vector length"
        );
        assert_eq!(
            dst_chain_ids.len(),
            expected_length,
            "Dst chain IDs vector length should match users vector length"
        );
        assert_eq!(
            chain_ids.len(),
            expected_length,
            "Chain IDs vector length should match users vector length"
        );

        // Assert that each inner vector has the same length
        for i in 0..expected_length {
            let user_count = users[i].len();
            assert_eq!(
                assets[i].len(),
                user_count,
                "Assets inner vector length should match users inner vector length at index {}",
                i
            );
            assert_eq!(dst_chain_ids[i].len(), user_count, "Dst chain IDs inner vector length should match users inner vector length at index {}", i);
        }

        if boundless {
            // Initialize basic logging for the boundless test
            println!("=== Starting Boundless Proof Test ===");

            println!("Starting boundless proof test with parameters:");
            println!("  Users: {:?}", users);
            println!("  Assets: {:?}", assets);
            println!("  Dst chain IDs: {:?}", dst_chain_ids);
            println!("  Chain IDs: {:?}", chain_ids);
            println!("  L1 inclusion: {}", l1_inclusion);
            println!("  Fallback: {}", fallback);

            let (journal, seal) = get_proof_data_prove_boundless(
                users.clone(),
                assets.clone(),
                dst_chain_ids.clone(),
                chain_ids.clone(),
                l1_inclusion,
                fallback,
                onchain,
            )
            .await
            .unwrap();

            println!("Boundless proof completed successfully!");
            println!("Journal length: {} bytes", journal.len());
            println!("Seal length: {} bytes", seal.len());
            println!("Journal (hex): 0x{}", hex::encode(&journal));
            println!("Seal (hex): 0x{}", hex::encode(&seal));

            // Decode and print journal entries
            let journals: Vec<Bytes> =
                <Vec<Bytes>>::abi_decode(&journal.0).expect("Failed to decode journal");

            println!("Decoded {} journal entries", journals.len());

            // Decode and print each journal entry
            let mut journal_index = 0;
            for (outer_idx, user_vec) in users.iter().enumerate() {
                for (inner_idx, user) in user_vec.iter().enumerate() {
                    let entry = decode_journal(&journals[journal_index]).unwrap();

                    println!(
                        "Journal entry {}: [{}, {}]",
                        journal_index, outer_idx, inner_idx
                    );
                    println!("  Sender: {:?}", entry.sender);
                    println!("  Market: {:?}", entry.market);
                    println!("  Chain ID: {}", entry.chain_id);
                    println!("  Dst Chain ID: {}", entry.dst_chain_id);
                    println!("  L1 Inclusion: {}", entry.l1_inclusion);

                    // Assert that all fields match the expected values for this entry
                    assert_eq!(
                        entry.sender, *user,
                        "Sender should match the test user at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.market, assets[outer_idx][inner_idx],
                        "Market should match the test asset at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.chain_id, chain_ids[outer_idx] as u32,
                        "Chain ID should match the test chain ID at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.dst_chain_id, dst_chain_ids[outer_idx][inner_idx] as u32,
                        "Dst chain ID should match the test dst chain ID at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.l1_inclusion, l1_inclusion,
                        "L1 inclusion should match the test parameter at position [{}, {}]",
                        outer_idx, inner_idx
                    );

                    journal_index += 1;
                }
            }

            println!("All journal entries validated successfully!");
        } else {
            let session_info = get_proof_data_exec(
                users.clone(),
                assets.clone(),
                dst_chain_ids.clone(),
                chain_ids.clone(),
                l1_inclusion,
                fallback,
            )
            .await
            .unwrap();

            let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
            println!("journal: 0x{}", hex::encode(&session_info.journal));
            println!("Cycles: {}", cycles);

            let journals: Vec<Bytes> = <Vec<Bytes>>::abi_decode(&session_info.journal.bytes)
                .expect("Failed to decode journal");

            // Assert that we have the expected number of journal entries
            let total_expected_entries: usize = users.iter().map(|user_vec| user_vec.len()).sum();
            assert_eq!(
                journals.len(),
                total_expected_entries,
                "Expected {} journal entries",
                total_expected_entries
            );

            // Decode and assert each journal entry
            let mut journal_index = 0;
            for (outer_idx, user_vec) in users.iter().enumerate() {
                for (inner_idx, user) in user_vec.iter().enumerate() {
                    let entry = decode_journal(&journals[journal_index]).unwrap();

                    // Assert that all fields match the expected values for this entry
                    assert_eq!(
                        entry.sender, *user,
                        "Sender should match the test user at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.market, assets[outer_idx][inner_idx],
                        "Market should match the test asset at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.chain_id, chain_ids[outer_idx] as u32,
                        "Chain ID should match the test chain ID at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.dst_chain_id, dst_chain_ids[outer_idx][inner_idx] as u32,
                        "Dst chain ID should match the test dst chain ID at position [{}, {}]",
                        outer_idx, inner_idx
                    );
                    assert_eq!(
                        entry.l1_inclusion, l1_inclusion,
                        "L1 inclusion should match the test parameter at position [{}, {}]",
                        outer_idx, inner_idx
                    );

                    journal_index += 1;
                }
            }
        }
    }

    // BOUNDLESS TESTS
    ///////////////////

    #[tokio::test]
    #[ignore]
    async fn test_prove_get_proof_data_boundless_on_linea() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![LINEA_CHAIN_ID],
            false,
            false,
            true,
            true, // onchain = false for offchain submission
        )
        .await;
    }

    // MAINNET TESTS
    ////////////////

    #[tokio::test]
    async fn test_exec_get_proof_data_on_linea() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![LINEA_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_linea_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![LINEA_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_linea_with_l1_inclusion() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![LINEA_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_linea_with_l1_inclusion_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![LINEA_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_base() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![BASE_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_base_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![BASE_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_base_with_l1_inclusion() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![BASE_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_base_with_l1_inclusion_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![BASE_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_ethereum() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![ETHEREUM_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_on_ethereum_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![ETHEREUM_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "L1 Inclusion only supported for Optimism, Base, Linea and their Sepolia variants"
    )]
    async fn test_exec_get_proof_data_on_ethereum_with_l1_inclusion() {
        // This test should fail because L1 inclusion is not supported for Ethereum
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![ETHEREUM_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "L1 Inclusion only supported for Optimism, Base, Linea and their Sepolia variants"
    )]
    async fn test_exec_get_proof_data_on_ethereum_with_l1_inclusion_fallback() {
        // This test should fail because L1 inclusion is not supported for Ethereum
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![ETHEREUM_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_multiple_users_multiple_chains() {
        // Test multiple users across multiple chains
        let user1 = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let user2 = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let user3 = address!("6446021F4E396dA3df4235C62537431372195D38");
        let user4 = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");

        test_get_proof_data_with_params(
            vec![
                vec![user1, user2], // Linea users
                vec![user3],        // Base user
                vec![user4],        // Ethereum user
            ],
            vec![
                vec![WETH_MARKET, WETH_MARKET], // Linea assets
                vec![WETH_MARKET],              // Base asset
                vec![WETH_MARKET],              // Ethereum asset
            ],
            vec![
                vec![OPTIMISM_CHAIN_ID, OPTIMISM_CHAIN_ID], // Linea destinations
                vec![OPTIMISM_CHAIN_ID],                    // Base destination
                vec![LINEA_CHAIN_ID],                       // Ethereum destination
            ],
            vec![
                LINEA_CHAIN_ID,    // Linea chain ID
                BASE_CHAIN_ID,     // Base chain ID
                ETHEREUM_CHAIN_ID, // Ethereum chain ID
            ],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    async fn test_exec_get_proof_data_multiple_users_multiple_chains_fallback() {
        // Test multiple users across multiple chains with fallback
        let user1 = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let user2 = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let user3 = address!("6446021F4E396dA3df4235C62537431372195D38");
        let user4 = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");

        test_get_proof_data_with_params(
            vec![
                vec![user1, user2], // Linea users
                vec![user3],        // Base user
                vec![user4],        // Ethereum user
            ],
            vec![
                vec![WETH_MARKET, WETH_MARKET], // Linea assets
                vec![WETH_MARKET],              // Base asset
                vec![WETH_MARKET],              // Ethereum asset
            ],
            vec![
                vec![OPTIMISM_CHAIN_ID, OPTIMISM_CHAIN_ID], // Linea destinations
                vec![OPTIMISM_CHAIN_ID],                    // Base destination
                vec![LINEA_CHAIN_ID],                       // Ethereum destination
            ],
            vec![
                LINEA_CHAIN_ID,    // Linea chain ID
                BASE_CHAIN_ID,     // Base chain ID
                ETHEREUM_CHAIN_ID, // Ethereum chain ID
            ],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    // SEPOLIA TESTS
    ////////////////
    // Note: These tests require Sepolia Sequencer Commitment Access to pass
    // They may fail with connection errors if the sequencer is not accessible

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_linea() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![LINEA_SEPOLIA_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_linea_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![LINEA_SEPOLIA_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_linea_with_l1_inclusion() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![LINEA_SEPOLIA_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_linea_with_l1_inclusion_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![LINEA_SEPOLIA_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_base() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![BASE_SEPOLIA_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_base_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![BASE_SEPOLIA_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_base_with_l1_inclusion() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![BASE_SEPOLIA_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_base_with_l1_inclusion_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![OPTIMISM_SEPOLIA_CHAIN_ID]],
            vec![BASE_SEPOLIA_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_optimism() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![OPTIMISM_SEPOLIA_CHAIN_ID],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_optimism_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![OPTIMISM_SEPOLIA_CHAIN_ID],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_optimism_with_l1_inclusion() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![OPTIMISM_SEPOLIA_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_optimism_with_l1_inclusion_fallback() {
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![OPTIMISM_SEPOLIA_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "L1 Inclusion only supported for Optimism, Base, Linea and their Sepolia variants"
    )]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_ethereum_with_l1_inclusion() {
        // This test should fail because L1 inclusion is not supported for Ethereum
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![ETHEREUM_SEPOLIA_CHAIN_ID],
            true,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[should_panic(
        expected = "L1 Inclusion only supported for Optimism, Base, Linea and their Sepolia variants"
    )]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_on_ethereum_with_l1_inclusion_fallback() {
        // This test should fail because L1 inclusion is not supported for Ethereum
        test_get_proof_data_with_params(
            vec![vec![TEST_USER]],
            vec![vec![WETH_MARKET_SEPOLIA]],
            vec![vec![LINEA_SEPOLIA_CHAIN_ID]],
            vec![ETHEREUM_SEPOLIA_CHAIN_ID],
            true,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_multiple_users_multiple_chains() {
        // Test multiple users across multiple Sepolia chains
        let user1 = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let user2 = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let user3 = address!("6446021F4E396dA3df4235C62537431372195D38");
        let user4 = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");

        test_get_proof_data_with_params(
            vec![
                vec![user1, user2], // Linea Sepolia users
                vec![user3],        // Base Sepolia user
                vec![user4],        // Optimism Sepolia user
            ],
            vec![
                vec![WETH_MARKET_SEPOLIA, WETH_MARKET_SEPOLIA], // Linea Sepolia assets
                vec![WETH_MARKET_SEPOLIA],                      // Base Sepolia asset
                vec![WETH_MARKET_SEPOLIA],                      // Optimism Sepolia asset
            ],
            vec![
                vec![OPTIMISM_SEPOLIA_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID], // Linea Sepolia destinations
                vec![OPTIMISM_SEPOLIA_CHAIN_ID], // Base Sepolia destination
                vec![LINEA_SEPOLIA_CHAIN_ID],    // Optimism Sepolia destination
            ],
            vec![
                LINEA_SEPOLIA_CHAIN_ID,    // Linea Sepolia chain ID
                BASE_SEPOLIA_CHAIN_ID,     // Base Sepolia chain ID
                OPTIMISM_SEPOLIA_CHAIN_ID, // Optimism Sepolia chain ID
            ],
            false,
            false,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_sepolia_exec_get_proof_data_multiple_users_multiple_chains_fallback() {
        // Test multiple users across multiple Sepolia chains with fallback
        let user1 = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let user2 = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let user3 = address!("6446021F4E396dA3df4235C62537431372195D38");
        let user4 = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");

        test_get_proof_data_with_params(
            vec![
                vec![user1, user2], // Linea Sepolia users
                vec![user3],        // Base Sepolia user
                vec![user4],        // Optimism Sepolia user
            ],
            vec![
                vec![WETH_MARKET_SEPOLIA, WETH_MARKET_SEPOLIA], // Linea Sepolia assets
                vec![WETH_MARKET_SEPOLIA],                      // Base Sepolia asset
                vec![WETH_MARKET_SEPOLIA],                      // Optimism Sepolia asset
            ],
            vec![
                vec![OPTIMISM_SEPOLIA_CHAIN_ID, OPTIMISM_SEPOLIA_CHAIN_ID], // Linea Sepolia destinations
                vec![OPTIMISM_SEPOLIA_CHAIN_ID], // Base Sepolia destination
                vec![LINEA_SEPOLIA_CHAIN_ID],    // Optimism Sepolia destination
            ],
            vec![
                LINEA_SEPOLIA_CHAIN_ID,    // Linea Sepolia chain ID
                BASE_SEPOLIA_CHAIN_ID,     // Base Sepolia chain ID
                OPTIMISM_SEPOLIA_CHAIN_ID, // Optimism Sepolia chain ID
            ],
            false,
            true,
            false,
            false, // onchain = false for exec tests
        )
        .await;
    }
}
