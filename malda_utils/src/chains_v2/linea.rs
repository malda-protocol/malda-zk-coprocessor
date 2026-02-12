use crate::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineaNetwork {
  Mainnet,
  Sepolia,
}

impl LineaNetwork {
  pub fn reorg_protection_depth(&self) -> u64 {
    match self {
      Self::Mainnet => constants::REORG_PROTECTION_DEPTH_LINEA,
      Self::Sepolia => constants::REORG_PROTECTION_DEPTH_LINEA_SEPOLIA,
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
