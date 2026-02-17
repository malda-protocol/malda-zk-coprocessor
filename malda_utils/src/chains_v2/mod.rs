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
