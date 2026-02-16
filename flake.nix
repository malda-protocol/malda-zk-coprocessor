{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    risc0pkgs.url = "github:malda-protocol/risc0pkgs";
    risc0pkgs.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      risc0pkgs,
      treefmt-nix,
      advisory-db,
      ...
    }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      treefmtEval = forAllSystems (
        system:
        treefmt-nix.lib.evalModule nixpkgs.legacyPackages.${system} {
          projectRootFile = "flake.nix";
          programs.nixfmt.enable = true;
          programs.rustfmt.enable = true;
          programs.rustfmt.edition = "2024";
        }
      );
    in
    {
      formatter = forAllSystems (system: treefmtEval.${system}.config.build.wrapper);

      checks = forAllSystems (system: {
        formatting = treefmtEval.${system}.config.build.check self;
        audit =
          nixpkgs.legacyPackages.${system}.runCommand "cargo-audit"
            {
              buildInputs = [ nixpkgs.legacyPackages.${system}.cargo-audit ];
            }
            ''
              # Vulnerabilities that we allow for now:
              #   RUSTSEC-2023-0071 (rsa) - see https://github.com/malda-protocol/malda-zk-coprocessor/issues/39
              #   RUSTSEC-2025-0055 (tracing-subscriber) - see https://github.com/malda-protocol/malda-zk-coprocessor/issues/40
              IGNORE="--ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2025-0055"
              GUEST_IGNORE="--ignore RUSTSEC-2025-0055"
              cargo-audit audit --no-fetch $IGNORE --db ${advisory-db} --file ${./Cargo.lock}
              cargo-audit audit --no-fetch $GUEST_IGNORE --db ${advisory-db} --file ${./methods/guest/Cargo.lock}
              touch $out
            '';
      });

      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ risc0pkgs.overlays.default ];
          };
        in
        {
          guest = pkgs.buildRisc0Guest {
            pname = "guests";
            src = ./.;
            postUnpack = "sourceRoot=$sourceRoot/methods/guest";
            RISC0_FEATURE_bigint2 = "";
            cargoLock = {
              lockFile = ./methods/guest/Cargo.lock;
              outputHashes = {
                "bls12_381-0.8.0" = "sha256-HyNhhVuMV7IC7n7nEV8s11MS1LWGUkSQj16ACvbRjfI=";
                "c-kzg-2.1.1" = "sha256-X5eGa61jioNaNragk1RlwBGfiJ10XGSmD3dYQPSTGsk=";
                "crypto-bigint-0.5.5" = "sha256-7kCaAgyJKOD5C7Av0po+NMqpNgRoA478URwOK7VF7Mc=";
                "k256-0.13.4" = "sha256-aO1qewUyopojjJrzLA7BGddfBtcepyiMOrO27RmSM5E=";
                "linea-block-verifier-0.1.0" = "sha256-nnai6DpKmypP06n9KAjwWGCN7ZtlK0uOKI8csOcrxrU=";
                "risc0-steel-2.4.1" = "sha256-fFsds95M8u2jjfFZ+M3AuX3CzwKG3XYsLgk0Bk32ras=";
                "tiny-keccak-2.0.2" = "sha256-YTmdBgqbFwVJlId5efXAZBqS4JQptmqDUXImGPkH/48=";
              };
            };
          };

          default = pkgs.buildRisc0Host {
            pname = "methods";
            src = ./.;
            buildAndTestSubdir = "methods";
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "ethereum-consensus-0.1.1" = "sha256-OFiLv71Ah3CNBP1cLk0NiLPlxrFpRzZe2RTB7UnweBQ=";
                "linea-block-verifier-0.1.0" = "sha256-nnai6DpKmypP06n9KAjwWGCN7ZtlK0uOKI8csOcrxrU=";
                "risc0-steel-2.4.1" = "sha256-fFsds95M8u2jjfFZ+M3AuX3CzwKG3XYsLgk0Bk32ras=";
                "ssz_rs-0.9.0" = "sha256-rQ+UEOvwa8Gr8DyJKTe4JGnAHaoeFi3VVaxB+oSiepA=";
              };
            };
            guests = [ self.packages.${system}.guest ];
          };

          # NOTE: outputHashes duplicated with `default` because methods and
          # malda_rs share the root Cargo.lock (same workspace). Once methods
          # moves to its own workspace this duplication goes away.
          malda_rs = pkgs.rustPlatform.buildRustPackage {
            pname = "malda_rs";
            version = "0.1.0";
            src = ./.;
            buildAndTestSubdir = "malda_rs";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            # risc0-steel uses include_str!("../../../README.md") relative to vendored source
            preBuild = "echo '# Vendored crate' > /build/README.md";
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "ethereum-consensus-0.1.1" = "sha256-OFiLv71Ah3CNBP1cLk0NiLPlxrFpRzZe2RTB7UnweBQ=";
                "linea-block-verifier-0.1.0" = "sha256-nnai6DpKmypP06n9KAjwWGCN7ZtlK0uOKI8csOcrxrU=";
                "risc0-steel-2.4.1" = "sha256-fFsds95M8u2jjfFZ+M3AuX3CzwKG3XYsLgk0Bk32ras=";
                "ssz_rs-0.9.0" = "sha256-rQ+UEOvwa8Gr8DyJKTe4JGnAHaoeFi3VVaxB+oSiepA=";
              };
            };
            doCheck = false;
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ risc0pkgs.overlays.default ];
          };

          rustVersion = pkgs.lib.removePrefix "r0." pkgs.risc0-rust.version;
          arch =
            {
              x86_64-linux = "x86_64-unknown-linux-gnu";
              aarch64-linux = "aarch64-unknown-linux-gnu";
              aarch64-darwin = "aarch64-apple-darwin";
              x86_64-darwin = "x86_64-apple-darwin";
            }
            .${system};
          toolchainName = "v${rustVersion}-rust-${arch}";
        in
        {
          default = pkgs.mkShell {
            RISC0_FEATURE_bigint2 = "";

            nativeBuildInputs = [
              pkgs.cargo
              pkgs.risc0-rust
              pkgs.r0vm
              pkgs.riscv32-cc
            ];

            shellHook = ''
              # Set up risc0 toolchain in expected location using symlinks.
              mkdir -p $HOME/.risc0/toolchains/${toolchainName}
              ln -sfn ${pkgs.risc0-rust}/bin $HOME/.risc0/toolchains/${toolchainName}/bin
              ln -sfn ${pkgs.risc0-rust}/lib $HOME/.risc0/toolchains/${toolchainName}/lib

              # Create settings.toml with default rust version
              printf '[default_versions]\nrust = "%s"\n' "${rustVersion}" > $HOME/.risc0/settings.toml

              # Set C/C++ cross-compiler for guest code (used by cc-rs in build.rs)
              export CC_riscv32im_risc0_zkvm_elf=${pkgs.riscv32-cc}/bin/${pkgs.riscv32-cc.targetPrefix}gcc
              export CXX_riscv32im_risc0_zkvm_elf=${pkgs.riscv32-cc}/bin/${pkgs.riscv32-cc.targetPrefix}g++
              export AR_riscv32im_risc0_zkvm_elf=${pkgs.riscv32-cc}/bin/${pkgs.riscv32-cc.targetPrefix}ar
            '';
          };
        }
      );
    };
}
