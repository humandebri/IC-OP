Phase0 Spec + 実装計画（ic-stable-structures最大活用版）
Phase0の目的（再定義）

Stable Memoryレイアウトを確定し、起動時に検証して壊れた起動を防ぐ

キー空間・エンコード・commit順序（決定性の核）を凍結

Phase1以降が乗る StableDB + OverlayDB の土台を実装（EVM実行はまだ）

つまり「書き換え不能な土台（Freeze）をコードとして固定」するのがPhase0。

0.1 Memoryレイアウト（MemoryManager）— 凍結対象
方針

MemoryId(0) は UPGRADES予約（Juno方式）

以降は用途別に VirtualMemory を割当

追加は末尾のみ（既存IDの意味変更禁止）

推奨レイアウト（Phase0で確定するID）
MemoryId	名称	用途	構造
0	UPGRADES	heap退避/移行用	Writer/Reader
1	META	magic/version/schema	Cell<Meta>
2	ACCOUNTS	Account基本情報	StableBTreeMap<AccountKey, AccountVal>
3	STORAGE	EVM storage	StableBTreeMap<StorageKey, U256Val>
4	CODES	bytecode	StableBTreeMap<CodeKey, CodeVal>
5	STATE_AUX	将来の拡張用（予約）	予約
6..	以降追加	tx/blocks/receipt等（Phase1/2で追加）	末尾追加

Phase0では 0〜4 を確定しておけば十分。Phase1以降のMap類は IDだけ予約するか、末尾追加でもOK（ただし予約しておくと将来のマイグレーション事故が減る）。

実装タスク

memory/mod.rs

AppMemoryId enum（repr(u8)）

MemoryManager::init(DefaultMemoryImpl::default())

get_memory(id) -> VirtualMemory<DefaultMemoryImpl>

起動時の init_memory() を thread_local で提供

0.2 Meta（Cell）— 起動検証・バージョン管理（凍結対象）
Metaに入れるべき最小セット

magic と version だけだと「別のスキーマでも version=1 で起動」みたいな事故が起きるので、schema_hash を足すのが堅いです。

struct Meta {
  magic: [u8; 4],        // 例: b"EVM0"
  layout_version: u32,   // MemoryId/Keyspace/Encoding の世代
  schema_hash: [u8; 32], // 凍結仕様から算出した固定値
}


layout_version は Phase0の凍結仕様が変わったら上げる

schema_hash は “凍結仕様文字列” を Keccak-256 した固定値でもいい（spec.mdでは Keccak-256 を採用）
（例："mem:0..4|keys:v1|ic_tx:rlp-fixed|merkle:v1|env:v1" をhash）

起動時の挙動（凍結）

未初期化なら Cell::init で書き込み

既存なら読み込み、magic/schema_hash/layout_version が一致しなければ trap

マイグレーションは Phase0ではやらない（trapする）。やるならPhaseXで明示的に追加

実装タスク

meta.rs

impl Storable for Meta（固定長、BOUND fixed）

init_meta_or_trap()（trapメッセージは具体的に）

0.3 キー空間・Valueエンコード（凍結対象）
方針（あなたのまとめ通りでOK）

String や可変長keyは避ける

固定長NewTypeで BOUND.is_fixed_size = true を最大化

prefix＋big-endianで辞書順が意味を持つようにする（range/prefix scanが安定）

キー定義（凍結）

AccountKey: 0x01 || addr20（21 bytes）

StorageKey: 0x02 || addr20 || slot32（53 bytes）

CodeKey: 0x03 || code_hash32（33 bytes）

値定義（凍結）

AccountVal（固定長推奨）
nonce_u64_be8 || balance_u256_be32 || code_hash32（72 bytes）
※ 将来フィールド追加したいなら “別Map” に逃がすのが安全

U256Val: 32 bytes固定

CodeVal: 可変（ただし上限Bounded）
Bound::Bounded{ max_size: MAX_CODE_SIZE, is_fixed_size:false }
※ Phase0実装では MAX_CODE_SIZE = 24KB を採用（凍結）

実装タスク

types/keys.rs

AccountKey([u8;21]) / StorageKey([u8;53]) / CodeKey([u8;33])

make_* 関数は純粋関数（panicしない）

types/values.rs

U256Val([u8;32]) など

types/storable.rs

impl Storable 群（from_bytesで長さ検証してtrap or unwrap）

ここがPhase0の“凍結の本丸”。後で変えるとstate_rootも互換も死ぬ。

0.4 StableDB 初期化（StableBTreeMap）— 土台実装
実装タスク

stable_state.rs

type Accounts = StableBTreeMap<AccountKey, AccountVal, VMem>;

type Storage = StableBTreeMap<StorageKey, U256Val, VMem>;

type Codes = StableBTreeMap<CodeKey, CodeVal, VMem>;

StableState { accounts, storage, codes }

init_stable_state()（MemoryIdを結線する場所：Junoのinternal.rs相当）

0.5 OverlayDB + Tombstone + 決定的Commit（凍結対象：順序規則）

あなたのまとめ通り、Overlayは決定性と原子性の要。Phase0で “commit順序＝キー昇順” を仕様として固定します。

Overlayの仕様（凍結）

Overlayは writes: BTreeMap<K, Option<V>>

Some(v) = set/update

None = delete（tombstone）

commit() は writes.iter() をそのまま昇順適用（BTreeMap保証）

touchedの扱いは “root計算の最適化用” だが、Phase0では APIだけ置く（後で使えるように）

Tombstoneの実装上の注意（ここだけはPhase0で明文化しておく）

「存在しない」と「ゼロ値」は区別したほうが良い

EVM storageのゼロは “未保存” に圧縮するのが普通

つまり set(slot, 0) は delete(slot) に正規化するポリシーを 後で入れたくなる

そのためOverlay層に normalize_storage_value() のフックを用意し、

Phase0では no-op

Phase1で “0ならdeleteに変換” を入れられるようにしておく

実装タスク

overlay.rs

OverlayMap<K,V> 汎用（get/set/delete/commit）

commit_to(base_map: &mut StableBTreeMap<...>)

低コストにするなら Cow::Borrowed を多用（Key/Valが固定長なら超効く）

0.6 Upgrade耐性（UPGRADES領域）— “必要最小限だけ”実装

Phase0では巨大データは StableBTreeMap に入るので、UPGRADES領域は基本「軽量設定」専用でいい。

方針

heapの軽量configだけ MemoryId(0) に退避

StableBTreeMap群は upgradeでそのまま残る（コピー不要）

post_upgrade で再初期化 → init_meta_or_trap() → StableState再結線

実装タスク

upgrade.rs

write_pre_upgrade(bytes) / read_post_upgrade()

pre_upgrade / post_upgrade hook

退避データのフォーマットはCBORでもいいが、ここも version を入れる

Phase0実装の補足（現状）
- schema_hash は SHA-256 を採用（spec.md に固定値あり）
- AccountVal は u64_be nonce + u256_be balance + code_hash32（72 bytes 固定）
- UPGRADES 領域は version(u32_le) のみ書き込み・post_upgradeで一致検証

0.7 Phase0のテスト計画（ここまでが“土台の品質”）

Phase0で落とすべきテストは全部 “決定性/互換性/辞書順” 系です。

必須テスト

Meta検証

初回initで書かれる

2回目起動で一致する

magic違いでtrap（ユニットテストはtrap検知）

Keyの辞書順

prefix scan / range が期待通り

big-endian固定で順序が崩れない

Overlay commit順序

同一writes集合を異なる挿入順で積んでも、commit結果が一致

Storable roundtrip

Key/Valの to_bytes/from_bytes が同値

（Phase0ではMerklizeまでは不要。root計算はPhase1以降でOK）

Phase0の成果物（ファイル構成の提案）

src/memory.rs（MemoryManager + AppMemoryId）

src/meta.rs（Cell + init_meta_or_trap）

src/types/{keys.rs, values.rs, storable.rs}

src/stable_state.rs（StableBTreeMap結線）

src/overlay.rs（Overlay + tombstone）

src/upgrade.rs（UPGRADES領域のread/write）

src/lib.rs（init入口：post_upgrade / init で meta→state 初期化）

tests/phase0_*.rs
