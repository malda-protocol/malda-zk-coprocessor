//! Types module containing core data structures and implementations for blockchain payload processing.
//!
//! This module provides essential types and structures for handling blockchain execution payloads,
//! sequencer commitments, and related blockchain data structures.

use alloy_sol_types::sol;

use eyre::Result;
use serde::{Deserialize, Serialize};

use alloy_rlp::RlpEncodable;
use ssz_derive::{Decode, Encode};
use ssz_types::{FixedVector, VariableList, typenum};

use crate::cryptography::signature_msg;
use alloy_primitives::{Address, B256, Bytes, Signature, U256};

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
  fn pack(data_type: &SolidityDataType) -> Vec<u8> {
    match data_type {
      SolidityDataType::String(s) => s.as_bytes().to_vec(),
      SolidityDataType::Address(a) => a.0.to_vec(),
      SolidityDataType::Number(n) => n.to_be_bytes::<32>().to_vec(),
      SolidityDataType::Bytes(b) => b.to_vec(),
      SolidityDataType::Bool(b) => vec![*b as u8],
      SolidityDataType::NumberWithShift(n, to_take) => {
        let bytes = n.to_be_bytes::<32>();
        let to_skip = bytes.len().saturating_sub(to_take.0 / 8);
        bytes.into_iter().skip(to_skip).collect()
      }
    }
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

    // https://github.com/ethereum-optimism/optimism/blob/op-contracts/v6.0.0-rc.2/packages/contracts-bedrock/interfaces/dispute/IAnchorStateRegistry.sol
    interface IAnchorStateRegistry {
        function isGameClaimValid(address game) external view returns (bool);
    }

    // https://github.com/ethereum-optimism/optimism/blob/op-contracts/v6.0.0-rc.2/packages/contracts-bedrock/interfaces/dispute/IDisputeGameFactory.sol
    interface IDisputeGameFactory {
        function gameAtIndex(uint256 index) external view returns (uint32, uint64, address);
    }

    // https://github.com/ethereum-optimism/optimism/blob/op-contracts/v6.0.0-rc.2/packages/contracts-bedrock/interfaces/dispute/IDisputeGame.sol
    interface IDisputeGame {
        function rootClaim() external pure returns (bytes32);
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

    // https://github.com/ethereum-optimism/optimism/blob/op-contracts/v6.0.0-rc.2/packages/contracts-bedrock/interfaces/L1/IOptimismPortal2.sol
    interface IOptimismPortal2 {
        /// @notice Returns the address of the AnchorStateRegistry
        function anchorStateRegistry() external view returns (address);

        /// @notice Returns the address of the DisputeGameFactory
        function disputeGameFactory() external view returns (address);

        /// @notice Returns the timestamp when the respected game type was last updated
        function respectedGameTypeUpdatedAt() external view returns (uint64);
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
    let decompressed = decoder.decompress_vec(data)?;

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
}

/// Conversion implementation from SequencerCommitment to ExecutionPayload.
impl TryFrom<&SequencerCommitment> for ExecutionPayload {
  type Error = eyre::Report;

  /// Attempts to convert a SequencerCommitment into an ExecutionPayload.
  ///
  /// # Arguments
  /// * `value` - The SequencerCommitment to convert
  ///
  /// # Returns
  /// * `Result<Self>` - The converted payload or an error
  fn try_from(value: &SequencerCommitment) -> Result<Self> {
    let payload_bytes = &value.data[32..];
    ssz::Decode::from_ssz_bytes(payload_bytes).map_err(|_| eyre::eyre!("decode failed"))
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
