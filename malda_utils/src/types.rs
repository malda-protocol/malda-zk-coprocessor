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
//! Types module containing core data structures and implementations for blockchain payload processing.
//!
//! This module provides essential types and structures for handling blockchain execution payloads,
//! sequencer commitments, and related blockchain data structures.

use alloy_sol_types::sol;

use eyre::Result;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssz")]
use alloy_rlp::RlpEncodable;
#[cfg(feature = "ssz")]
use ssz_derive::{Decode, Encode};

use crate::cryptography::signature_msg;
use alloy_primitives::{Address, Bytes, Signature, B256, U256};

use risc0_steel::config::{ChainSpec, ForkCondition};

use revm::primitives::hardfork::SpecId;
use std::{collections::BTreeMap, sync::LazyLock};

pub type EthChainSpec = ChainSpec<SpecId>;

// NOTE: Forks are configured according to the official Linea protocol release:
// https://github.com/Consensys/protocols-release-sandbox/blob/b2562432e9ff7b2a1bcd29f48d22d07c1da62630/config/src/main/resources/linea-mainnet.json#L15-L18
pub static LINEA_MAINNET_CHAIN_SPEC: LazyLock<EthChainSpec> = LazyLock::new(|| ChainSpec {
    chain_id: 59144,
    forks: BTreeMap::from([
        (SpecId::LONDON, ForkCondition::Block(0)),
        (SpecId::SHANGHAI, ForkCondition::Timestamp(1761213600)),
        (SpecId::CANCUN, ForkCondition::Timestamp(1761645600)),
        (SpecId::PRAGUE, ForkCondition::Timestamp(1761646200)),
        (SpecId::OSAKA, ForkCondition::Timestamp(1764798551)),
    ]),
});

// NOTE: Forks are configured according to the official Linea protocol release:
// https://github.com/Consensys/protocols-release-sandbox/blob/b2562432e9ff7b2a1bcd29f48d22d07c1da62630/config/src/main/resources/sepolia.json#L16-L19
pub static LINEA_SEPOLIA_CHAIN_SPEC: LazyLock<EthChainSpec> = LazyLock::new(|| ChainSpec {
    chain_id: 59141,
    forks: BTreeMap::from([
        (SpecId::LONDON, ForkCondition::Block(0)),
        (SpecId::SHANGHAI, ForkCondition::Timestamp(1677557088)),
        (SpecId::CANCUN, ForkCondition::Timestamp(1706655072)),
        (SpecId::PRAGUE, ForkCondition::Timestamp(1741159776)),
        (SpecId::OSAKA, ForkCondition::Timestamp(1760427360)),
    ]),
});

pub struct TakeLastXBytes(pub usize);

pub enum SolidityDataType<'a> {
    String(&'a str),
    Address(Address),
    Bytes(&'a [u8]),
    Bool(bool),
    Number(U256),
    NumberWithShift(U256, TakeLastXBytes),
}

pub mod abi {
    use super::SolidityDataType;

    /// Pack a single `SolidityDataType` into bytes
    fn pack<'a>(data_type: &'a SolidityDataType) -> Vec<u8> {
        let mut res = Vec::new();
        match data_type {
            SolidityDataType::String(s) => {
                res.extend(s.as_bytes());
            }
            SolidityDataType::Address(a) => {
                res.extend(a.0);
            }
            SolidityDataType::Number(n) => {
                res.extend(n.to_be_bytes::<32>());
            }
            SolidityDataType::Bytes(b) => {
                res.extend(*b);
            }
            SolidityDataType::Bool(b) => {
                if *b {
                    res.push(1);
                } else {
                    res.push(0);
                }
            }
            SolidityDataType::NumberWithShift(n, to_take) => {
                let local_res = n.to_be_bytes::<32>().to_vec();

                let to_skip = local_res.len() - (to_take.0 / 8);

                let local_res = local_res.into_iter().skip(to_skip).collect::<Vec<u8>>();
                res.extend(local_res);
            }
        };
        return res;
    }

    pub fn encode_packed(items: &[SolidityDataType]) -> (Vec<u8>, String) {
        let res = items.iter().fold(Vec::new(), |mut acc, i| {
            let pack = pack(i);
            acc.push(pack);
            acc
        });
        let res = res.join(&[][..]);
        let hexed = hex::encode(&res);
        (res, hexed)
    }
}

sol! {
    /// Interface for querying proof data from the Malda Market.
    interface IMaldaMarket {
        /// Returns the proof data for a given account.
        ///
        /// # Arguments
        /// * `account` - The address to query the proof data for
        /// * `dstChainId` - The chainId to query the proof data for
        function getProofData(address account, uint32 dstChainId) external view returns (bytes memory);
    }

    interface IL1MessageService {
        /// Returns the latest L2 block number known to L1.
        ///
        /// This function is used to query the last L2 block number that has been processed by L1.
        /// Note: This value is not updated by proof and relies on trust in the Linea team.
        function currentL2BlockNumber() external view returns (uint256);
    }

    /// Interface for accessing L1 block information.
    interface IL1Block {
        /// Returns the hash of the current L1 block.
        function hash() external view returns (bytes32);
        /// Returns the number of the current L1 block.
        function number() external view returns (uint64);
    }

    // https://github.com/ethereum-optimism/optimism/blob/v1.9.3/packages/contracts-bedrock/src/dispute/interfaces/IDisputeGameFactory.sol
    interface IDisputeGameFactory {
        function gameCount() external view returns (uint256);
        function gameAtIndex(uint256 index) external view returns (uint256, uint256, address);
    }

    // https://github.com/ethereum-optimism/optimism/blob/v1.9.3/packages/contracts-bedrock/src/dispute/interfaces/IDisputeGame.sol
    interface IDisputeGame {
        function status() external view returns (GameStatus);
        function resolvedAt() external view returns (uint64);
        function rootClaim() external pure returns (bytes32);
        function l2BlockNumberChallenged() external view returns (bool);
        function l2BlockNumber() external view returns (uint256);
        function extraData() external view returns (bytes memory);
    }

    struct OutputRootProof {
        bytes32 version;
        bytes32 stateRoot;
        bytes32 messagePasserStorageRoot;
        bytes32 latestBlockhash;
    }

    // https://github.com/ethereum-optimism/optimism/blob/v1.9.3/packages/contracts-bedrock/src/dispute/lib/Types.sol
    #[derive(Debug, PartialEq)]
    enum GameStatus {
        IN_PROGRESS,
        CHALLENGER_WINS,
        DEFENDER_WINS
    }

    /// @title Multicall3 interface for batch calling contracts
    /// @dev Allows batching multiple proof data queries in a single transaction
    struct Call3 {
        /// @dev Target contract to call
        address target;
        /// @dev If true, allows the call to fail without reverting the entire transaction
        bool allowFailure;
        /// @dev Calldata to execute on the target contract
        bytes callData;
    }

    /// @dev Result of an individual proof data query within the batch
    struct CallResult {
        /// @dev Indicates if the call was successful
        bool success;
        /// @dev Contains the return data (if successful) or revert data (if failed)
        bytes returnData;
    }

    /// @title Interface for batched contract calls
    interface IMulticall3 {
        /// @notice Executes a batch of function calls on various contracts
        /// @param calls Array of Call3 structs containing call parameters
        /// @return results Array of CallResult structs containing call results
        function aggregate3(Call3[] calldata calls) external payable returns (CallResult[] memory results);
    }

    struct Amounts {
        uint256 amountIn;
        uint256 amountOut;
    }

    /// @title Interface for the Optimism Portal
    interface IOptimismPortal {
        /// @notice Returns the address of the DisputeGameFactory
        function disputeGameFactory() external view returns (address);

        /// @notice Returns the timestamp when the respected game type was last updated
        function respectedGameTypeUpdatedAt() external view returns (uint256);

        /// @notice Checks if a dispute game is blacklisted
        /// @param game The address of the dispute game
        function disputeGameBlacklist(address game) external view returns (bool);

        /// @notice Returns the proof maturity delay in seconds
        function proofMaturityDelaySeconds() external view returns (uint256);
    }
}

/// Represents a commitment made by a sequencer, containing signed payload data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerCommitment {
    /// The compressed payload data
    pub data: Bytes,
    /// The cryptographic signature of the commitment
    pub signature: Signature,
}

/// Offset within the SSZ-encoded ExecutionPayload where block_hash field starts.
///
/// SSZ layout of ExecutionPayload fixed-size portion:
/// - parent_hash: B256 (32 bytes)
/// - fee_recipient: Address (20 bytes)
/// - state_root: B256 (32 bytes)
/// - receipts_root: B256 (32 bytes)
/// - logs_bloom: FixedVector<u8, 256> (256 bytes)
/// - prev_randao: B256 (32 bytes)
/// - block_number: u64 (8 bytes)
/// - gas_limit: u64 (8 bytes)
/// - gas_used: u64 (8 bytes)
/// - timestamp: u64 (8 bytes)
/// - extra_data offset: u32 (4 bytes) - variable-length field stores offset
/// - base_fee_per_gas: U256 (32 bytes)
/// - block_hash: B256 (32 bytes) <- starts at byte 472
const SSZ_BLOCK_HASH_OFFSET: usize = 472;

impl SequencerCommitment {
    /// Creates a new SequencerCommitment from compressed data.
    ///
    /// # Arguments
    /// * `data` - The compressed data bytes
    ///
    /// # Returns
    /// * `Result<Self>` - The created commitment or an error
    pub fn new(data: &[u8]) -> Result<Self> {
        let mut decoder = snap::raw::Decoder::new();
        let decompressed = decoder.decompress_vec(&data)?;

        let signature = Signature::try_from(&decompressed[..65])?;
        let data = Bytes::from(decompressed[65..].to_vec());

        Ok(SequencerCommitment { data, signature })
    }

    /// Verifies the commitment signature against a given signer and chain ID.
    ///
    /// # Arguments
    /// * `signer` - The expected signer's address
    /// * `chain_id` - The blockchain network ID
    ///
    /// # Returns
    /// * `Result<()>` - Ok if verification succeeds, Error otherwise
    pub fn verify(&self, signer: Address, chain_id: u64) -> Result<()> {
        let msg = signature_msg(&self.data, chain_id);
        let pk = self.signature.recover_from_prehash(&msg)?;
        let recovered_signer = Address::from_public_key(&pk);

        if signer != recovered_signer {
            eyre::bail!("invalid signer");
        }

        Ok(())
    }

    /// Extracts the block hash directly from the SSZ-encoded payload using fast access.
    ///
    /// This method reads the block_hash field directly from its known offset in the
    /// SSZ-encoded data, avoiding full SSZ decoding. This is useful for environments
    /// where the full SSZ decoding dependencies (e.g., `ethereum_hashing`) are not
    /// available, such as RISC-V zkVM targets.
    ///
    /// The data layout is: `[32 bytes prefix][SSZ-encoded ExecutionPayload]`
    /// Block hash is located at offset 472 within the SSZ-encoded portion.
    ///
    /// # Returns
    /// * `Result<B256>` - The block hash or an error if data is too short
    pub fn fast_block_hash(&self) -> Result<B256> {
        // Skip the 32-byte prefix, then access block_hash at SSZ_BLOCK_HASH_OFFSET
        const DATA_PREFIX: usize = 32;
        const BLOCK_HASH_SIZE: usize = 32;
        let start = DATA_PREFIX + SSZ_BLOCK_HASH_OFFSET;
        let end = start + BLOCK_HASH_SIZE;

        if self.data.len() < end {
            eyre::bail!(
                "data too short for fast block hash access: need {} bytes, have {}",
                end,
                self.data.len()
            );
        }

        Ok(B256::from_slice(&self.data[start..end]))
    }
}

// ============================================================================
// SSZ-dependent types and implementations (requires "ssz" feature)
// ============================================================================

#[cfg(feature = "ssz")]
mod ssz_types_impl {
    use super::*;
    use ssz_types::{typenum, FixedVector, VariableList};

    /// Conversion implementation from SequencerCommitment to ExecutionPayload.
    impl TryFrom<&SequencerCommitment> for ExecutionPayload {
        type Error = eyre::Report;

        /// Attempts to convert a SequencerCommitment into an ExecutionPayload.
        ///
        /// This performs full SSZ decoding and validates that the fast-access block hash
        /// matches the decoded block hash. This validation ensures the fast-access offset
        /// remains correct as the codebase evolves.
        ///
        /// # Arguments
        /// * `value` - The SequencerCommitment to convert
        ///
        /// # Returns
        /// * `Result<Self>` - The converted payload or an error
        fn try_from(value: &SequencerCommitment) -> Result<Self> {
            let payload_bytes = &value.data[32..];
            let payload: ExecutionPayload = ssz::Decode::from_ssz_bytes(payload_bytes)
                .map_err(|_| eyre::eyre!("decode failed"))?;

            // Validate that fast-access block hash matches decoded block hash.
            // This ensures SSZ_BLOCK_HASH_OFFSET remains correct.
            let fast_hash = value.fast_block_hash()?;
            if fast_hash != payload.block_hash {
                eyre::bail!(
                    "fast-access block hash mismatch: fast={}, decoded={}",
                    fast_hash,
                    payload.block_hash
                );
            }

            Ok(payload)
        }
    }

    /// Represents a complete blockchain execution payload.
    #[derive(Debug, Clone, Encode, Decode)]
    pub struct ExecutionPayload {
        /// Hash of the parent block
        pub parent_hash: B256,
        /// Address of the fee recipient
        pub fee_recipient: Address,
        /// Root hash of the state trie
        pub state_root: B256,
        /// Root hash of the receipt trie
        pub receipts_root: B256,
        /// Bloom filter for the logs
        pub logs_bloom: LogsBloom,
        /// Previous random value used in block production
        pub prev_randao: B256,
        /// Block number
        pub block_number: u64,
        /// Maximum gas allowed in the block
        pub gas_limit: u64,
        /// Total gas used in the block
        pub gas_used: u64,
        /// Block timestamp
        pub timestamp: u64,
        /// Additional data included in the block
        pub extra_data: ExtraData,
        /// Base fee per gas unit
        pub base_fee_per_gas: U256,
        /// Hash of the current block
        pub block_hash: B256,
        /// List of transactions included in the block
        pub transactions: VariableList<Transaction, typenum::U1048576>,
        /// List of withdrawals processed in the block
        pub withdrawals: VariableList<Withdrawal, typenum::U16>,
        /// Amount of blob gas used in the block
        pub blob_gas_used: u64,
        /// Excess blob gas in the block
        pub excess_blob_gas: u64,
        /// Root of withdrawals - optional to match Go implementation for Bedrock, Canyon, Delta, Ecotone, Fjord, Granite, Holocene
        pub withdrawals_root: B256,
    }

    /// Type alias for a transaction, represented as a variable-length byte list
    pub type Transaction = VariableList<u8, typenum::U1073741824>;
    /// Type alias for a logs bloom filter, represented as a fixed-length byte vector
    pub type LogsBloom = FixedVector<u8, typenum::U256>;
    /// Type alias for extra data, represented as a variable-length byte list
    pub type ExtraData = VariableList<u8, typenum::U32>;

    /// Represents a withdrawal operation in the blockchain.
    ///
    /// Copied from https://docs.rs/alloy/latest/alloy/eips/eip4895/struct.Withdrawal.html
    /// which doesn't work as direct input due to mismatch between crate versions between alloy and ssz
    #[derive(Clone, Debug, Encode, Decode, RlpEncodable)]
    pub struct Withdrawal {
        /// Sequential index of the withdrawal
        index: u64,
        /// Index of the validator processing the withdrawal
        validator_index: u64,
        /// Recipient address of the withdrawal
        address: Address,
        /// Amount being withdrawn
        amount: u64,
    }
}

#[cfg(feature = "ssz")]
pub use ssz_types_impl::*;

#[cfg(all(test, feature = "ssz"))]
mod tests {
    use super::*;
    use ssz::Encode;
    use ssz_types::{FixedVector, VariableList};

    #[test]
    fn test_fast_block_hash_matches_decoded() {
        // Create a minimal ExecutionPayload with a known block_hash
        let expected_block_hash = B256::repeat_byte(0xAB);

        let payload = ExecutionPayload {
            parent_hash: B256::repeat_byte(0x01),
            fee_recipient: Address::repeat_byte(0x02),
            state_root: B256::repeat_byte(0x03),
            receipts_root: B256::repeat_byte(0x04),
            logs_bloom: FixedVector::from(vec![0u8; 256]),
            prev_randao: B256::repeat_byte(0x05),
            block_number: 12345,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            timestamp: 1700000000,
            extra_data: VariableList::from(vec![]),
            base_fee_per_gas: U256::from(1_000_000_000u64),
            block_hash: expected_block_hash,
            transactions: VariableList::from(vec![]),
            withdrawals: VariableList::from(vec![]),
            blob_gas_used: 0,
            excess_blob_gas: 0,
            withdrawals_root: B256::repeat_byte(0x06),
        };

        // Encode the payload using SSZ
        let ssz_bytes = payload.as_ssz_bytes();

        // Create a SequencerCommitment with 32-byte prefix + SSZ bytes
        let mut data = vec![0u8; 32]; // 32-byte prefix
        data.extend_from_slice(&ssz_bytes);

        let commitment = SequencerCommitment {
            data: Bytes::from(data),
            signature: Signature::new(U256::ZERO, U256::ZERO, false),
        };

        // Test fast_block_hash
        let fast_hash = commitment
            .fast_block_hash()
            .expect("fast_block_hash should succeed");
        assert_eq!(
            fast_hash, expected_block_hash,
            "fast_block_hash should match the expected block hash"
        );

        // Verify that try_from also works and returns the same hash
        let decoded_payload =
            ExecutionPayload::try_from(&commitment).expect("try_from should succeed");
        assert_eq!(
            decoded_payload.block_hash, expected_block_hash,
            "decoded block_hash should match"
        );
    }

    #[test]
    fn test_fast_block_hash_data_too_short() {
        // Create a commitment with data that's too short
        let commitment = SequencerCommitment {
            data: Bytes::from(vec![0u8; 100]), // Way too short for block_hash access
            signature: Signature::new(U256::ZERO, U256::ZERO, false),
        };

        let result = commitment.fast_block_hash();
        assert!(
            result.is_err(),
            "fast_block_hash should fail with short data"
        );
    }

    #[test]
    fn test_ssz_block_hash_offset_is_correct() {
        // This test verifies that our SSZ_BLOCK_HASH_OFFSET constant is correct
        // by checking the SSZ encoding structure.

        // Calculate expected offset based on ExecutionPayload field sizes:
        // parent_hash: 32, fee_recipient: 20, state_root: 32, receipts_root: 32,
        // logs_bloom: 256, prev_randao: 32, block_number: 8, gas_limit: 8,
        // gas_used: 8, timestamp: 8, extra_data_offset: 4, base_fee_per_gas: 32
        // Total: 32 + 20 + 32 + 32 + 256 + 32 + 8 + 8 + 8 + 8 + 4 + 32 = 472
        let expected_offset: usize = 32 + 20 + 32 + 32 + 256 + 32 + 8 + 8 + 8 + 8 + 4 + 32;
        assert_eq!(
            SSZ_BLOCK_HASH_OFFSET, expected_offset,
            "SSZ_BLOCK_HASH_OFFSET should be 472"
        );
        assert_eq!(SSZ_BLOCK_HASH_OFFSET, 472);
    }
}
