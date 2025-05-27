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
//! Generated crate containing the image ID and ELF binary of the build guest.
include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {

    use alloy_primitives::{address, Address, B256};
    use hex;
    use malda_rs::{
        constants::*,
        viewcalls::{
            get_proof_data_exec, get_proof_data_prove,
            get_proof_data_prove_sdk,
        },
        viewcalls_ethereum_light_client::get_proof_data_exec as get_proof_data_exec_ethereum_light_client,
    };

    pub const WETH_MARKET_SEPOLIA: Address = address!("B84644c24B4D0823A0770ED698f7C20B88Bcf824");
    pub const WETH_MARKET: Address = address!("C7Bc6bD45Eb84D594f51cED3c5497E6812C7732f");

    #[tokio::test]
    async fn prove_get_proof_data_on_linea() {
        let user_linea = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let asset = WETH_MARKET;
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

    }

    #[tokio::test]
    async fn should_pass_prove_sepolia_get_proof_data_on_linea() {
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
    }

    #[tokio::test]
    async fn should_pass_prove_sepolia_get_proof_data_on_linea_slow_lane() {
        let user_linea = address!("2693946791da99dA78Ac441abA6D5Ce2Bccd96D3");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = LINEA_SEPOLIA_CHAIN_ID;

        let start_time = std::time::Instant::now();
        let session_info = get_proof_data_exec(
            vec![vec![user_linea]],
            vec![vec![asset]],
            vec![vec![OPTIMISM_CHAIN_ID]],
            vec![chain_id],
            true,
        )
        .await
        .unwrap();
        let duration = start_time.elapsed();

        let cycles = session_info.segments.iter().map(|s| s.cycles).sum::<u32>();
        println!("journal: 0x{}", hex::encode(&session_info.journal));
        println!("Cycles: {}", cycles);
        println!("Duration: {:?}", duration);
    }


    #[tokio::test]
    async fn should_pass_prove_get_proof_data_on_optimism_sepolia_sdk() {
        let user_optimism = address!("e50fA9b3c56FfB159cB0FCA61F5c9D750e8128c8");
        let asset = WETH_MARKET_SEPOLIA;
        let chain_id = OPTIMISM_SEPOLIA_CHAIN_ID;

        let _session_info = get_proof_data_prove_sdk(
            vec![vec![user_optimism]],
            vec![vec![asset]],
            vec![vec![LINEA_CHAIN_ID]],
            vec![chain_id],
            false,
        )
        .await
        .unwrap();

    }

    #[tokio::test]
    async fn should_pass_prove_get_proof_data_on_optimism_sepolia_slow_lane() {
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
    }


    #[tokio::test]
    async fn prove_get_proof_data_on_base() {
        let user_base = address!("6446021F4E396dA3df4235C62537431372195D38");
        let asset = WETH_MARKET;
        let chain_id = BASE_CHAIN_ID;

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
        let asset = WETH_MARKET;
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
    async fn should_pass_prove_get_proof_data_on_ethereum_sepolia_via_op() {
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

    // #[tokio::test]
    // async fn prove_get_proof_data_on_ethereum_via_light_client() {
    //     let user_ethereum = address!("F04a5cC80B1E94C69B48f5ee68a08CD2F09A7c3E");
    //     let asset = WETH_MARKET_SEPOLIA;
    //     let chain_id = ETHEREUM_CHAIN_ID;

    //     // update this to recent available checkpoint
    //     let trusted_hash_bytes: [u8; 32] = [
    //         0xec, 0x00, 0x6a, 0x34, 0x19, 0x2a, 0x3f, 0x07, 0x2e, 0x7a, 0x50, 0x23, 0xa7, 0x5d,
    //         0xb3, 0xc6, 0x36, 0xf1, 0x8c, 0x48, 0xc4, 0x33, 0x51, 0xa3, 0x31, 0x10, 0xff, 0xad,
    //         0x85, 0xa2, 0xd4, 0x83,
    //     ];
    //     let trusted_hash = B256::from(trusted_hash_bytes);

    //     let session_info =
    //         get_proof_data_exec_ethereum_light_client(user_ethereum, asset, chain_id, trusted_hash)
    //             .await
    //             .unwrap();

    //     let cycles = session_info
    //         .segments
    //         .iter()
    //         .map(|s| s.cycles as u64)
    //         .sum::<u64>();
    //     println!("Cycles: {}", cycles);
    // }

}
