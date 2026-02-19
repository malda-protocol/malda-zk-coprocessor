use std::{collections::BTreeMap, sync::LazyLock};

use alloy_primitives::Address;
use op_revm::spec::OpSpecId;
use risc0_op_steel::optimism::OpChainSpec;
use risc0_steel::config::{ChainSpec, ForkCondition};

use crate::constants;

// Fork timestamps sourced from:
// https://github.com/ethereum-optimism/optimism/blob/6679f0bd/rust/alloy-op-hardforks/src/base/mainnet.rs
// https://github.com/ethereum-optimism/superchain-registry/blob/main/superchain/configs/mainnet/base.toml
static BASE_MAINNET_CHAIN_SPEC: LazyLock<OpChainSpec> = LazyLock::new(|| ChainSpec {
  chain_id: 8453,
  forks: BTreeMap::from([
    (OpSpecId::BEDROCK, ForkCondition::Block(0)),
    (OpSpecId::REGOLITH, ForkCondition::Timestamp(0)),
    (OpSpecId::CANYON, ForkCondition::Timestamp(1704992401)),
    (OpSpecId::ECOTONE, ForkCondition::Timestamp(1710374401)),
    (OpSpecId::FJORD, ForkCondition::Timestamp(1720627201)),
    (OpSpecId::GRANITE, ForkCondition::Timestamp(1726070401)),
    (OpSpecId::HOLOCENE, ForkCondition::Timestamp(1736445601)),
    (OpSpecId::ISTHMUS, ForkCondition::Timestamp(1746806401)),
  ]),
});

// Fork timestamps sourced from:
// https://github.com/ethereum-optimism/optimism/blob/6679f0bd/rust/alloy-op-hardforks/src/optimism/sepolia.rs
// (Base Sepolia uses same timestamps as OP Sepolia)
static BASE_SEPOLIA_CHAIN_SPEC: LazyLock<OpChainSpec> = LazyLock::new(|| ChainSpec {
  chain_id: 84532,
  forks: BTreeMap::from([
    (OpSpecId::BEDROCK, ForkCondition::Block(0)),
    (OpSpecId::REGOLITH, ForkCondition::Timestamp(0)),
    (OpSpecId::CANYON, ForkCondition::Timestamp(1699981200)),
    (OpSpecId::ECOTONE, ForkCondition::Timestamp(1708534800)),
    (OpSpecId::FJORD, ForkCondition::Timestamp(1716998400)),
    (OpSpecId::GRANITE, ForkCondition::Timestamp(1723478400)),
    (OpSpecId::HOLOCENE, ForkCondition::Timestamp(1732633200)),
    (OpSpecId::ISTHMUS, ForkCondition::Timestamp(1744905600)),
  ]),
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseNetwork {
  Mainnet,
  Sepolia,
}

impl BaseNetwork {
  pub fn chain_spec(&self) -> &'static OpChainSpec {
    match self {
      Self::Mainnet => &BASE_MAINNET_CHAIN_SPEC,
      Self::Sepolia => &BASE_SEPOLIA_CHAIN_SPEC,
    }
  }

  pub fn portal_address(&self) -> Address {
    match self {
      Self::Mainnet => constants::BASE_PORTAL,
      Self::Sepolia => constants::BASE_SEPOLIA_PORTAL,
    }
  }

  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Mainnet => constants::REORG_PROTECTION_DEPTH_BASE,
      Self::Sepolia => constants::REORG_PROTECTION_DEPTH_BASE_SEPOLIA,
    }
  }

  pub fn rpc_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "RPC_URL_BASE",
      (Self::Mainnet, true) => "RPC_URL_BASE_FALLBACK",
      (Self::Sepolia, false) => "RPC_URL_BASE_SEPOLIA",
      (Self::Sepolia, true) => "RPC_URL_BASE_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }

  pub fn sequencer_request_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "SEQUENCER_REQUEST_BASE",
      (Self::Mainnet, true) => "SEQUENCER_REQUEST_BASE_FALLBACK",
      (Self::Sepolia, false) => "SEQUENCER_REQUEST_BASE_SEPOLIA",
      (Self::Sepolia, true) => "SEQUENCER_REQUEST_BASE_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }
}
