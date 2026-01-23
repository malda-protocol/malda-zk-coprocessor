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

//! Bonsai SDK proving backend.
//!
//! This module provides functions for generating ZK proofs using the Bonsai remote proving service.
//! Bonsai handles the computationally intensive proof generation on remote infrastructure.

use std::time::Duration;

use anyhow::Result;
use bonsai_sdk::blocking::Client;
use risc0_zkvm::Receipt;

use crate::prover::types::{BonsaiProveInfo, SessionStats};

/// Generates a ZK proof using the Bonsai SDK with an explicit image ID.
///
/// This function uploads input data to Bonsai, creates a proof session,
/// waits for STARK proof generation, then creates and waits for SNARK
/// proof generation, and finally returns the complete proof receipt.
///
/// # Arguments
/// * `image_id` - The hex-encoded Risc0 image ID for the guest program.
/// * `input` - The serialized input data for the guest program.
///
/// # Returns
/// * `Result<BonsaiProveInfo>` - The proof receipt and session statistics.
///
/// # Environment Variables
/// Requires Bonsai SDK environment variables to be set:
/// - `BONSAI_API_KEY` - API key for Bonsai service
/// - `BONSAI_API_URL` - URL for the Bonsai service
pub fn prove_bonsai(image_id: &str, input: Vec<u8>) -> Result<BonsaiProveInfo> {
    // Initialize the Bonsai client from environment variables (uses RISC Zero version for compatibility)
    let client = Client::from_env(risc0_zkvm::VERSION)?;

    // Upload the input data to Bonsai and get an input ID
    let input_id = client.upload_input(input)?;

    let assumptions: Vec<String> = vec![];
    let execute_only = false;

    // Create a new proof session on Bonsai
    let session = client.create_session(image_id.to_string(), input_id, assumptions, execute_only)?;

    let polling_interval = Duration::from_millis(500);

    // --- STARK phase: Wait for the session to complete and collect stats ---
    let stark_time = std::time::Instant::now();
    let stats = loop {
        let res = session.status(&client)?;
        if res.status == "RUNNING" {
            // Session is still running, wait and poll again
            std::thread::sleep(polling_interval);
            continue;
        }
        if res.status == "SUCCEEDED" {
            // Session succeeded, extract stats
            let bonsai_stats = res
                .stats
                .expect("Missing stats object on Bonsai status res");
            tracing::debug!(
                "Bonsai usage: cycles: {} total_cycles: {}",
                bonsai_stats.cycles,
                bonsai_stats.total_cycles
            );

            break SessionStats {
                segments: bonsai_stats.segments,
                total_cycles: bonsai_stats.total_cycles,
                user_cycles: bonsai_stats.cycles,
                paging_cycles: 0,   // Paging cycles not tracked in this context
                reserved_cycles: 0, // Reserved cycles not tracked in this context
            };
        } else {
            // Session failed or exited unexpectedly
            return Err(anyhow::Error::msg(format!(
                "Bonsai prover workflow [{}] exited: {} err: {}",
                session.uuid,
                res.status,
                res.error_msg
                    .unwrap_or("Bonsai workflow missing error_msg".into())
            )));
        }
    };
    let stark_time = stark_time.elapsed();

    // --- SNARK phase: Create a SNARK session and wait for completion ---
    let snark_session = client.create_snark(session.uuid.clone())?;

    let start = std::time::Instant::now();
    let snark_receipt_url = loop {
        let res = snark_session.status(&client)?;
        match res.status.as_str() {
            "RUNNING" => {
                // SNARK session is still running, wait and poll again
                std::thread::sleep(polling_interval);
                continue;
            }
            "SUCCEEDED" => {
                // SNARK session succeeded, get the output URL
                break res.output.ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "Bonsai prover workflow [{}] reported success, but provided no receipt",
                        snark_session.uuid
                    ))
                })?;
            }
            _ => {
                // SNARK session failed or exited unexpectedly
                return Err(anyhow::Error::msg(format!(
                    "Bonsai prover workflow [{}] exited: {} err: {}",
                    snark_session.uuid,
                    res.status,
                    res.error_msg
                        .unwrap_or("Bonsai workflow missing error_msg".into())
                )));
            }
        }
    };

    let snark_time = start.elapsed();

    // Download the Groth16 receipt (proof) from Bonsai and deserialize it
    let receipt_buf = client.download(&snark_receipt_url)?;
    let groth16_receipt: Receipt = bincode::deserialize(&receipt_buf)?;

    Ok(BonsaiProveInfo {
        receipt: groth16_receipt,
        stats,
        uuid: session.uuid,
        stark_time: stark_time.as_secs(),
        snark_time: snark_time.as_secs(),
    })
}

/// Generates a ZK proof using the Bonsai SDK with image ID from environment.
///
/// This is a convenience function that reads the image ID from the
/// `IMAGE_ID_BONSAI` environment variable.
///
/// # Arguments
/// * `input` - The serialized input data for the guest program.
///
/// # Returns
/// * `Result<BonsaiProveInfo>` - The proof receipt and session statistics.
///
/// # Environment Variables
/// - `IMAGE_ID_BONSAI` - Hex-encoded Risc0 image ID
/// - `BONSAI_API_KEY` - API key for Bonsai service
/// - `BONSAI_API_URL` - URL for the Bonsai service
pub fn prove_bonsai_from_env(input: Vec<u8>) -> Result<BonsaiProveInfo> {
    let image_id =
        dotenvy::var("IMAGE_ID_BONSAI").expect("IMAGE_ID_BONSAI must be set in environment");
    prove_bonsai(&image_id, input)
}
