use crate::constants;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseNetwork {
  Mainnet,
  Sepolia,
}

impl BaseNetwork {
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
