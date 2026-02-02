# Indexer Runbook v2.1（TS + SQLite + local zstd archive）

## 0. 目的と前提（壊れないための不変条件）

- Indexer（SQLite/Archive）は **キャッシュ**。チェーン本体は canister 側。
- 取り込みの整合は「**アーカイブ成功 → DBコミット → cursor更新（同一TX）**」で守る。
- canister 側 prune は **外部ACKに依存しない**（依存させると外部障害でチェーン死亡する）。

## 1. コンポーネント

- canister: `export_blocks(cursor, max_bytes)` を提供
- indexer:
  - export を poll
  - Chunk を復元して payload decode
  - SQLite に upsert
  - raw(3seg) を zstd 圧縮して `*.bundle.zst` として保存（任意キャッシュ）
  - 起動時に archive GC（tmp削除 + orphan削除）

## 2. 起動手順

### 2.1 依存
- Node.js（`better-sqlite3` のABIが合うバージョン）
- npm install

ABIズレで落ちる場合:
- `npm rebuild better-sqlite3`

### 2.2 設定（環境変数）
主要:
- `INDEXER_DB_PATH`（SQLiteファイル）
- `INDEXER_ARCHIVE_DIR`（アーカイブ保存先）
- `INDEXER_MAX_BYTES`（export の max_bytes。推奨 1〜1.5MiB）
- `INDEXER_IDLE_POLL_MS`（追いつき時の固定ポーリング間隔。既定 1000ms）
- `INDEXER_BACKOFF_MAX_MS`（失敗時の最大バックオフ。既定 5000ms）
- `INDEXER_FETCH_ROOT_KEY`（local向け）

### 2.3 起動
- `node dist/run.js`（実際の起動コマンドはプロジェクトの package.json に合わせる）

起動直後にやること:
- archive GC が走る（失敗しても warning のみ）

## 3. 停止手順

- SIGINT / SIGTERM で停止（stop_requested を立ててループを抜ける）
- 途中停止しても、cursor は DBコミット単位でしか進まないので再開は安全

## 4. ログの見方（JSON lines）

主な event:
- `retry`: ネットワーク/呼び出し失敗（backoffあり）
- `idle`: chunks=[]（追いつき状態、60秒に1回程度）
- `fatal`: 取り込み継続不可（exit(1)）

fatal の代表:
- `Pruned`: 取り込もうとした範囲が canister 側で prune 済み
- `InvalidCursor`: cursor/chunk整合違反 or max_bytes超過 or カーソル不正
- `Decode`: payload decode 失敗
- `ArchiveIO`: アーカイブ書き込み失敗
- `Db`: SQLite 失敗

## 5. SQLite マイグレーション

- 起動時に `schema_migrations` を見て未適用SQLを適用する
- 適用は `BEGIN IMMEDIATE` で全体を1トランザクション化
- すでに適用済みの migration はスキップされる（idempotent）

運用ルール:
- migration SQL を増やしたら `MIGRATIONS` 配列に追加する

## 6. アーカイブ（zstd）

### 6.1 保存形式
- 1ブロック1ファイル: `<archiveDir>/<chainId>/<blockNumber>.bundle.zst`
- raw は 3seg を `u32be(len)+payload` で連結してから zstd 圧縮

### 6.2 atomicity
- `*.tmp` に書いて `rename`（同一FS内で原子的）
- `.tmp` が残っても起動時GCで削除される

### 6.3 起動時GC
- `.tmp` は常に削除
- orphan（DBに紐づかない `*.bundle.zst`）は **DBに参照が1件以上ある場合のみ削除**
  - DBが空の状態で「全削除」しないための安全弁

## 7. 日次メトリクス（metrics_daily）

最低限の観測:
- `blocks_ingested`（コミット1回につき +1）
- `raw_bytes`（取り込んだ raw ）
- `compressed_bytes`（zstd後）
- `sqlite_bytes`（現状は「SQLiteファイルサイズ（bytes）」を日次で保存。差分は集計側で計算）
- `archive_bytes`（現状は「アーカイブディレクトリ総サイズ（bytes）」を日次で保存）

注:
- サイズ計測は「その日の最初のコミット時」に更新（best-effort）

## 8. 典型障害と復旧

### 8.1 Pruned で停止した
意味:
- indexer が追いつく前に canister が古いブロックを prune した
- その範囲は canister からはもう取れない

対応:
1) まず canister 側の `pruned_before_block` を確認
2) indexer の cursor を `pruned_before_block + 1` 以降に進めて再開
3) 過去分が必要なら「アーカイブが残っている範囲」から再構築（アーカイブが無いなら復旧不能）

再発防止:
- pruning を有効化する前に indexer を常時稼働させ、lag を監視する
- hard_emergency が発動する前に通常水位で prune できるようにする

### 8.2 InvalidCursor / Decode
- 仕様違反か実装バグの可能性が高い
- `fatal` ログに `cursor / next_cursor / chunks_summary` が出るので、その組み合わせで再現テストを作る

### 8.3 ArchiveIO
- ディスク枯渇・権限・別FSへのrenameなど
- まず保存先の空き容量/パーミッション確認

## 9. pruning の段階的ON（canister側）

推奨フロー（事故りにくい順）:
1) **policy だけ投入**（enabled=false のまま）
2) `get_prune_status` で水位・oldest・推定容量を確認
3) enabled=true にして timer を動かす（最初は小さく）

初期推奨パラメータ（例）:
- `timer_interval_ms`: 30_000〜60_000
- `max_ops_per_tick`: 200〜500（最初は小さく）
- `headroom_ratio_bps`: 2000（20%）
- `hard_emergency_ratio_bps`: 9500（95%）
- `retain_days`: 14（監査重視なら 30）
- `target_bytes`: 実測（bytes/day）× retain_days × (1+headroom) で決める

次（実装の続きとしてやるべき順）

ドキュメントじゃなくて実装の話に戻すと、もう **「実測→policy決定→段階的ON」**のフェーズだから、次の3つだけやればいい。

canister 側の get_prune_status を indexer 側に定期pullして meta に書く（head/pruned_before/estimated_kept_bytes/stable_pages）

cursor_lag（head - cursor）をメトリクス化（日次じゃなくてもいい、ログでもいい）

pruning enable の手順をスクリプト化（set_policy → enabled=true をワンコマンド化）
### 7.1 prune_status 監視

* `get_prune_status()` を定期ポーリングして `meta.prune_status` に JSON 保存
* JSON は `estimated_kept_bytes` / `high_water_bytes` / `hard_emergency_bytes` を文字列で保持して追跡
* 監視側は `need_prune` フラグと `cursor_lag` を合わせてアラート
