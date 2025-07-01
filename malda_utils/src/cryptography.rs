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

use crate::constants::SECP256K1N_HALF;
use alloy_primitives::{keccak256, Address, Bytes, Signature, B256, U256};
use k256::ecdsa::{Error, RecoveryId, VerifyingKey};

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

/// Recovers the signer's address from a signature and message hash.
///
/// # Arguments
///
/// * `signature` - The signature to recover from
/// * `sighash` - The hash of the signed message
///
/// # Returns
///
/// Returns `Some(Address)` if recovery is successful, `None` otherwise.
///
/// # Notes
///
/// This function performs signature normalization and validates that the S value
/// is in the lower half of the curve order to prevent signature malleability.
pub fn recover_signer(signature: Signature, sighash: B256) -> Option<Address> {
    if signature.s() > SECP256K1N_HALF {
        return None;
    }

    let mut sig: [u8; 65] = [0; 65];

    sig[0..32].copy_from_slice(&signature.r().to_be_bytes::<32>());
    sig[32..64].copy_from_slice(&signature.s().to_be_bytes::<32>());
    sig[64] = signature.v() as u8;

    // NOTE: we are removing error from underlying crypto library as it will restrain primitive
    // errors and we care only if recovery is passing or not.
    recover_signer_unchecked(&sig, &sighash.0).ok()
}

/// Internal function to perform the actual signature recovery operation.
///
/// # Arguments
///
/// * `sig` - Raw signature bytes (65 bytes: r[32] || s[32] || v[1])
/// * `msg` - 32-byte message hash
///
/// # Returns
///
/// Returns `Result<Address, Error>` with the recovered signer's address or an error.
///
/// # Notes
///
/// This function handles signature normalization and recovery ID adjustment
/// as needed for proper key recovery.
fn recover_signer_unchecked(sig: &[u8; 65], msg: &[u8; 32]) -> Result<Address, Error> {
    let mut signature = k256::ecdsa::Signature::from_slice(&sig[0..64])?;
    let mut recid = sig[64];

    // normalize signature and flip recovery id if needed.
    if let Some(sig_normalized) = signature.normalize_s() {
        signature = sig_normalized;
        recid ^= 1;
    }
    let recid = RecoveryId::from_byte(recid)
        .expect("recovery ID should be valid as it's derived from the last byte of signature");

    // recover key
    let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &signature, recid)?;
    Ok(Address::from_public_key(&recovered_key))
}

///
/// Converts a byte slice representing an Ethereum signature into a `Signature` object.
///
/// # Arguments
///
/// * `signature` - The byte slice representing the Ethereum signature.
///
/// # Returns
///
/// Returns a `Signature` object parsed from the input byte slice.
///
/// # Notes
///
/// This function assumes the input byte slice is a valid Ethereum signature, which is 65 bytes long.
/// It extracts the `r`, `s`, and `v` components from the signature and constructs a `Signature` object.
/// The `v` component is interpreted as a boolean value, where `1` represents the parity bit.
///
/// # Errors
///
/// This function will panic if the input byte slice is not exactly 65 bytes long or if the `r` or `s` components
/// cannot be parsed into `U256` values.
pub fn signature_from_bytes(signature: &Bytes) -> Signature {
    if signature.len() != 65 {
        panic!("Invalid signature length");
    }

    let r = U256::from_be_bytes::<32>(signature[0..32].try_into().unwrap());
    let s = U256::from_be_bytes::<32>(signature[32..64].try_into().unwrap());
    let v = signature[64];

    Signature::new(r, s, v == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;
    use k256::ecdsa::SigningKey;

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

    #[test]
    fn test_recover_signer() {
        // Test with a known public key and its corresponding address
        let signing_key = SigningKey::from_slice(
            &hex::decode("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .expect("Failed to decode test private key hex string"),
        )
        .expect("Failed to create signing key from bytes");

        let verifying_key = signing_key.verifying_key();
        let expected_address = Address::from_public_key(verifying_key);

        let message = b"Test message";
        let msg_hash: [u8; 32] = keccak256(message).into();

        // Sign the message
        let (sig, recid) = signing_key
            .sign_prehash_recoverable(&msg_hash)
            .expect("Failed to sign test message");

        let mut sig_bytes = [0u8; 65];
        sig_bytes[..64].copy_from_slice(&sig.to_bytes());
        sig_bytes[64] = recid.to_byte();

        // Convert to Signature type
        let signature = signature_from_bytes(&sig_bytes.into());

        // Test recovery
        let recovered_address = recover_signer(signature, msg_hash.into());
        assert_eq!(Some(expected_address), recovered_address);

        // Test with invalid signature (modified S value)
        let mut invalid_sig = signature;
        let invalid_s = SECP256K1N_HALF + U256::from(1);

        invalid_sig = Signature::new(invalid_sig.r(), invalid_s, invalid_sig.v());
        let recovered_invalid = recover_signer(invalid_sig, msg_hash.into());
        assert_eq!(None, recovered_invalid);
    }
}
