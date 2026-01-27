Junoのアーキテクチャから、特に参考になる重要なファイルを4つ挙げます。

### 1. 構造体の初期化と紐づけ (Instantiation)
**ファイル:** `src/libs/satellite/src/memory/internal.rs`

ここで `manager.rs` で定義したメモリ領域を、実際の `StableBTreeMap` に渡しています。
「どのIDが、どのデータ構造になるか」の結合点です。

*   **見るべきポイント:** `init_stable_state` 関数。
*   **コード例:**
    ```rust
    // ソースコード- 周辺
    pub fn init_stable_state() -> StableState {
        StableState {
            db: StableBTreeMap::init(get_memory_db()), // ID: 1
            assets: StableBTreeMap::init(get_memory_assets()), // ID: 2
            // ...
        }
    }
    ```

### 2. データ型の定義 (Type Definitions)
**ファイル:** `src/libs/satellite/src/assets/storage/types.rs`

`StableBTreeMap` の Key と Value に何を使っているかが定義されています。ジェネリクス型を見ることで、データのスキーマがわかります。

*   **見るべきポイント:** `AssetsStable` などの型エイリアス。
*   **コード例:**
    ```rust
    // ソースコード 周辺
    pub type AssetsStable = StableBTreeMap<StableKey, Asset, Memory>;
    ```

### 3. シリアライズの実装 (Serialization)
**ファイル:** `src/libs/satellite/src/assets/storage/impls.rs`

`ic-stable-structures` を使う上で最も重要な `Storable` トレイトの実装部分です。
KeyやValueをどのようにバイト列に変換しているか（固定長か可変長か、Cborを使っているかなど）がわかります。あなたの設計にある「Valueエンコード凍結」の参考になります。

*   **見るべきポイント:** `impl Storable for ...` の実装。
*   **コード例:**
    ```rust
    // ソースコード 周辺
    impl Storable for StableKey {
        fn to_bytes(&self) -> Cow<'_, [u8]> {
            serialize_to_bytes(self)
        }
        // ...
    }
    ```

### 4. アップグレード時のヒープ退避ロジック
**ファイル:** `src/libs/shared/src/memory/upgrade.rs`

`MemoryId(0)` (UPGRADES領域) を使って、ヒープメモリ上のデータをアップグレード前後で退避・復元するロジックです。
あなたの設計にある「MemoryId 0 を予約する」という運用を具体的にどうコードに落とし込んでいるかがわかります。

*   **見るべきポイント:** `write_pre_upgrade` と `read_post_upgrade` 関数。
*   **コード例:**
    ```rust
    // ソースコード- 周辺
    pub fn write_pre_upgrade(state_bytes: &[u8], memory: &mut Memory) {
        let len = state_bytes.len() as u32;
        let mut writer = Writer::new(memory, 0);
        writer.write(&len.to_le_bytes()).unwrap();
        writer.write(state_bytes).unwrap()
    }
    ```

### まとめ：読むべき順序

1.  `manager.rs` (ID定義)
2.  `internal.rs` (BTreeの初期化)
3.  `types.rs` & `impls.rs` (データ型とシリアライズ)
4.  `upgrade.rs` (ヒープデータの退避)

これらをセットで読むことで、Junoの永続化層の全体像が掴めます。