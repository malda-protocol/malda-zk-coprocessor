use alloy::{
    primitives::{Address, Bytes, FixedBytes, U256},
    rpc::types::Log,
};
use hex;
use serde::{Deserialize, Serialize};

type Bytes32 = FixedBytes<32>;

#[derive(Debug, Serialize, Deserialize)]
pub struct LiquidateExternalEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub user_to_liquidate: Address,
    pub receiver: Address,
    pub collateral: Address,
    pub src_chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MintExternalEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub receiver: Address,
    pub chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BorrowExternalEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepayExternalEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub position: Address,
    pub chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawExternalEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawOnExtensionChainEvent {
    pub sender: Address,
    pub dst_chain_id: u32,
    pub amount: U256,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuppliedEvent {
    pub from: Address,
    pub acc_amount_in: U256,
    pub acc_amount_out: U256,
    pub amount: U256,
    pub src_chain_id: u32,
    pub dst_chain_id: u32,
    pub linea_method_selector: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedEvent {
    pub msg_sender: Address,
    pub src_sender: Address,
    pub receiver: Address,
    pub acc_amount_in: U256,
    pub acc_amount_out: U256,
    pub amount: U256,
    pub src_chain_id: u32,
    pub dst_chain_id: u32,
}

// Event signatures as constants
pub const HOST_LIQUIDATE_EXTERNAL_SIG: &str =
    "mErc20Host_LiquidateExternal(address,address,address,address,address,uint32,uint256)";
pub const HOST_MINT_EXTERNAL_SIG: &str =
    "mErc20Host_MintExternal(address,address,address,uint32,uint256)";
pub const HOST_BORROW_EXTERNAL_SIG: &str =
    "mErc20Host_BorrowExternal(address,address,uint32,uint256)";
pub const HOST_REPAY_EXTERNAL_SIG: &str =
    "mErc20Host_RepayExternal(address,address,address,uint32,uint256)";
pub const HOST_WITHDRAW_EXTERNAL_SIG: &str =
    "mErc20Host_WithdrawExternal(address,address,uint32,uint256)";
pub const HOST_BORROW_ON_EXTENSION_CHAIN_SIG: &str =
    "mErc20Host_BorrowOnExternsionChain(address,uint32,uint256)";
pub const HOST_WITHDRAW_ON_EXTENSION_CHAIN_SIG: &str =
    "mErc20Host_WithdrawOnExtensionChain(address,uint32,uint256)";
pub const EXTENSION_SUPPLIED_SIG: &str =
    "mTokenGateway_Supplied(address,uint256,uint256,uint256,uint32,uint32,bytes4)";
pub const EXTENSION_EXTRACTED_SIG: &str =
    "mTokenGateway_Extracted(address,address,address,uint256,uint256,uint256,uint32,uint32)";

pub const MINT_EXTERNAL_SELECTOR: &str = "9d9339b3";
pub const REPAY_EXTERNAL_SELECTOR: &str = "08fee263";
pub const OUT_HERE_SELECTOR: &str = "b511d3b1";

pub const MINT_EXTERNAL_SELECTOR_FB4: &[u8] = &[0x9d, 0x93, 0x39, 0xb3];
pub const REPAY_EXTERNAL_SELECTOR_FB4: &[u8] = &[0x08, 0xfe, 0xe2, 0x63];
pub const OUT_HERE_SELECTOR_FB4: &[u8] = &[0xb5, 0x11, 0xd3, 0xb1];

// Add the parsing functions here
pub fn parse_supplied_event(log: &Log) -> SuppliedEvent {
    let from = Address::from_slice(&log.topics()[1][12..]);

    // The non-indexed parameters are packed in the data field
    let data = log.data().data.clone();

    SuppliedEvent {
        from,
        acc_amount_in: U256::from_be_slice(&data[0..32]),
        acc_amount_out: U256::from_be_slice(&data[32..64]),
        amount: U256::from_be_slice(&data[64..96]),
        src_chain_id: u32::from_be_bytes(data[124..128].try_into().unwrap()),
        dst_chain_id: u32::from_be_bytes(data[156..160].try_into().unwrap()),
        linea_method_selector: hex::encode(&data[160..164]),
    }
}

pub fn parse_withdraw_on_extension_chain_event(log: &Log) -> WithdrawOnExtensionChainEvent {
    WithdrawOnExtensionChainEvent {
        sender: Address::from_slice(&log.topics()[1][12..]),
        // Chain ID is padded to 32 bytes, we want the last 4 bytes
        dst_chain_id: u32::from_be_bytes(log.data().data[28..32].try_into().unwrap()),
        amount: U256::from_be_slice(&log.data().data[32..64]),
    }
}

pub const BATCH_PROCESS_FAILED_SIG: &str = "BatchProcessFailed(bytes32,bytes)";
pub const BATCH_PROCESS_SUCCESS_SIG: &str = "BatchProcessSuccess(bytes32)";

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchProcessFailedEvent {
    pub init_hash: Bytes32,
    pub reason: Bytes,
}

pub fn parse_batch_process_failed_event(log: &Log) -> BatchProcessFailedEvent {
    // For non-indexed events, all data is in the data field
    let data = log.data().data.clone();

    BatchProcessFailedEvent {
        init_hash: Bytes32::from_slice(data[0..32].into()),
        reason: Bytes::from(data[32..].to_vec()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchProcessSuccessEvent {
    pub init_hash: Bytes32,
}

pub fn parse_batch_process_success_event(log: &Log) -> BatchProcessSuccessEvent {
    BatchProcessSuccessEvent {
        init_hash: Bytes32::from_slice(log.data().data[0..32].into()),
    }
}
