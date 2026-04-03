use boundless_market::storage::{StandardUploader, StorageError, StorageUploaderConfig};
use clap::Parser;

/// Parse [StorageUploaderConfig] from environment variables using clap,
/// then construct a [StandardUploader] from it.
///
/// Replaces `storage_provider_from_env()` which was removed in boundless-market v1.3.0.
pub async fn storage_provider_from_env() -> Result<StandardUploader, StorageError> {
  #[derive(Parser)]
  struct StorageEnv {
    #[command(flatten)]
    storage: StorageUploaderConfig,
  }

  let config = if std::env::var_os("RISC0_DEV_MODE").is_some() {
    StorageUploaderConfig::dev_mode()
  } else {
    StorageEnv::try_parse()
      .map_err(|e| StorageError::Other(e.into()))?
      .storage
  };

  StandardUploader::from_config(&config).await
}
