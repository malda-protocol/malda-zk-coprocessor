// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generated crate containing the image ID and ELF binary of the build guest.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {

    use alloy::{
        eips::BlockNumberOrTag,
        providers::{Provider, ProviderBuilder},
        transports::http::reqwest::Url,
    };
    use alloy_primitives::{address, Address, B256};
    use hex;
    use malda_rs::{
        constants::*,
        viewcalls::{
            get_current_sequencer_commitment, get_proof_data_exec, get_proof_data_prove,
            get_proof_data_prove_sdk,
        },
        viewcalls_ethereum_light_client::get_proof_data_exec as get_proof_data_exec_ethereum_light_client,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::{io::Write, time::Duration};

    // Common market tokens for all chains
    const MARKETS: [Address; 2] = [
        WETH_MARKET_SEPOLIA, // WETH Market Sepolia
        USDC_MARKET_SEPOLIA, // USDC Market Sepolia
    ];

    #[tokio::test]
    async fn prove_get_proof_data_on_linea() {
        let user_linea = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_linea]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("journal: 0x{}", hex::encode(&session_info.journal));
        println!("Cycles: {}", cycles);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_sepolia_get_proof_data_on_linea() {
        let user_linea = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let start_time = std::time::Instant::now();
        let session_info = get_proof_data_exec(
            vec![vec![user_linea]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("journal: 0x{}", hex::encode(&session_info.journal));
        println!("Cycles: {}", cycles);
        println!("Duration: {:?}", duration);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_optimism() {
        let user_optimism = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = OPTIMISM_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_optimism]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_optimism_sepolia() {
        let user_optimism = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = OPTIMISM_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_optimism]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_optimism_sepolia_slow_lane() {
        let user_optimism = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = OPTIMISM_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_optimism]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            true,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_optimism123() {
        let user_optimism = address!("6c7d89c32ead20F980AD76A33377550F3F72a338");
        let market = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_prove(
            vec![vec![user_optimism]],
            vec![vec![market]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        // let duration = start_time.elapsed();
        // println!("Duration: {:?}", duration);
        let cycles = session_info.stats.total_cycles / 1000;
        println!("KCycles: {}", cycles);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_optimism123_sdk() {
        let user_optimism = address!("6c7d89c32ead20F980AD76A33377550F3F72a338");
        let market = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_prove_sdk(
            vec![vec![user_optimism]],
            vec![vec![market]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        // let duration = start_time.elapsed();
        // println!("Duration: {:?}", duration);
        let cycles = session_info.stats.total_cycles / 1000;
        println!("KCycles: {}", cycles);
        panic!("test");
    }

    #[tokio::test]
    async fn prove_testnet_bonsai_state() {
        let user = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let market = WETH_MARKET_SEPOLIA;

        // Parameters for parallel execution
        let n_parallel = 20; // Number of parallel executions
        let delay_secs = [30]; // Delay between spawns in seconds

        for delay in delay_secs {
            // Write delay info to file
            let delay_info = format!("Delay between submissions: {} seconds\n\n", delay);
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("parallel_benchmark.txt")
                .unwrap()
                .write_all(delay_info.as_bytes())
                .unwrap();

            // Create shared atomic counter
            static ACTIVE_PROOFS: AtomicUsize = AtomicUsize::new(0);

            // Spawn parallel tasks
            for i in 0..n_parallel {
                let user = user.clone();
                let market = market.clone();

                tokio::spawn(async move {
                    let active_proofs = ACTIVE_PROOFS.fetch_add(1, Ordering::SeqCst);
                    let start_time = std::time::Instant::now();

                    let result = get_proof_data_prove(
                        vec![vec![user]],
                        vec![vec![market]],
                        vec![vec![OPTIMISM_CHAIN_ID]],
                        vec![OPTIMISM_SEPOLIA_CHAIN_ID],
                        false,
                    )
                    .await;

                    match result {
                        Ok(session_info) => {
                            let duration = start_time.elapsed();
                            let cycles = session_info.stats.total_cycles / 1000;
                            let log_entry = format!("Parallel proof {} - Cycles: {}, Duration: {:?}, Active proofs: {}\n", 
                                i, cycles, duration, active_proofs);

                            // Print to console
                            println!("{}", log_entry);

                            // Write to file
                            std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("parallel_benchmark.txt")
                                .unwrap()
                                .write_all(log_entry.as_bytes())
                                .unwrap();
                        }
                        Err(e) => {
                            let error_log = format!("Parallel proof {} failed: {:?}\n", i, e);
                            println!("{}", error_log);

                            // Write error to file
                            std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("parallel_benchmark.txt")
                                .unwrap()
                                .write_all(error_log.as_bytes())
                                .unwrap();
                        }
                    }
                    ACTIVE_PROOFS.fetch_sub(1, Ordering::SeqCst);
                });
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }

            tokio::time::sleep(Duration::from_secs(600)).await;
        }
    }

    #[tokio::test]
    async fn prove_get_proof_data_batch() {
        // Single chain test (Linea)
        let users = vec![vec![address!("Ad7f33984bed10518012013D4aB0458D37FEE6F3")]];
        let assets = vec![vec![WETH_MARKET_SEPOLIA]];
        let chain_ids = vec![LINEA_CHAIN_ID];
        let target_chain_ids = vec![vec![OPTIMISM_CHAIN_ID]];

        let session_info = get_proof_data_exec(users, assets, target_chain_ids, chain_ids, false)
            .await
            .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("SINGLE BALANCE CALL PER CHAIN");
        println!("Linea");
        println!("Cycles: {}", cycles);

        // Test with Linea + Optimism
        let users = vec![
            vec![address!("Ad7f33984bed10518012013D4aB0458D37FEE6F3")],
            vec![address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8")],
        ];
        let assets = vec![vec![WETH_MARKET_SEPOLIA], vec![WETH_MARKET_SEPOLIA]];
        let chain_ids = vec![LINEA_CHAIN_ID, OPTIMISM_CHAIN_ID];
        let target_chain_ids = vec![vec![OPTIMISM_CHAIN_ID], vec![LINEA_CHAIN_ID]];

        let session_info = get_proof_data_exec(users, assets, target_chain_ids, chain_ids, false)
            .await
            .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("\nLinea + Optimism");
        println!("Cycles: {}", cycles);

        // Test with Linea + Optimism + Base
        let users = vec![
            vec![address!("Ad7f33984bed10518012013D4aB0458D37FEE6F3")],
            vec![address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8")],
            vec![address!("6446021F4E396dA3df4235C62537431372195D38")],
        ];
        let assets = vec![
            vec![WETH_MARKET_SEPOLIA],
            vec![WETH_MARKET_SEPOLIA],
            vec![WETH_MARKET_SEPOLIA],
        ];
        let chain_ids = vec![LINEA_CHAIN_ID, OPTIMISM_CHAIN_ID, BASE_CHAIN_ID];
        let target_chain_ids = vec![
            vec![OPTIMISM_CHAIN_ID],
            vec![LINEA_CHAIN_ID],
            vec![OPTIMISM_CHAIN_ID],
        ];

        let session_info = get_proof_data_exec(users, assets, target_chain_ids, chain_ids, false)
            .await
            .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("\nLinea + Optimism + Base");
        println!("Cycles: {}", cycles);

        // Test with Linea + Optimism + Base + Ethereum
        let users = vec![
            vec![address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8")],
            vec![address!("6446021F4E396dA3df4235C62537431372195D38")],
            vec![address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E")],
        ];
        let assets = vec![
            vec![WETH_MARKET_SEPOLIA],
            vec![WETH_MARKET_SEPOLIA],
            vec![WETH_MARKET_SEPOLIA],
            vec![WETH_MARKET_SEPOLIA],
        ];
        let chain_ids = vec![
            LINEA_CHAIN_ID,
            OPTIMISM_CHAIN_ID,
            BASE_CHAIN_ID,
            ETHEREUM_CHAIN_ID,
        ];
        let target_chain_ids = vec![
            vec![OPTIMISM_CHAIN_ID],
            vec![LINEA_CHAIN_ID],
            vec![OPTIMISM_CHAIN_ID],
            vec![LINEA_CHAIN_ID],
        ];

        let session_info = get_proof_data_exec(users, assets, target_chain_ids, chain_ids, false)
            .await
            .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("\nLinea + Optimism + Base + Ethereum via OP");
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_batch_stats() {
        use rand::Rng;
        use std::time::Instant;

        let chain_ids = vec![
            LINEA_CHAIN_ID,
            OPTIMISM_CHAIN_ID,
            BASE_CHAIN_ID,
            ETHEREUM_CHAIN_ID,
        ];

        let available_assets = [&MARKETS; 4];

        // Run the test 5 times
        for iteration in 0..1 {
            println!("\nIteration {}", iteration + 1);

            let mut users = Vec::new();
            let mut assets = Vec::new();

            // Generate random data
            let mut rng = rand::thread_rng();
            for (idx, _chain_id) in chain_ids.iter().enumerate() {
                let size = 10;

                let chain_users: Vec<Address> = (0..size)
                    .map(|_| {
                        let random_bytes: [u8; 20] = rng.gen();
                        Address::from(random_bytes)
                    })
                    .collect();

                let chain_assets: Vec<Address> = (0..size)
                    .map(|_| available_assets[idx][rng.gen_range(0..available_assets[idx].len())])
                    .collect();

                users.push(chain_users);
                assets.push(chain_assets);
            }

            let start_time = Instant::now();
            // Create target_chain_ids with same length as users/assets
            let target_chain_ids = users
                .iter()
                .map(|_| vec![OPTIMISM_CHAIN_ID])
                .collect::<Vec<_>>();

            let session_info = get_proof_data_exec(
                users.clone(),
                assets.clone(),
                target_chain_ids,
                chain_ids.clone(),
                false,
            )
            .await
            .unwrap();
            let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
            let duration = start_time.elapsed();

            // Create log entry
            let mut log_entry = String::new();

            // Add metrics for each chain
            for (idx, &chain_id) in chain_ids.iter().enumerate() {
                let chain_name = match chain_id {
                    LINEA_CHAIN_ID => "linea",
                    OPTIMISM_CHAIN_ID => "optimism",
                    BASE_CHAIN_ID => "base",
                    ETHEREUM_CHAIN_ID => "ethereum",
                    _ => "unknown",
                };

                let num_users = users[idx].len();
                let num_unique_assets = assets[idx]
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len();

                log_entry.push_str(&format!(
                    "users_{} {} assets_{} {} ",
                    chain_name, num_users, chain_name, num_unique_assets
                ));
            }

            // Add total metrics
            let total_users: usize = users.iter().map(|u| u.len()).sum();
            let total_assets: usize = assets
                .iter()
                .flat_map(|a| a.iter())
                .collect::<std::collections::HashSet<_>>()
                .len();

            log_entry.push_str(&format!(
                "total_users {} total_assets {} mcycles {} duration_s {:.2}\n",
                total_users,
                total_assets,
                cycles / 1_000_000,
                duration.as_secs_f64()
            ));

            // Append to file (using append instead of write)
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("batch_logs.txt")
                .unwrap()
                .write_all(log_entry.as_bytes())
                .unwrap();
        }
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_base_sepolia() {
        let user_base = address!("6446021F4E396dA3df4235C62537431372195D38");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = BASE_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_base]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_ethereum_via_op() {
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = ETHEREUM_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_ethereum]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_ethereum_sepolia_via_op() {
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = ETHEREUM_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_ethereum]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_ethereum_sepolia_via_op_new_get_proof_data_exec() {
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let asset = GETPROOFDATA_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_exec(
            vec![vec![user_ethereum]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_ethereum_sepolia_via_op_new_get_proof_data_prove() {
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let user_ethereum2 = address!("A04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let asset = GETPROOFDATA_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let session_info = get_proof_data_prove(
            vec![vec![user_ethereum, user_ethereum2]],
            vec![vec![asset, asset]],
            vec![vec![LINEA_CHAIN_ID, OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

        let journal = hex::encode(&session_info.receipt.journal.bytes);
        println!("Journal: 0x{}", journal);
    }

    #[tokio::test]
    async fn prove_get_proof_data_on_ethereum_via_light_client() {
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = ETHEREUM_CHAIN_ID;

        // update this to recent available checkpoint
        let trusted_hash_bytes: [u8; 32] = [
            0xec, 0x00, 0x6a, 0x34, 0x19, 0x2a, 0x3f, 0x07, 0x2e, 0x7a, 0x50, 0x23, 0xa7, 0x5d,
            0xb3, 0xc6, 0x36, 0xf1, 0x8c, 0x48, 0xc4, 0x33, 0x51, 0xa3, 0x31, 0x10, 0xff, 0xad,
            0x85, 0xa2, 0xd4, 0x83,
        ];
        let trusted_hash = B256::from(trusted_hash_bytes);

        let session_info =
            get_proof_data_exec_ethereum_light_client(user_ethereum, asset, chain_id, trusted_hash)
                .await
                .unwrap();

        let cycles = session_info
            .segments
            .iter()
            .map(|s| s.cycles as u64)
            .sum::<u64>();
        println!("Cycles: {}", cycles);
    }

    #[tokio::test]
    async fn benchmark_prove_get_proof_data_all_chains() {
        let user_linea = address!("Ad7f33984bed10518012013D4aB0458D37FEE6F3");
        let user_optimism = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let user_base = address!("6446021F4E396dA3df4235C62537431372195D38");
        let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");

        println!("Benchmarking with new k256 accelerator");
        println!("-------------------------------------");
        println!("Benchmarking Linea...");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_CHAIN_ID;

        let start_time = std::time::Instant::now();
        let prove_info = get_proof_data_prove(
            vec![vec![user_linea]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        println!("MCycles: {}", prove_info.stats.total_cycles / 1000000);
        println!("e2e time: {:?}", duration);

        println!("Benchmarking Optimism...");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = OPTIMISM_CHAIN_ID;
        let start_time = std::time::Instant::now();
        let prove_info = get_proof_data_prove(
            vec![vec![user_optimism]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        println!("MCycles: {}", prove_info.stats.total_cycles / 1000000);
        println!("e2e time: {:?}", duration);

        println!("Benchmarking Base...");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = BASE_CHAIN_ID;
        let start_time = std::time::Instant::now();
        let prove_info = get_proof_data_prove(
            vec![vec![user_base]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        println!("MCycles: {}", prove_info.stats.total_cycles / 1000000);
        println!("e2e time: {:?}", duration);

        println!("Benchmarking Ethereum via Optimism...");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = ETHEREUM_CHAIN_ID;
        let start_time = std::time::Instant::now();
        let prove_info = get_proof_data_prove(
            vec![vec![user_ethereum]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        println!("MCycles: {}", prove_info.stats.total_cycles / 1000000);
        println!("e2e time: {:?}", duration);
    }

    #[tokio::test]
    async fn benchmark_block_delay_opstack_sequencer_commitment() {
        let http_url: Url = rpc_url_optimism().parse().unwrap();
        let provider = ProviderBuilder::new().on_http(http_url);
        let block_from_provider = provider
            .get_block_by_number(BlockNumberOrTag::Latest, false.into())
            .await
            .unwrap()
            .unwrap()
            .header
            .number;

        let (_, block_from_commitment) = get_current_sequencer_commitment(OPTIMISM_CHAIN_ID).await;

        println!("OPTIMISM BLOCKCHAIN:");
        println!("Block from provider: {}", block_from_provider);
        println!("Block from commitment: {}", block_from_commitment);
        println!(
            "Sequencer lag: {}",
            block_from_provider - block_from_commitment
        );

        let http_url: Url = rpc_url_base().parse().unwrap();
        let provider = ProviderBuilder::new().on_http(http_url);
        let block_from_provider = provider
            .get_block_by_number(BlockNumberOrTag::Latest, false.into())
            .await
            .unwrap()
            .unwrap()
            .header
            .number;

        let (_, block_from_commitment) = get_current_sequencer_commitment(BASE_CHAIN_ID).await;

        println!("BASE BLOCKCHAIN:");
        println!("Block from provider: {}", block_from_provider);
        println!("Block from commitment: {}", block_from_commitment);
        println!(
            "Sequencer lag: {}",
            block_from_provider - block_from_commitment
        );
    }

    #[tokio::test]
    async fn prove_empty_proof() {
        let start_time = std::time::Instant::now();
        let session_info = get_proof_data_prove_sdk(
            vec![], // empty users vectors
            vec![], // empty markets vectors
            vec![], // empty chain_ids vectors
            vec![], // empty target chain ids
            false,
        )
        .await
        .unwrap();

        let duration = start_time.elapsed();
        let cycles = session_info.stats.total_cycles / 1000;
        println!("KCycles: {}", cycles);
        println!("Duration: {:?}", duration);
        panic!("test");
    }
}
