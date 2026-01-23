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

//! Local proving backend.
//!
//! This module provides functions for generating ZK proofs locally using
//! the Risc0 ZKVM. Local proving is useful for development and testing.

use anyhow::Result;
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv, ProveInfo, ProverOpts, SessionInfo};

/// Executes a guest program without generating a proof.
///
/// This function runs the guest program in the ZKVM executor and returns
/// session information including the journal output. Useful for testing
/// and debugging guest programs before generating proofs.
///
/// # Arguments
/// * `env` - The executor environment with inputs.
/// * `elf` - The ELF binary for the guest program.
///
/// # Returns
/// * `Result<SessionInfo>` - Session information including journal output.
pub fn execute(env: ExecutorEnv<'_>, elf: &[u8]) -> Result<SessionInfo> {
    Ok(default_executor().execute(env, elf)?)
}

/// Generates a ZK proof locally using the default prover.
///
/// This function runs the guest program and generates a Groth16 proof
/// suitable for on-chain verification.
///
/// # Arguments
/// * `env` - The executor environment with inputs.
/// * `elf` - The ELF binary for the guest program.
///
/// # Returns
/// * `Result<ProveInfo>` - The proof receipt and session information.
///
/// # Note
/// Local proving can be computationally intensive and slow compared to
/// remote proving services like Bonsai or Boundless.
pub fn prove_local(env: ExecutorEnv<'_>, elf: &[u8]) -> Result<ProveInfo> {
    Ok(default_prover().prove_with_opts(env, elf, &ProverOpts::groth16())?)
}

/// Generates a ZK proof locally using the default prover with succinct (STARK) proof.
///
/// This function runs the guest program and generates a STARK proof.
/// STARK proofs are faster to generate but larger than Groth16 proofs.
///
/// # Arguments
/// * `env` - The executor environment with inputs.
/// * `elf` - The ELF binary for the guest program.
///
/// # Returns
/// * `Result<ProveInfo>` - The proof receipt and session information.
pub fn prove_local_succinct(env: ExecutorEnv<'_>, elf: &[u8]) -> Result<ProveInfo> {
    Ok(default_prover().prove_with_opts(env, elf, &ProverOpts::succinct())?)
}
