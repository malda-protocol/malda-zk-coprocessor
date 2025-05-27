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
//
//
use crate::{Sha256, Sha256Context, HASH_LEN};
use sha2::Digest;

/// Implementation of SHA256 using the `sha2` crate (fastest on x86_64 CPUs with SHA extensions).
pub struct Sha2CrateImpl;

impl Sha256Context for sha2::Sha256 {
    fn new() -> Self {
        sha2::Digest::new()
    }

    fn update(&mut self, bytes: &[u8]) {
        sha2::Digest::update(self, bytes)
    }

    fn finalize(self) -> [u8; HASH_LEN] {
        sha2::Digest::finalize(self).into()
    }
}

impl Sha256 for Sha2CrateImpl {
    type Context = sha2::Sha256;

    fn hash(&self, input: &[u8]) -> Vec<u8> {
        Self::Context::digest(input).into_iter().collect()
    }

    fn hash_fixed(&self, input: &[u8]) -> [u8; HASH_LEN] {
        Self::Context::digest(input).into()
    }
}
