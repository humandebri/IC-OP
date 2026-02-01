//! どこで: export APIの実体 / 何を: cursor→chunks生成 / なぜ: lib.rsを薄く保つため

use evm_db::chain_data::{BlockData, ReceiptLike, TxId, TxIndexEntry};
use evm_db::stable_state::with_state;
use ic_stable_structures::Storable;
use std::borrow::Cow;

const MAX_EXPORT_BYTES: u32 = 1_500_000;
const MAX_SEGMENT_LEN: u32 = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportCursor {
    pub block_number: u64,
    pub segment: u8,
    pub byte_offset: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportChunk {
    pub segment: u8,
    pub start: u32,
    pub bytes: Vec<u8>,
    pub payload_len: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportResponse {
    pub chunks: Vec<ExportChunk>,
    pub next_cursor: Option<ExportCursor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExportError {
    InvalidCursor(&'static str),
    Pruned { pruned_before_block: u64 },
    MissingData(&'static str),
    Limit,
}

pub fn export_blocks(
    cursor: Option<ExportCursor>,
    max_bytes: u32,
) -> Result<ExportResponse, ExportError> {
    if max_bytes == 0 {
        return Err(ExportError::Limit);
    }
    let max_bytes = if max_bytes > MAX_EXPORT_BYTES {
        MAX_EXPORT_BYTES
    } else {
        max_bytes
    };

    with_state(|state| {
        let head = state.head.get().number;
        let pruned_before = state.prune_state.get().pruned_before();
        let start_block = match pruned_before {
            Some(value) => value.saturating_add(1),
            None => 0,
        };
        let cursor = cursor.unwrap_or(ExportCursor {
            block_number: start_block,
            segment: 0,
            byte_offset: 0,
        });
        if let Some(pruned) = pruned_before {
            if cursor.block_number <= pruned {
                return Err(ExportError::Pruned {
                    pruned_before_block: pruned,
                });
            }
        }
        if cursor.segment > 2 {
            return Err(ExportError::InvalidCursor("segment out of range"));
        }
        if cursor.block_number > head {
            return Ok(ExportResponse {
                chunks: Vec::new(),
                next_cursor: Some(cursor),
            });
        }

        let block_ptr = state
            .blocks
            .get(&cursor.block_number)
            .ok_or(ExportError::MissingData("block missing"))?;
        let block_bytes = state
            .blob_store
            .read(&block_ptr)
            .map_err(|_| ExportError::MissingData("block bytes missing"))?;
        let block = BlockData::from_bytes(Cow::Owned(block_bytes.clone()));

        let receipts_payload = build_receipts_payload(state, &block.tx_ids)?;
        let tx_index_payload = build_tx_index_payload(state, &block.tx_ids)?;

        let block_payload = block_bytes;
        let payloads = [block_payload, receipts_payload, tx_index_payload];
        let payload_lens = [
            u32::try_from(payloads[0].len()).map_err(|_| ExportError::InvalidCursor("block too large"))?,
            u32::try_from(payloads[1].len()).map_err(|_| ExportError::InvalidCursor("receipts too large"))?,
            u32::try_from(payloads[2].len()).map_err(|_| ExportError::InvalidCursor("tx_index too large"))?,
        ];
        for len in payload_lens.iter() {
            if *len > MAX_SEGMENT_LEN {
                return Err(ExportError::InvalidCursor("segment too large"));
            }
        }
        let seg_index = usize::try_from(cursor.segment)
            .map_err(|_| ExportError::InvalidCursor("segment"))?;
        if cursor.byte_offset > payload_lens[seg_index] {
            return Err(ExportError::InvalidCursor("byte_offset out of range"));
        }

        let mut chunks = Vec::new();
        let mut remaining = max_bytes;
        let mut seg = cursor.segment;
        let mut offset = cursor.byte_offset;

        let mut is_first = true;
        while remaining > 0 && seg <= 2 {
            let seg_index = usize::try_from(seg).map_err(|_| ExportError::InvalidCursor("segment"))?;
            let payload = &payloads[seg_index];
            let payload_len = payload_lens[seg_index];
            if offset > payload_len {
                return Err(ExportError::InvalidCursor("offset out of range"));
            }
            if offset == payload_len {
                if is_first {
                    chunks.push(ExportChunk {
                        segment: seg,
                        start: offset,
                        bytes: Vec::new(),
                        payload_len,
                    });
                    is_first = false;
                }
                if seg == 2 {
                    break;
                }
                seg = seg.saturating_add(1);
                offset = 0;
                continue;
            }
            let available = payload_len.saturating_sub(offset);
            let take = if available > remaining { remaining } else { available };
            let start = usize::try_from(offset).map_err(|_| ExportError::InvalidCursor("offset"))?;
            let end = start
                .checked_add(usize::try_from(take).map_err(|_| ExportError::InvalidCursor("length"))?)
                .ok_or(ExportError::InvalidCursor("slice overflow"))?;
            let bytes = payload[start..end].to_vec();
            chunks.push(ExportChunk {
                segment: seg,
                start: offset,
                bytes,
                payload_len,
            });
            is_first = false;
            remaining = remaining.saturating_sub(take);
            offset = offset.saturating_add(take);
            if offset == payload_len {
                if seg == 2 {
                    break;
                }
                seg = seg.saturating_add(1);
                offset = 0;
            }
        }

        let next_cursor = ExportCursor {
            block_number: cursor.block_number,
            segment: seg,
            byte_offset: offset,
        };

        Ok(ExportResponse {
            chunks,
            next_cursor: Some(next_cursor),
        })
    })
}

fn build_receipts_payload(
    state: &evm_db::stable_state::StableState,
    tx_ids: &[TxId],
) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    for tx_id in tx_ids.iter() {
        let ptr = state
            .receipts
            .get(tx_id)
            .ok_or(ExportError::MissingData("receipt missing"))?;
        let bytes = state
            .blob_store
            .read(&ptr)
            .map_err(|_| ExportError::MissingData("receipt bytes missing"))?;
        let receipt = ReceiptLike::from_bytes(Cow::Owned(bytes));
        let encoded = receipt.to_bytes().into_owned();
        let len = u32::try_from(encoded.len())
            .map_err(|_| ExportError::InvalidCursor("receipt too large"))?;
        out.extend_from_slice(&tx_id.0);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&encoded);
    }
    Ok(out)
}

fn build_tx_index_payload(
    state: &evm_db::stable_state::StableState,
    tx_ids: &[TxId],
) -> Result<Vec<u8>, ExportError> {
    let mut out = Vec::new();
    for tx_id in tx_ids.iter() {
        let ptr = state
            .tx_index
            .get(tx_id)
            .ok_or(ExportError::MissingData("tx_index missing"))?;
        let bytes = state
            .blob_store
            .read(&ptr)
            .map_err(|_| ExportError::MissingData("tx_index bytes missing"))?;
        let entry = TxIndexEntry::from_bytes(Cow::Owned(bytes));
        let encoded = entry.to_bytes().into_owned();
        let len = u32::try_from(encoded.len())
            .map_err(|_| ExportError::InvalidCursor("tx_index too large"))?;
        out.extend_from_slice(&tx_id.0);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&encoded);
    }
    Ok(out)
}
