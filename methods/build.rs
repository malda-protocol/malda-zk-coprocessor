// Copyright 2023 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};
use std::{collections::HashMap, env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root_dir = manifest_dir.parent().unwrap().to_path_buf();
    let malda_rs_dir = root_dir.join("malda_rs");
    let malda_rs_src = malda_rs_dir.join("src");
    let malda_rs_bin = malda_rs_dir.join("bin");

    let mut builder = GuestOptionsBuilder::default();
    if env::var("RISC0_USE_DOCKER").is_ok() {
        let docker_options = DockerOptionsBuilder::default()
            .root_dir(manifest_dir.join("../"))
            .build()
            .unwrap();
        builder.use_docker(docker_options);
    }
    let guest_options = builder.build().unwrap();

    // Generate Rust source files for the methods crate.
    let _guests = embed_methods_with_options(HashMap::from([("guests", guest_options)]));

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Copy and rename specific files
    let methods_path = out_dir.join("methods.rs");
    let elfs_ids_path = malda_rs_src.join("elfs_ids.rs");
    fs::copy(&methods_path, &elfs_ids_path).unwrap();

    // Read elfs_ids.rs to get the original ELF paths
    let elfs_ids_content = fs::read_to_string(&elfs_ids_path).unwrap();

    // Copy the ELF files to malda_rs/bin
    if let Some(path_line) = elfs_ids_content
        .lines()
        .find(|line| line.contains("GET_PROOF_DATA_PATH"))
    {
        if let Some(path) = path_line.split('"').nth(1) {
            let source_path = PathBuf::from(path);
            let filename = source_path.file_name().unwrap();
            let dest_path = malda_rs_bin.join(filename);
            fs::copy(&source_path, &dest_path).unwrap();
            println!(
                "Copied ELF file from {} to {}",
                source_path.display(),
                dest_path.display()
            );
        }
    }

    if let Some(path_line) = elfs_ids_content
        .lines()
        .find(|line| line.contains("GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_PATH"))
    {
        if let Some(path) = path_line.split('"').nth(1) {
            let source_path = PathBuf::from(path);
            let filename = source_path.file_name().unwrap();
            let dest_path = malda_rs_bin.join(filename);
            fs::copy(&source_path, &dest_path).unwrap();
            println!(
                "Copied ELF file from {} to {}",
                source_path.display(),
                dest_path.display()
            );
        }
    }

    // Now update the paths in elfs_ids.rs to use relative paths
    let mut elfs_ids_content = elfs_ids_content.replace(
        "pub const GET_PROOF_DATA_ELF: &[u8] = &[];",
        "pub const GET_PROOF_DATA_ELF: &[u8] = include_bytes!(\"../bin/get-proof-data\");",
    );

    elfs_ids_content = elfs_ids_content.replace(
        "pub const GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_ELF: &[u8] = &[];",
        "pub const GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_ELF: &[u8] = include_bytes!(\"../bin/get_proof_data_ethereum_light_client\");"
    );

    // Extract just the filenames for the paths
    if let Some(path_line) = elfs_ids_content
        .lines()
        .find(|line| line.contains("GET_PROOF_DATA_PATH"))
    {
        if let Some(path) = path_line.split('"').nth(1) {
            let path_buf = PathBuf::from(path);
            let file_name = path_buf.file_name().unwrap();
            let filename = file_name.to_str().unwrap();
            elfs_ids_content = elfs_ids_content.replace(path, &format!("../bin/{}", filename));
        }
    }

    if let Some(path_line) = elfs_ids_content
        .lines()
        .find(|line| line.contains("GET_PROOF_DATA_ETHEREUM_LIGHT_CLIENT_PATH"))
    {
        if let Some(path) = path_line.split('"').nth(1) {
            let path_buf = PathBuf::from(path);
            let file_name = path_buf.file_name().unwrap();
            let filename = file_name.to_str().unwrap();
            elfs_ids_content = elfs_ids_content.replace(path, &format!("../bin/{}", filename));
        }
    }

    // Write the updated content back to elfs_ids.rs
    fs::write(&elfs_ids_path, elfs_ids_content).unwrap();
}
