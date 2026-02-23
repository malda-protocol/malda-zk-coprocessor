use alloy_primitives::Address;

use crate::constants;
use crate::types::{EthChainSpec, LINEA_MAINNET_CHAIN_SPEC, LINEA_SEPOLIA_CHAIN_SPEC};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineaNetwork {
  Mainnet,
  Sepolia,
}

impl LineaNetwork {
  pub fn l1_ethereum_network(&self) -> super::EthereumNetwork {
    match self {
      Self::Mainnet => super::EthereumNetwork::Mainnet,
      Self::Sepolia => super::EthereumNetwork::Sepolia,
    }
  }

  pub fn chain_spec(&self) -> &'static EthChainSpec {
    match self {
      Self::Mainnet => &LINEA_MAINNET_CHAIN_SPEC,
      Self::Sepolia => &LINEA_SEPOLIA_CHAIN_SPEC,
    }
  }

  pub fn message_service_address(&self) -> Address {
    match self {
      Self::Mainnet => constants::L1_MESSAGE_SERVICE_LINEA,
      Self::Sepolia => constants::L1_MESSAGE_SERVICE_LINEA_SEPOLIA,
    }
  }

  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Mainnet => constants::REORG_PROTECTION_DEPTH_LINEA,
      Self::Sepolia => constants::REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
    }
  }

  pub fn beacon_api_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "BEACON_API_URL_LINEA",
      (Self::Mainnet, true) => "BEACON_API_URL_LINEA_FALLBACK",
      (Self::Sepolia, false) => "BEACON_API_URL_LINEA_SEPOLIA",
      (Self::Sepolia, true) => "BEACON_API_URL_LINEA_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }

  pub fn block_verifier_network(&self) -> linea_block_verifier::core::constants::LineaNetwork {
    match self {
      Self::Mainnet => linea_block_verifier::core::constants::LineaNetwork::Mainnet,
      Self::Sepolia => linea_block_verifier::core::constants::LineaNetwork::Sepolia,
    }
  }

  pub fn rpc_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "RPC_URL_LINEA",
      (Self::Mainnet, true) => "RPC_URL_LINEA_FALLBACK",
      (Self::Sepolia, false) => "RPC_URL_LINEA_SEPOLIA",
      (Self::Sepolia, true) => "RPC_URL_LINEA_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }
}
