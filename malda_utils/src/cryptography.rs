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
//! Cryptographic utilities for Ethereum-style signature operations.
//!
//! This module provides functionality for signature message creation,
//! signer recovery, and address derivation from public keys using
//! the secp256k1 elliptic curve.

use alloy_primitives::{keccak256, B256};

/// Creates a signature message hash following Ethereum's signing scheme.
///
/// # Arguments
///
/// * `data` - The raw data to be signed
/// * `chain_id` - The blockchain network identifier
///
/// # Returns
///
/// Returns a `B256` containing the final message hash to be signed.
///
/// # Details
///
/// The function concatenates three components:
/// - A domain separator (currently zero)
/// - The chain ID in padded format
/// - The keccak256 hash of the input data
pub fn signature_msg(data: &[u8], chain_id: u64) -> B256 {
    let domain = B256::ZERO;
    let chain_id = B256::left_padding_from(&chain_id.to_be_bytes());
    let payload_hash = keccak256(data);

    let signing_data = [
        domain.as_slice(),
        chain_id.as_slice(),
        payload_hash.as_slice(),
    ];

    keccak256(signing_data.concat()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_msg() {
        let data = b"Hello, World!";
        let chain_id = 1;
        let msg = signature_msg(data, chain_id);

        // Verify the result is deterministic and non-zero
        assert_ne!(msg, B256::ZERO);

        // Test with empty data
        let empty_msg = signature_msg(&[], 1);
        assert_ne!(empty_msg, B256::ZERO);

        // Test with different chain IDs
        let msg1 = signature_msg(data, 1);
        let msg2 = signature_msg(data, 2);
        assert_ne!(msg1, msg2);
    }
}
