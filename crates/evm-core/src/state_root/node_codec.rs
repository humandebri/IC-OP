//! どこで: trieノード補助 / 何を: root変換共通化 / なぜ: 更新経路の重複を防ぐため

use crate::hash::keccak256;
use alloy_primitives::B256;
use alloy_trie::nodes::RlpNode;
use evm_db::chain_data::HashKey;

pub fn rlp_node_to_root(node: RlpNode) -> B256 {
    if let Some(hash) = node.as_hash() {
        hash
    } else {
        B256::from(keccak256(node.as_ref()))
    }
}

pub fn root_hash_key(root: [u8; 32]) -> Option<HashKey> {
    if root == [0u8; 32] {
        None
    } else {
        Some(HashKey(root))
    }
}
