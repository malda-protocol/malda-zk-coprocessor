use alloy_primitives::Address;
use risc0_op_steel::optimism::{OP_MAINNET_CHAIN_SPEC, OP_SEPOLIA_CHAIN_SPEC, OpChainSpec};

use crate::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimismNetwork {
  Mainnet,
  Sepolia,
}

impl OptimismNetwork {
  pub fn chain_spec(&self) -> &'static OpChainSpec {
    match self {
      Self::Mainnet => &OP_MAINNET_CHAIN_SPEC,
      Self::Sepolia => &OP_SEPOLIA_CHAIN_SPEC,
    }
  }

  pub fn chain_id(&self) -> u64 {
    match self {
      Self::Mainnet => constants::OPTIMISM_CHAIN_ID,
      Self::Sepolia => constants::OPTIMISM_SEPOLIA_CHAIN_ID,
    }
  }

  pub fn sequencer_address(&self) -> Address {
    match self {
      Self::Mainnet => constants::OPTIMISM_SEQUENCER,
      Self::Sepolia => constants::OPTIMISM_SEPOLIA_SEQUENCER,
    }
  }

  pub fn portal_address(&self) -> Address {
    match self {
      Self::Mainnet => constants::OPTIMISM_PORTAL,
      Self::Sepolia => constants::OPTIMISM_SEPOLIA_PORTAL,
    }
  }

  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Mainnet => constants::REORG_PROTECTION_DEPTH_OPTIMISM,
      Self::Sepolia => constants::REORG_PROTECTION_DEPTH_OPTIMISM_SEPOLIA,
    }
  }

  pub fn rpc_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "RPC_URL_OPTIMISM",
      (Self::Mainnet, true) => "RPC_URL_OPTIMISM_FALLBACK",
      (Self::Sepolia, false) => "RPC_URL_OPTIMISM_SEPOLIA",
      (Self::Sepolia, true) => "RPC_URL_OPTIMISM_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }

  pub fn sequencer_request_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "SEQUENCER_REQUEST_OPTIMISM",
      (Self::Mainnet, true) => "SEQUENCER_REQUEST_OPTIMISM_FALLBACK",
      (Self::Sepolia, false) => "SEQUENCER_REQUEST_OPTIMISM_SEPOLIA",
      (Self::Sepolia, true) => "SEQUENCER_REQUEST_OPTIMISM_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }
}
