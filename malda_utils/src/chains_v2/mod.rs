mod base;
mod ethereum;
mod linea;
mod optimism;

pub use base::BaseNetwork;
pub use ethereum::EthereumNetwork;
pub use linea::LineaNetwork;
pub use optimism::OptimismNetwork;

use crate::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chain {
  Base(BaseNetwork),
  Ethereum(EthereumNetwork),
  Linea(LineaNetwork),
  Optimism(OptimismNetwork),
}

impl Chain {
  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Base(n) => n.reorg_protection_depth(),
      Self::Ethereum(n) => n.reorg_protection_depth(),
      Self::Linea(n) => n.reorg_protection_depth(),
      Self::Optimism(n) => n.reorg_protection_depth(),
    }
  }

  /// Returns the L1 Ethereum network for this chain.
  ///
  /// Panics if called on an Ethereum chain (which is already L1).
  pub fn l1_ethereum_network(&self) -> EthereumNetwork {
    match self {
      Self::Base(n) => n.l1_ethereum_network(),
      Self::Linea(n) => n.l1_ethereum_network(),
      Self::Optimism(n) => n.l1_ethereum_network(),
      Self::Ethereum(_) => panic!("l1_ethereum_network: Ethereum is already L1"),
    }
  }

  pub fn rpc_url(&self, fallback: bool) -> String {
    match self {
      Self::Base(n) => n.rpc_url(fallback),
      Self::Ethereum(n) => n.rpc_url(fallback),
      Self::Linea(n) => n.rpc_url(fallback),
      Self::Optimism(n) => n.rpc_url(fallback),
    }
  }
}

impl TryFrom<u64> for Chain {
  type Error = String;

  fn try_from(chain_id: u64) -> Result<Self, Self::Error> {
    match chain_id {
      constants::ETHEREUM_CHAIN_ID => Ok(Self::Ethereum(EthereumNetwork::Mainnet)),
      constants::ETHEREUM_SEPOLIA_CHAIN_ID => Ok(Self::Ethereum(EthereumNetwork::Sepolia)),
      constants::OPTIMISM_CHAIN_ID => Ok(Self::Optimism(OptimismNetwork::Mainnet)),
      constants::OPTIMISM_SEPOLIA_CHAIN_ID => Ok(Self::Optimism(OptimismNetwork::Sepolia)),
      constants::BASE_CHAIN_ID => Ok(Self::Base(BaseNetwork::Mainnet)),
      constants::BASE_SEPOLIA_CHAIN_ID => Ok(Self::Base(BaseNetwork::Sepolia)),
      constants::LINEA_CHAIN_ID => Ok(Self::Linea(LineaNetwork::Mainnet)),
      constants::LINEA_SEPOLIA_CHAIN_ID => Ok(Self::Linea(LineaNetwork::Sepolia)),
      _ => Err(format!("Unsupported chain ID: {chain_id}")),
    }
  }
}
