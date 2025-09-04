# Malda ZK Coprocessor

This repository contains the ZK coprocessor implementation for the Malda Protocol, a unified liquidity lending protocol that enables seamless cross-chain lending operations without traditional bridging. While this repository focuses on the ZK verification layer, it's part of a larger protocol architecture that includes smart contracts, the zk-coprocessor and the sequencer.

![Malda Protocol Flow](malda_flow.png)

## About Malda Protocol

Malda Protocol solves the fragmentation problem in DeFi by creating a unified lending experience across multiple EVM networks. The protocol enables users to:

- Access lending markets across different L2s as if they were a single network
- Unified liquidity and interest rates across all chains
- Execute lending operations across chains without bridging or wrapping assets
- Maintain full control of their funds in a self-custodial manner



## About the ZK Coprocessor

This repository contains the ZK coprocessor, a critical component that enables Malda's cross-chain capabilities through zero-knowledge proofs. The coprocessor:

- Generates zero-knowledge proofs that verify cross-chain state and operations
- Enables trustless cross-chain communication without relying on bridges or oracles
- Provides cryptographic guarantees for cross-chain lending operations

## Table of Contents

- [Architecture](#architecture)
  - [Smart Contracts](#smart-contracts)
  - [Sequencer Infrastructure](#sequencer-infrastructure)
  - [ZK Coprocessor](#zk-coprocessor)
- [Technical Details](#technical-details)
  - [Proof Generation and Verification](#proof-generation-and-verification)
  - [Chain-Specific Verification](#chain-specific-verification)
  - [Self-Sequencing](#self-sequencing)
- [Development](#development)
  - [Prerequisites](#prerequisites)
  - [Building](#building)
  - [Testing](#testing)
- [License](#license)
- [Contributing](#contributing)
- [Security](#security)

## Architecture

### ZK Coprocessor
The core component that enables trustless cross-chain operations. It:

- Generates zero-knowledge proofs for cross-chain state verification
- Provides reorg protection for all supported chains
- Enables trustless cross-chain communication without bridges

### Other Protocol Components

The ZK coprocessor works alongside the protocol's smart contracts, which handle lending operations and state management, and the Sequencer infrastructure, which monitors events and submits proofs. Despite being centralized, the Sequencer is constrained by ZK proofs and cannot manipulate user funds, ensuring the protocol remains self-custodial. For censorship resistance, users can [generate their own proofs](#self-sequencing) if needed.

## Technical Details

### Proof Generation and Verification

The ZK coprocessor generates and verifies proofs through several key components:

#### Proof Data Generation
The `get_proof_data` functionality is central to the protocol's cross-chain operations:

1. **Guest Program**
   - Executes inside the RISC Zero zkVM
   - Verifies cross-chain state through zero-knowledge proofs
   - Validates user positions and market states across chains
   - Generates cryptographic proofs of state verification
   - Handles view calls to market contracts for state verification

#### Chain-Specific Verification

Each supported chain has specialized verification mechanisms:

1. **Ethereum (L1)**
   - Light client verification through beacon chain
   - Proof state via OPstack L1 reads

2. **Optimism/Base (OpStack)**
   - Sequencer commitment verification
   - Dispute game validation
   - L1 block inclusion proofs

3. **Linea**
   - Sequencer commitment verification
   - L1 block inclusion proofs

### Ethereum Light Client Implementation

This repository includes an experimental implementation of an Ethereum light client that demonstrates how light client verification can be integrated with Steel view calls for cross-chain proofs. **Note: This is not fully integrated into the main Malda protocol flow but serves as an example of how light client verification should work and how it can be connected with Steel for view calls on Ethereum to any chain proof.**

#### How the Light Client Works

The Ethereum light client implementation builds the Ethereum chain from sync committee verification:

1. **Trusted Hash Starting Point**: The light client starts from a trusted beacon chain block hash (checkpoint) that serves as the root of trust
2. **Sync Committee Verification**: Uses Ethereum's sync committee mechanism to verify subsequent beacon chain blocks
3. **Chain Building**: Constructs a verified chain of beacon blocks leading to the target execution block
4. **Execution Block Verification**: Verifies the execution payload within the beacon block to access Ethereum state
5. **View Call Execution**: Performs the actual view call to the Malda market contract using the verified state

#### Key Differences from Other Proofs

Unlike the other chain verification mechanisms in this protocol, the light client proof:
- **Does not use batching**: Each proof verifies a single user's state rather than multiple users
- **Requires trusted checkpoints**: Depends on a trusted beacon chain hash that must be updated regularly
- **Uses BLS signature verification**: Leverages the RISC Zero BLS318 accelerator for sync aggregation signature verification

#### Dependencies and Compatibility

The implementation is based on the Helios light client but has been modified for compatibility with the Steel framework:
- **Helios-Steel-Compatible**: Uses a modified version of Helios dependencies located in the `helios-steel-compatible` repository under the Malda organization
- **Data Type Adaptations**: Some data types and structures have been adapted for compatibility with the RISC Zero zkVM environment
- **BLS Acceleration**: Utilizes RISC Zero's BLS318 accelerator for efficient sync committee signature verification

#### Testing the Light Client

You can test the light client implementation using:

```bash
cargo test prove_get_proof_data_on_ethereum_via_light_client
```

**Important**: The trusted hash in the test needs to be updated to a recent beacon checkpoint to prevent the verification chain from becoming too long, which would impact proof generation performance.

#### Integration Requirements

To fully integrate this light client into the Malda protocol, the following modifications would be needed:
- **Anchor State Registry**: Malda contracts would need to be modified to include an anchor state registry for managing trusted beacon chain checkpoints
- **Checkpoint Management**: A mechanism for updating trusted checkpoints as the beacon chain progresses
- **Proof Verification**: Integration of the light client proof verification into the main protocol flow

This implementation demonstrates the feasibility of using Ethereum light clients for cross-chain verification while maintaining the security guarantees of the beacon chain consensus mechanism.

### Self-Sequencing

While the [Sequencer Infrastructure](#sequencer-infrastructure) handles proof generation and submission for most users, the protocol maintains censorship resistance through self-sequencing capabilities. Users can generate and submit their own proofs if:

- The Sequencer is unavailable
- The Sequencer attempts to censor transactions
- Users prefer to handle their own proof generation
- Additional security guarantees are required

#### Self-Sequencing Guide

To generate proofs independently:

1. **Setup**
   ```bash
   # Install RISC Zero zkVM and Bonsai SDK
   For detailed installation instructions, see the [RISC Zero documentation](https://dev.risczero.com/api/zkvm/install).
   Also to request proof in boundless market, look at the setup here: (https://docs.beboundless.xyz/developers/what).

2. **Environment Configuration**
   Create a `.env` file with required RPC endpoints:

   ```env
   # --- Mainnet RPC Endpoints ---
   RPC_URL_LINEA=<YOUR_LINEA_MAINNET_RPC_URL>
   RPC_URL_ETHEREUM=<YOUR_ETHEREUM_MAINNET_RPC_URL>
   RPC_URL_BASE=<YOUR_BASE_MAINNET_RPC_URL>
   RPC_URL_OPTIMISM=<YOUR_OPTIMISM_MAINNET_RPC_URL>
   RPC_URL_BEACON=https://www.lightclientdata.org   # Public Ethereum Beacon chain endpoint

   # --- Mainnet Fallback RPC Endpoints ---
   RPC_URL_LINEA_FALLBACK=<YOUR_LINEA_MAINNET_FALLBACK_RPC_URL>
   RPC_URL_ETHEREUM_FALLBACK=<YOUR_ETHEREUM_MAINNET_FALLBACK_RPC_URL>
   RPC_URL_BASE_FALLBACK=<YOUR_BASE_MAINNET_FALLBACK_RPC_URL>
   RPC_URL_OPTIMISM_FALLBACK=<YOUR_OPTIMISM_MAINNET_FALLBACK_RPC_URL>

   # --- Sepolia (Testnet) RPC Endpoints ---
   RPC_URL_LINEA_SEPOLIA=<YOUR_LINEA_SEPOLIA_RPC_URL>
   RPC_URL_ETHEREUM_SEPOLIA=<YOUR_ETHEREUM_SEPOLIA_RPC_URL>
   RPC_URL_BASE_SEPOLIA=<YOUR_BASE_SEPOLIA_RPC_URL>
   RPC_URL_OPTIMISM_SEPOLIA=<YOUR_OPTIMISM_SEPOLIA_RPC_URL>
   RPC_URL_LINEA_SEPOLIA_FALLBACK=<YOUR_LINEA_SEPOLIA_FALLBACK_RPC_URL>
   RPC_URL_ETHEREUM_SEPOLIA_FALLBACK=<YOUR_ETHEREUM_SEPOLIA_FALLBACK_RPC_URL>
   RPC_URL_BASE_SEPOLIA_FALLBACK=<YOUR_BASE_SEPOLIA_FALLBACK_RPC_URL>
   RPC_URL_OPTIMISM_SEPOLIA_FALLBACK=<YOUR_OPTIMISM_SEPOLIA_FALLBACK_RPC_URL>

   # --- Sequencer Commitment Endpoints (Operation Solarstorm, public) ---
   SEQUENCER_REQUEST_OPTIMISM=https://optimism.operationsolarstorm.org/latest
   SEQUENCER_REQUEST_OPTIMISM_FALLBACK=https://optimism.operationsolarstorm.org/latest
   SEQUENCER_REQUEST_BASE=https://base.operationsolarstorm.org/latest
   SEQUENCER_REQUEST_BASE_FALLBACK=https://base.operationsolarstorm.org/latest
 

   # --- ZK Prover via Bonsai ---

   IMAGE_ID_BONSAI=<YOUR_BONSAI_IMAGE_ID>        # Bonsai image ID for ZK proof generation
   BONSAI_API_KEY=<YOUR_BONSAI_API_KEY>
   BONSAI_API_URL=<YOUR_BONSAI_API_URL>

   # --- ZK Prover via Boundless Market ---
   PINATA_JWT=<YOUR_PINATA_JWT>                  # Pinata JWT for IPFS uploads (if required)
   PROGRAM_URL=<YOUR_PROGRAM_URL>                # Program URL
   PRIVATE_KEY=<YOUR_PRIVATE_KEY>                # Private key for proof request on boundless market on BASE
   RPC_URL=<YOUR_RPC_URL>                        # RPC URL for BASE mainnet
   ```

3. **Proof Generation**
   Use the Malda SDK to generate proofs:
   ```rust
   // Using Bonsai SDK for remote proving
   pub async fn get_proof_data_prove_sdk(
       users: Vec<Vec<Address>>,
       markets: Vec<Vec<Address>>,
       target_chain_ids: Vec<Vec<u64>>,
       chain_ids: Vec<u64>,
       l1_inclusion: bool,
   ) -> Result<MaldaProveInfo, Error>

   // Using local zkVM for proving
   pub async fn get_proof_data_prove(
       users: Vec<Vec<Address>>,
       markets: Vec<Vec<Address>>,
       target_chain_ids: Vec<Vec<u64>>,
       chain_ids: Vec<Vec<u64>>,
       l1_inclusion: bool,
   ) -> Result<MaldaProveInfo, Error>
   ```

4. **Transaction Preparation**
   Extract the required data for on-chain submission:
   ```rust
   let journal = Bytes::from(proof_info.receipt.journal.bytes);
   let seal = risc0_ethereum_contracts::encode_seal(&receipt);
   ```

Note: For self-sequencing, `l1_inclusion` must be set to `true` to ensure additional security guarantees against potential reorg exploits.


## Development

### Prerequisites

- Rust toolchain
- RISC Zero toolchain
- Access to RPC endpoints for supported chains

### Building

```bash
cargo build
```

### Environment Setup for Testing

Before running tests, you must configure your environment variables. Create and fill out a `.env` file as described in the [Environment Configuration](#environment-configuration) section above. Ensure that all required RPC endpoints and `SEQUENCER_REQUEST` URLs are set for the relevant networks.

- **RPC Endpoints:** You must provide valid RPC URLs for Ethereum, Linea, Base, and Optimism (both mainnet and testnet, as needed).
- **Sequencer Commitment Endpoints:** The `SEQUENCER_REQUEST` variables for Optimism and Base must be set to enable proof data retrieval.

#### Integration Test Details

The test suite uses integration tests that interact with real contract deployments on both mainnet and testnet. This is necessary to obtain authentic Merkle proof data for proof building. However, note that tests targeting Sepolia (testnet) are marked as `ignore` by default. This is because the sequencer commitment endpoints for Optimism and Base testnets are not publicly available, making it impossible to fetch the required data for those tests.

### Testing

```bash
cargo test
```

## License

This project is licensed under the Business Source License 1.1. See [LICENSE-BSL](LICENSE-BSL) for details.

## Contributing

Please read our contributing guidelines before submitting pull requests.

## Security

For security concerns, please contact the team through our security contact channels.
