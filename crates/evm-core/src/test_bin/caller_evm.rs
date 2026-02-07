//! どこで: 開発用CLI / 何を: principal文字列 -> caller_evm / なぜ: canister外で導出するため

use alloy_primitives::keccak256;
use candid::Principal;

const DOMAIN_SEP: &[u8] = b"ic-evm:caller_evm:v1";

fn main() {
    let principal = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: caller_evm <principal_text>");
        std::process::exit(1);
    });
    let bytes = match decode_principal_text(&principal) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("invalid principal: {err}");
            std::process::exit(1);
        }
    };
    let mut payload = Vec::with_capacity(DOMAIN_SEP.len() + bytes.len());
    payload.extend_from_slice(DOMAIN_SEP);
    payload.extend_from_slice(&bytes);
    let out = keccak256(&payload).0;
    let addr = &out[12..32];
    println!("{}", hex::encode(addr));
}

fn decode_principal_text(text: &str) -> Result<Vec<u8>, String> {
    Principal::from_text(text)
        .map(|principal| principal.as_slice().to_vec())
        .map_err(|err| err.to_string())
}
