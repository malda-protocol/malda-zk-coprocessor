# Rust SDK for the Malda protocol

Code for host/client and zkVM guest program including constants,
view calls, cryptographic operations, type definitions, and validation logic.

Generate docs:

```bash
cargo doc --no-deps
```


Self-Sequencing guide (How to generate a proof for Malda cross-chain actions)

Malda uses zero-knowledge proofs for secure cross-chain transactions that are based on risc0 zkVM. To follow this guide, a basic understanding of risc0 and their tools is required. Please refer to https://dev.risczero.com/api for information about installation, local proving and remote proving with bonsai.

1)
Install risc0 zkvm and bonsai sdk:

https://dev.risczero.com/api/zkvm/install

2)
Create .env file with the following if relevant. Some public defaults are provided:

RPC_URL_LINEA=
RPC_URL_ETHEREUM=
RPC_URL_BASE=
RPC_URL_OPTIMISM=
RPC_URL_ARBITRUM=


RPC_URL_LINEA_SEPOLIA=  
RPC_URL_ETHEREUM_SEPOLIA=
RPC_URL_BASE_SEPOLIA=
RPC_URL_OPTIMISM_SEPOLIA=
RPC_URL_ARBITRUM_SEPOLIA=

RPC_URL_BEACON=https://www.lightclientdata.org

SEQUENCER_REQUEST_OPTIMISM=https://optimism.operationsolarstorm.org/latest
SEQUENCER_REQUEST_BASE=https://base.operationsolarstorm.org/latest
SEQUENCER_REQUEST_OPTIMISM_SEPOLIA=
SEQUENCER_REQUEST_BASE_SEPOLIA=

IMAGE_ID_BONSAI=d6d8248d1e786f29a2523024755fec278834380b35606307682d1411b65adba6 (current one from the protocol)

3)
Malda_rs sdk to generate proof from simple inputs. There are two functions get_proof_data_prove(_sdk), which have the same input and output, but get_proof_data_prove_sdk is using the bonsaisdk to generate the proof and get_proof_data_prove is using a local zkvm + prover to generate the proof. 

pub async fn get_proof_data_prove_sdk(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Result<MaldaProveInfo, Error>

pub async fn get_proof_data_prove(
    users: Vec<Vec<Address>>,
    markets: Vec<Vec<Address>>,
    target_chain_ids: Vec<Vec<u64>>,
    chain_ids: Vec<u64>,
    l1_inclusion: bool,
) -> Result<MaldaProveInfo, Error>

pub struct MaldaProveInfo {
    pub receipt: Receipt,
    pub stats: MaldaSessionStats,
    pub uuid: String,
    pub stark_time: u64,
    pub snark_time: u64,
}

These functions can generate batch proofs for multiple users, chains and markets and therefore takes vectors as inputs. For a single user and chain all the vectors have a single element. For self-sequencing, l1_inclusion needs to be set to true as we require additional security guarantees about the proven chain state to avoid potential reorg exploits.

The data needed for an onchain transaction is the journal (data with the users malda balance) and the seal which guarantees this data is valid. From the output of the above functions they can be generated like this:

let journal = Bytes::from(proof_info.receipt.journal.bytes);
let seal = match risc0_ethereum_contracts::encode_seal(&receipt)

