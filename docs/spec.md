Phase0 Freeze Spec（凍結仕様）

目的
- Stable Memory のレイアウト/キー空間/エンコード/commit順序を凍結する
- 起動時検証で壊れた状態を検出し、誤起動を防ぐ

1. Memoryレイアウト（凍結）
方針
- MemoryId(0) は UPGRADES 予約
- 以降は用途別の VirtualMemory を割当
- 追加は末尾のみ（既存IDの意味変更禁止）

確定レイアウト（Phase0）
MemoryId | 名称 | 用途 | 構造
0 | UPGRADES | heap退避/移行用 | Writer/Reader
1 | META | magic/version/schema | StableCell<Meta>
2 | ACCOUNTS | Account基本情報 | StableBTreeMap<AccountKey, AccountVal>
3 | STORAGE | EVM storage | StableBTreeMap<StorageKey, U256Val>
4 | CODES | bytecode | StableBTreeMap<CodeKey, CodeVal>
5 | STATE_AUX | 将来拡張（予約） | 予約
6.. | 追加 | tx/blocks/receipt等 | 末尾追加

2. Meta（凍結）
構造
- magic: [u8; 4] = b"EVM0"
- layout_version: u32
- schema_hash: [u8; 32]

schema_hash の定義
- schema_string = "mem:0..4|keys:v1|ic_tx:rlp-fixed|merkle:v1|env:v1"
- schema_hash = Keccak-256(schema_string)

起動時挙動
- 未初期化: StableCell::init で書き込み
- 既存: magic/layout_version/schema_hash が一致しなければ trap
- Phase0ではマイグレーションは行わない（trap）

3. Key空間（凍結）
キーは固定長・prefix + big-endian を採用
- AccountKey: 0x01 || addr20（21 bytes）
- StorageKey: 0x02 || addr20 || slot32（53 bytes）
- CodeKey: 0x03 || code_hash32（33 bytes）

4. Valueエンコード（凍結）
- AccountVal: nonce_u64_be8 || balance_u256_be32 || code_hash32（72 bytes）
- U256Val: 32 bytes 固定
- CodeVal: 可変長（Bounded: max_size = MAX_CODE_SIZE, is_fixed_size=false）

5. OverlayDB（凍結）
- writes: BTreeMap<K, Option<V>>
- Some(v)=set/update, None=delete
- commit順序: BTreeMapの昇順イテレーション順

6. Upgrade領域（凍結）
- MemoryId(0) は軽量設定の退避専用
- StableBTreeMap群は upgrade で残る（コピー不要）
- post_upgrade は meta検証 → StableState再結線

7. テスト要求（Phase0）
- Meta検証: 初回init / 再起動一致 / 不一致trap
- Key辞書順: prefix順 + big-endian順
- Overlay順序: insert順に依らず昇順commit
- Storable roundtrip: to_bytes/from_bytes 一致
