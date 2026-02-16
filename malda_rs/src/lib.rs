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
//!
//! Code for host/client and zkVM guest program including constants,
//! view calls, cryptographic operations, type definitions, and validation logic.

pub mod constants;

pub mod viewcalls;

pub use malda_utils::cryptography;
pub use malda_utils::types;
pub use malda_utils::validators;

#[cfg(feature = "guest")]
pub use methods;

mod boundless;
