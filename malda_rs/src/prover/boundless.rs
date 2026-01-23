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

//! Boundless market proving backend.
//!
//! This module provides functions for generating ZK proofs using the Boundless
//! decentralized proving market. The market uses a reverse Dutch auction mechanism
//! to match proof requests with provers.

use std::str::FromStr;
use std::time::Duration;

use alloy::primitives::{Bytes, U256};
use anyhow::{bail, Context, Result};
use boundless_market::request_builder::OfferParams;
use boundless_market::Client as BoundlessClient;
use boundless_market::storage::storage_provider_from_env;
use url::Url;

use alloy::signers::local::PrivateKeySigner;

use crate::prover::types::BoundlessParams;

/// Generates a ZK proof using the Boundless decentralized proving market.
///
/// This function creates a proof request and submits it to the Boundless market,
/// which handles the ZK proof generation in a decentralized manner. The function
/// waits for the request to be fulfilled and returns the proof journal and seal.
///
/// # Arguments
/// * `elf` - The ELF binary for the guest program.
/// * `input` - The serialized input data for the guest program.
/// * `params` - Configuration parameters for the Boundless market client.
/// * `onchain` - Whether to submit onchain (true) or offchain (false).
///
/// # Returns
/// * `Result<(Bytes, Bytes)>` - Tuple of (journal, seal) if successful.
///
/// # Environment Variables
/// Required:
/// - `RPC_URL`: Ethereum RPC endpoint for transactions
/// - `PRIVATE_KEY`: Private key for signing transactions
///
/// Optional:
/// - `PROGRAM_URL`: URL of a pre-uploaded program to avoid re-upload latency
/// - `PINATA_JWT`: JWT for Pinata storage provider (falls back to boundless storage)
pub async fn prove_boundless(
    elf: &[u8],
    input: Vec<u8>,
    params: BoundlessParams,
    onchain: bool,
) -> Result<(Bytes, Bytes)> {
    // Only initialize tracing if it hasn't been set up already
    if tracing_subscriber::util::SubscriberInitExt::try_init(
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()),
    )
    .is_err()
    {
        // Tracing is already initialized, which is fine
        tracing::debug!("Tracing subscriber already initialized");
    }

    // Load environment variables from .env if present
    match dotenvy::dotenv() {
        Ok(path) => tracing::debug!("Loaded environment variables from {:?}", path),
        Err(e) if e.not_found() => tracing::debug!("No .env file found"),
        Err(e) => bail!("failed to load .env file: {}", e),
    }

    // Get required environment variables for RPC and signing
    let rpc_url = dotenvy::var("RPC_URL").context("RPC_URL environment variable not set")?;
    let private_key =
        dotenvy::var("PRIVATE_KEY").context("PRIVATE_KEY environment variable not set")?;

    let rpc_url = Url::parse(&rpc_url)?;
    let private_key = PrivateKeySigner::from_str(&private_key)?;

    // Create a Boundless client from the provided parameters
    let client = BoundlessClient::builder()
        .with_storage_provider(Some(storage_provider_from_env()?))
        .with_rpc_url(rpc_url)
        .with_private_key(private_key)
        .config_offer_layer(|config| {
            config
                .max_price_per_cycle(U256::from(params.max_price_per_cycle))
                .min_price_per_cycle(U256::from(params.min_price_per_cycle))
                .ramp_up_period(params.ramp_up_period.try_into().unwrap())
                .lock_timeout(params.lock_timeout.try_into().unwrap())
                .timeout(params.timeout.try_into().unwrap())
        })
        .build()
        .await
        .context("failed to build boundless client")?;

    // Get program URL - upload if not available in environment
    let program_url = if let Ok(program_url) = dotenvy::var("PROGRAM_URL") {
        tracing::info!("Using pre-uploaded program from URL: {}", program_url);
        Url::parse(&program_url).context("Failed to parse PROGRAM_URL")?
    } else {
        tracing::info!("No PROGRAM_URL found, uploading program directly");
        let program_url = client.upload_program(elf).await?;
        tracing::info!("program uploaded to {}", program_url);
        program_url
    };

    use std::time::{SystemTime, UNIX_EPOCH};

    let current_unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Build the request
    let request = client
        .new_request()
        .with_program_url(program_url)?
        .with_stdin(input)
        .with_offer(
            OfferParams::builder()
                .bidding_start(current_unix_time + params.bidding_start_delay),
        )
        .with_groth16_proof();
    tracing::info!("request built");

    // Submit the request to the Boundless market (onchain or offchain)
    tracing::info!("submitting request");
    let (request_id, expires_at) = if onchain {
        let cl = client.submit_onchain(request).await?;
        tracing::info!("request submitted onchain");
        cl
    } else {
        client.submit_offchain(request).await?
    };

    // Wait for the request to be fulfilled. The market will return the journal and seal.
    tracing::info!("Waiting for request {:x} to be fulfilled", request_id);
    let fulfillment = client
        .wait_for_request_fulfillment(
            request_id,
            Duration::from_secs(5), // check every 5 seconds
            expires_at,
        )
        .await?;

    let journal = fulfillment
        .data()?
        .journal()
        .ok_or_else(|| anyhow::anyhow!("Journal not found in fulfillment data"))?
        .clone();
    let seal = fulfillment.seal;
    tracing::info!("Request {:x} fulfilled", request_id);

    Ok((journal, seal))
}
