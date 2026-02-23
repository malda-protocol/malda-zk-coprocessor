// Copyright (c) 2025 Merge Layers Inc.
//
// This source code is licensed under the Business Source License 1.1
// (the "License"); you may not use this file except in compliance with the
// License. You may obtain a copy of the License at
//
//     https://github.com/malda-protocol/malda-zk-coprocessor/blob/main/LICENSE-BSL
//
// See the License for the specific language governing permissions and
// limitations under the License.

//! Chain ID helper functions for multi-chain support.
//!
//! This module provides utility functions for working with different blockchain networks,
//! including reorg protection depth lookups and chain type identification.

use crate::constants::*;
