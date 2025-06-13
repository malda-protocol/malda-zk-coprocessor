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
// This file contains code derived from or inspired by Risc0,
// originally licensed under the Apache License 2.0. See LICENSE-RISC0
// and the NOTICE file for original license terms and attributions.

//! Guest utilities for Risc Zero zkVM methods
//!
//! This crate provides common utilities, types, and functions used by guest methods
//! running inside the Risc Zero zkVM. It includes modules for constants, custom types,
//! cryptographic operations, and validation functions.

/// Commonly used constants
pub mod constants;

/// Custom types and data structures
pub mod types;

/// Validation and verification utilities
pub mod validators;

/// Cryptographic operations and primitives
pub mod cryptography;
