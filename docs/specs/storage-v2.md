# Storage v2 Spec (Stable Tx)

目的: stableに保存する構造とdecode責務を分離し、trap/互換事故/境界ズレを防ぐ。

## 方針
- stableは「bytes容器」に徹する
- decodeはcodec層のResultで扱う
- domain型にinvalidを混ぜない
- drop_code=decodeで統一

## 層分離
- infra(stable): StoredTxBytesのみStorable
- codec: TryFrom<StoredTxBytes> for StoredTx (Result)
- domain: StoredTxは常にvalid
- service(chain): load_txは必ずResultを返す

## 保存型 (stable)

StoredTxBytes:
- version: u8 (固定2)
- kind: TxKind
- raw: Vec<u8> (kind依存で解釈)
- fee: FeeFields (max_fee/max_priority/is_dynamic)
- caller_evm: Option<[u8;20]>
- canister_id: Vec<u8> (IcSyntheticのみ, principal bytes)
- caller_principal: Vec<u8> (IcSyntheticのみ, principal bytes)
- tx_id: TxId (下記生成ルール)

**重要**: stableのrawはkindに依存して解釈される。RawTxはdomain層でのみenum化。

## domain型

StoredTx:
- kind: TxKind
- raw: RawTx
- fee: FeeFields
- caller_evm: Option<[u8;20]>
- tx_id: TxId

RawTx:
- Eth2718(Vec<u8>)
- IcSynthetic(Vec<u8>)

FeeFields:
- max_fee_per_gas: u128
- max_priority_fee_per_gas: u128
- is_dynamic_fee: bool

## tx_id生成ルール

```
tx_id = keccak256(
  "ic-evm:storedtx:v2" || kind || raw || caller_evm?
)
```

- kindは固定1byteでエンコードする
  - 0x01 = EthSigned
  - 0x02 = IcSynthetic
- caller_evmを混ぜる場合は20 bytesそのまま
  - Noneの場合は混ぜない
- canister_id/caller_principalは長さprefix(u16be)+bytesで混ぜる

## codec仕様

- encode/decodeはResult
- version!=2 -> Err(UnsupportedVersion)
- 長さ不正 -> Err(InvalidLength)
- raw>MAX_TX_SIZE -> Err(DataTooLarge)

## Storable仕様

- Storable::from_bytesはtrapしない
- version不一致/破損はStoredTxBytes::invalid(version, raw)を返す
- invalidは**保存し直さない** (roundtrip禁止)

## rawのエンコード形式

- Vec<u8>は u32(be) length prefix + bytes
- 上限は MAX_TX_SIZE

## decodeタイミング

- rekey/ready判定: fee_fieldsのみ使用 (decode不要)
- execute直前のみdecode
- decode失敗 -> drop_code=decode

## ガード

- load_txは必ずResult
- produce_blockはinvalidをdrop_code=decode
- queue_snapshot/metricsはinvalidを除外

## テスト最低限

- StoredTxBytes roundtrip (version=2)
- unsupported versionがtrapしない
- invalid StoredTxBytesがdrop_code=decode
- rekeyはfee_fieldsのみで順序決定
- execute直前decode失敗でdrop_code=decode
