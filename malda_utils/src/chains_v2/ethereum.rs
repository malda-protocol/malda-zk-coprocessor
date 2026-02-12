use crate::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthereumNetwork {
  Mainnet,
  Sepolia,
}

impl EthereumNetwork {
  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Mainnet => constants::REORG_PROTECTION_DEPTH_ETHEREUM,
      Self::Sepolia => constants::REORG_PROTECTION_DEPTH_ETHEREUM_SEPOLIA,
    }
  }

  pub fn rpc_url(&self, fallback: bool) -> String {
    let key = match (self, fallback) {
      (Self::Mainnet, false) => "RPC_URL_ETHEREUM",
      (Self::Mainnet, true) => "RPC_URL_ETHEREUM_FALLBACK",
      (Self::Sepolia, false) => "RPC_URL_ETHEREUM_SEPOLIA",
      (Self::Sepolia, true) => "RPC_URL_ETHEREUM_SEPOLIA_FALLBACK",
    };
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set in environment"))
  }
}
