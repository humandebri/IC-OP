Phase4（OP + FPVM）改訂版ロードマップ
Phase4の到達点（これが“L1担保”の定義）

L1に データ（tx列） があり、誰でも 同じ再実行ができる

出力（state_rootなど）の投稿に対し、第三者が challengeして差し止め可能

争いが起きたら、L1上の Dispute Game が FPVMの1-step検証で白黒つける

ブリッジ出金は finalized output にのみ紐づく（運営/relayerの裁量なし）

Phase4.0 追加で凍結する仕様（Phase0〜3の上に乗る“OP用Freeze”）
0.1 L2 OutputRoot（L1が扱う「出力」の定義）を凍結

L1で最終的に守りたいのは state_root だけじゃなく、ブリッジの出金根拠も含むべきなので、最低これを固定します。

output_root = keccak256( domain || l2_block_hash || state_root || tx_list_hash || outbox_root )

outbox_root は L2の「出金要求（withdrawal）」を Merkle 化した根（後述）。
※ domain は固定（衝突回避）。
※ Phase3の掲示板（OutputOracle）は state_root だけだったが、Phase4ではこれを output_root に上げる。

0.2 Batch（DAデータ）のエンコード凍結

L1に投稿する tx列の表現（再実行入力）を凍結。

batch = concat( u32_be(len) || tx_bytes )*

batch_hash = keccak256(batch)

tx_bytesは Phase0/1で凍結済み（Eth raw or IcSynthetic bytes）。

0.3 再実行環境（Fixed Env）を“検証用”として凍結

あなたの Phase1 でやってる timestamp = parent+1 等の決定性Envは、Phase4では「検証可能性」の要件になるので、ここからは破れない（破るとFPVMが一致しない）。

Phase4.1 DA（Data Availability）— L1に tx列を置く
4.1.1 L1: BatchInbox コントラクト

appendBatch(bytes batch, bytes32 batchHash, uint64 startBlock, uint64 endBlock)

BatchAppended(batchHash, startBlock, endBlock) event

権限：proposer（DAO管理）で開始してOK（Phase4の“trustless”はここではなく dispute 側で担保する）

4.1.2 L2（EVM canister）側の変更

ブロック保存時に batch_hash を BlockData に記録（後で output と紐づける）

get_batch_bytes(start,end) みたいな内部関数（relayer生成用。RPC経由でも良い）

4.1.3 OutputOracle の投稿内容を拡張

proposeOutput(l2BlockNumber, output_root, batch_hash) を最低限にする

4.1 合格条件

L1の batch だけで第三者が “再実行入力” を取れる
（＝ challenger が材料不足で負ける状況を潰す）

Phase4.2 Outbox（出金要求を“証明可能”にする）

ログを拾って出金、はL1で検証できないので終わり。ここは Outbox Merkle が必要です。

4.2.1 L2: Outbox（Merkle 木）

各ブロックで withdrawal_leaves を集めて outbox_root を確定

leaf例：

leaf = keccak256(domain || l1Token || toL1 || amount || l2TxId || index)

outbox_root はブロックメタに保存し、output_root に含める（Phase4.0で凍結）

4.2.2 L1: Bridge は “proof-based withdraw” に変える

proveWithdrawal(output, leaf, merkleProof) が通ったら finalizeWithdrawal

output は finalized 済みであることが条件

4.2 合格条件

L2→L1出金が「運営承認」じゃなく「証明」で通る形になる（ただしoutput finalizationはまだ次のOP）

Phase4.3 OP（Fault Proof）— FPVM（1-step）＋ Dispute Game

ここが指定の(A)。

4.3.1 FPVMの選定（“特定VM”）

この手の設計で現実的なのは **Cannon系（MIPS風）**か RISC-V。Solidityで1-step実装するコストと前例を考えると、計画としては：

FPVM = MIPS32相当の小さいISA（Cannonライク） を推奨
理由：1-stepの実装が比較的現実的、設計パターンが固い

（RISC-Vでも可能だが、実装量が増えがち）

4.3.2 “再実行プログラム”（Client Program）

やることは単純で、重い。

入力：

pre_state_root

batch（L1 BatchInboxから）

fixed_env_params（凍結）
出力：

post_state_root

outbox_root（同時に計算して良い）

これを FPVM上で動くバイナリとして固定（ツールチェーンも凍結する：Rustのバージョン・コンパイラ設定含む）。

4.3.3 Preimage Oracle（L1上）

FPVMは「メモリの中身」を全部L1に載せられないので、必要なデータは

ハッシュ（コミット）＋

必要になった時に preimage を提示
で取り出す。

L1に PreimageOracle を置いて、

loadPreimage(hash, bytes)（proposer/challengerが提示）

以後、DisputeGameが参照できる
という形にする（“必要になるまで出さない”でガスを抑える）。

4.3.4 Dispute Game（バイセクション）

proposerが output_root を提案（L1）

challengerが challenge(output_root) を開始（bondを積む）

両者は「実行トレースの中間状態（state）」を提出して 二分探索

最後に 1 step（1命令）まで絞ったら、L1の FPVM stepper が

“この1ステップは正しいか”
を判定して勝敗を決める

勝った側がbond回収、負けた側はスラッシュ。

4.3.5 L1コントラクト群（Phase4-OPの構成）

BatchInbox（Phase4.1）

OutputOracle（propose/finalize、finalize条件に dispute を繋ぐ）

DisputeGameFactory（ゲーム生成）

FaultDisputeGame（二分探索＋最終1-step判定）

PreimageOracle（preimage格納）

BridgeVault（withdrawは finalized output + inclusion proof）

4.3 合格条件

不正な output を出しても、第三者が challenge して finalizeを止められる

dispute は最終的に L1の1-stepで決着し、運営に依存しない

Phase4.4 ブリッジを “OP finality” に接続して trusted 経路を落とす

Phase3の RELAYER_ROLE finalizeWithdrawal を主系から外す。

正規ルート：proveWithdrawal(finalized_output, leaf, proof) -> finalize

trustedルート：緊急用に pause解除/資産救済としてだけ残す（timelock必須）

追加で入れるべき「作業順序（依存関係）」まとめ

Phase4はこの順が手戻りしない：

DA（BatchInbox）：再実行入力をL1に固定

Outbox：出金を証明可能にする（ログ依存を捨てる）

FPVM（再実行プログラム + 1-step）：裁判の最小単位を作る

Dispute Game：二分探索＋1-step判定

Bridgeをfinalized outputに接続：初めてL1担保の体裁が完成

Phase4で増える実装負荷（正直な見積もり感覚）

“再実行プログラム” 自体は Rust で作れるが、FPVM向けに落とすのが重い

Solidity側の 1-step実装 + メモリ証明が大工事

ただし「EVMの1-step」をやるよりは遥かにマシ（あなたが選んだ(A)は正しい）

Phase4の「凍結ポイント」（後で変えると死ぬ）

batchエンコード

output_root の定義（含めるフィールド）

outbox leafの定義

FPVM ISAと state hash 形式

再実行プログラムのツールチェーン固定