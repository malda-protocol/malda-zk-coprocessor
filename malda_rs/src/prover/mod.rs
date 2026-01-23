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

//! Generic Risc0 proving infrastructure.
//!
//! This module provides backend-agnostic functionality for ZK proof generation
//! using the Risc0 ZKVM. It supports multiple proving backends:
//!
//! - **Local**: Direct proving on the local machine
//! - **Bonsai**: Remote proving via the Bonsai SDK
//! - **Boundless**: Decentralized proving market
//!
//! # Example
//!
//! ```ignore
//! use malda_rs::prover::{prove_bonsai_from_env, BonsaiProveInfo};
//!
//! let input = vec![1, 2, 3, 4];
//! let prove_info = prove_bonsai_from_env(input)?;
//! println!("Proof generated with {} cycles", prove_info.stats.total_cycles);
//! ```

mod bonsai;
mod boundless;
mod local;
mod types;

pub use bonsai::{prove_bonsai, prove_bonsai_from_env};
pub use boundless::prove_boundless;
pub use local::{execute, prove_local, prove_local_succinct};
pub use types::{BonsaiProveInfo, BoundlessParams, SessionStats, TX_TIMEOUT};
