//! どこで: chain_data のReceipt / 何を: 最小結果 + logs の保存 / なぜ: 互換性と観測のため

use crate::chain_data::constants::{
    HASH_LEN, MAX_LOG_DATA, MAX_LOGS_PER_TX, MAX_LOG_TOPICS, MAX_RETURN_DATA,
    RECEIPT_CONTRACT_ADDR_LEN, RECEIPT_MAX_SIZE_U32,
};
use crate::chain_data::tx::TxId;
use ic_stable_structures::storable::Bound;
use ic_stable_structures::Storable;
use std::borrow::Cow;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogEntry {
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptLike {
    pub tx_id: TxId,
    pub block_number: u64,
    pub tx_index: u32,
    pub status: u8,
    pub gas_used: u64,
    pub effective_gas_price: u64,
    pub return_data_hash: [u8; HASH_LEN],
    pub return_data: Vec<u8>,
    pub contract_address: Option<[u8; RECEIPT_CONTRACT_ADDR_LEN]>,
    pub logs: Vec<LogEntry>,
}

impl Storable for ReceiptLike {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        if self.return_data.len() > MAX_RETURN_DATA {
            ic_cdk::trap("receipt: return_data too large");
        }
        if self.logs.len() > MAX_LOGS_PER_TX {
            ic_cdk::trap("receipt: too many logs");
        }
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&self.tx_id.0);
        out.extend_from_slice(&self.block_number.to_be_bytes());
        out.extend_from_slice(&self.tx_index.to_be_bytes());
        out.push(self.status);
        out.extend_from_slice(&self.gas_used.to_be_bytes());
        out.extend_from_slice(&self.effective_gas_price.to_be_bytes());
        out.extend_from_slice(&self.return_data_hash);
        let data_len = u32::try_from(self.return_data.len())
            .unwrap_or_else(|_| ic_cdk::trap("receipt: return_data len"));
        out.extend_from_slice(&data_len.to_be_bytes());
        out.extend_from_slice(&self.return_data);
        match self.contract_address {
            Some(addr) => {
                out.push(1);
                out.extend_from_slice(&addr);
            }
            None => {
                out.push(0);
                out.extend_from_slice(&[0u8; RECEIPT_CONTRACT_ADDR_LEN]);
            }
        }
        let logs_len = u32::try_from(self.logs.len())
            .unwrap_or_else(|_| ic_cdk::trap("receipt: logs len"));
        out.extend_from_slice(&logs_len.to_be_bytes());
        for log in self.logs.iter() {
            if log.topics.len() > MAX_LOG_TOPICS {
                ic_cdk::trap("receipt: too many topics");
            }
            if log.data.len() > MAX_LOG_DATA {
                ic_cdk::trap("receipt: log data too large");
            }
            out.extend_from_slice(&log.address);
            let topics_len = u32::try_from(log.topics.len())
                .unwrap_or_else(|_| ic_cdk::trap("receipt: topics len"));
            out.extend_from_slice(&topics_len.to_be_bytes());
            for topic in log.topics.iter() {
                out.extend_from_slice(topic);
            }
            let data_len = u32::try_from(log.data.len())
                .unwrap_or_else(|_| ic_cdk::trap("receipt: log data len"));
            out.extend_from_slice(&data_len.to_be_bytes());
            out.extend_from_slice(&log.data);
        }
        Cow::Owned(out)
    }

    fn into_bytes(self) -> Vec<u8> {
        self.to_bytes().into_owned()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        let data = bytes.as_ref();
        if data.len() > RECEIPT_MAX_SIZE_U32 as usize {
            ic_cdk::trap("receipt: invalid length");
        }
        let mut offset = 0;
        let mut tx_id = [0u8; 32];
        tx_id.copy_from_slice(&data[offset..offset + 32]);
        offset += 32;
        let mut bn = [0u8; 8];
        bn.copy_from_slice(&data[offset..offset + 8]);
        offset += 8;
        let mut ti = [0u8; 4];
        ti.copy_from_slice(&data[offset..offset + 4]);
        offset += 4;
        let status = data[offset];
        offset += 1;
        let mut gas = [0u8; 8];
        gas.copy_from_slice(&data[offset..offset + 8]);
        offset += 8;
        let mut gas_price = [0u8; 8];
        gas_price.copy_from_slice(&data[offset..offset + 8]);
        offset += 8;
        let mut ret = [0u8; 32];
        ret.copy_from_slice(&data[offset..offset + 32]);
        offset += 32;
        let mut return_len = [0u8; 4];
        return_len.copy_from_slice(&data[offset..offset + 4]);
        offset += 4;
        let return_len = u32::from_be_bytes(return_len) as usize;
        if return_len > MAX_RETURN_DATA {
            ic_cdk::trap("receipt: return_data too large");
        }
        let return_end = offset + return_len;
        if return_end > data.len() {
            ic_cdk::trap("receipt: invalid return_data length");
        }
        let return_data = data[offset..return_end].to_vec();
        offset = return_end;
        let has_addr = data[offset];
        offset += 1;
        let mut addr = [0u8; RECEIPT_CONTRACT_ADDR_LEN];
        addr.copy_from_slice(&data[offset..offset + RECEIPT_CONTRACT_ADDR_LEN]);
        let contract_address = if has_addr == 1 { Some(addr) } else { None };
        offset += RECEIPT_CONTRACT_ADDR_LEN;
        let mut logs_len = [0u8; 4];
        logs_len.copy_from_slice(&data[offset..offset + 4]);
        offset += 4;
        let logs_len = u32::from_be_bytes(logs_len) as usize;
        if logs_len > MAX_LOGS_PER_TX {
            ic_cdk::trap("receipt: too many logs");
        }
        let mut logs = Vec::with_capacity(logs_len);
        for _ in 0..logs_len {
            if offset + 20 + 4 > data.len() {
                ic_cdk::trap("receipt: log truncated");
            }
            let mut address = [0u8; 20];
            address.copy_from_slice(&data[offset..offset + 20]);
            offset += 20;
            let mut topics_len = [0u8; 4];
            topics_len.copy_from_slice(&data[offset..offset + 4]);
            offset += 4;
            let topics_len = u32::from_be_bytes(topics_len) as usize;
            if topics_len > MAX_LOG_TOPICS {
                ic_cdk::trap("receipt: too many topics");
            }
            let mut topics = Vec::with_capacity(topics_len);
            for _ in 0..topics_len {
                if offset + 32 > data.len() {
                    ic_cdk::trap("receipt: topic truncated");
                }
                let mut topic = [0u8; 32];
                topic.copy_from_slice(&data[offset..offset + 32]);
                offset += 32;
                topics.push(topic);
            }
            let mut data_len = [0u8; 4];
            data_len.copy_from_slice(&data[offset..offset + 4]);
            offset += 4;
            let data_len = u32::from_be_bytes(data_len) as usize;
            if data_len > MAX_LOG_DATA {
                ic_cdk::trap("receipt: log data too large");
            }
            let data_end = offset + data_len;
            if data_end > data.len() {
                ic_cdk::trap("receipt: log data truncated");
            }
            let data = data[offset..data_end].to_vec();
            offset = data_end;
            logs.push(LogEntry {
                address,
                topics,
                data,
            });
        }
        Self {
            tx_id: TxId(tx_id),
            block_number: u64::from_be_bytes(bn),
            tx_index: u32::from_be_bytes(ti),
            status,
            gas_used: u64::from_be_bytes(gas),
            effective_gas_price: u64::from_be_bytes(gas_price),
            return_data_hash: ret,
            return_data,
            contract_address,
            logs,
        }
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: RECEIPT_MAX_SIZE_U32,
        is_fixed_size: false,
    };
}
