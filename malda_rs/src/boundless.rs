// Copyright (c) 2025-2026 Merge Layers Inc.
//
// This source code is licensed under the Business Source License 1.1
// (the "License"); you may not use this file except in compliance with the
// License. You may obtain a copy of the License at
//
//     https://github.com/malda-protocol/malda-zk-coprocessor/blob/main/LICENSE-BSL
//
// See the License for the specific language governing permissions and
// limitations under the License.

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
