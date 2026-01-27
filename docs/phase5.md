Phase5（OP後のプロダクト化フェーズ）
Phase5の目的

Phase4のOPセキュリティを壊さずに

スループット/コスト/運用を現実にする

EVM互換を上げてツールや他チェーンとちゃんと繋がる

開発者体験と “ICPから呼べる強み” をプロダクト化する

非目的

セキュリティモデルの変更（OP→ZK）はしない

仕様凍結（Phase0/4のfreeze）は原則維持（破るならハードフォーク相当）

Phase5.1 パフォーマンス（State commitment の差分化）

Phase1で全件走査rootをやってるなら、Phase5では必ず詰まる。

施策

touched set を本格利用して 差分Merkle（増分更新）

または StateCommitter差し替えで MPT互換に寄せる（ただしPhase4の再実行プログラムと整合が要る）

成果物

state_root 計算が「O(changes log N)」になる

ブロック生成が現実速度になる

Phase5.2 過去state参照（スナップショット/履歴）

OP運用やRPC互換で重要。

施策

ブロックごとに「state snapshot」全部は無理なので、現実は

チェックポイント（Nブロックごと）スナップショット

その間は差分ログで復元

eth_call を過去ブロックで実行できるようにする（ツールが喜ぶ）

Phase5.3 RPC互換拡張（開発者が本当に使える）

Phase2の“最低限ノード”から、次を段階追加。

追加候補（優先順）

eth_getLogs（indexあり/なし選択。最初はブロック走査でOK）

eth_feeHistory / eth_gasPrice（固定値から徐々に現実へ）

eth_subscribe（これは後回し。WebSocket地獄）

trace系（もっと後）

Phase5.4 ブリッジ拡張（資産と接続の現実化）

Phase4で「trustless exit」はできる。でも実際は入金導線・複数トークン・メッセージが必要。

施策

token allowlist + metadata registry

複数L1/L2対応（必要なら）

L1↔L2メッセージパッシング（汎用）
例：L1 contractからL2 contract呼び出し（逆も）を “proof付きメッセージ” として扱う

Phase5.5 分散運用（単一canisterの限界を超える）

あなたは今「単一canister＝Sequencer+Execution」。Phase4までなら成立する。Phase5で圧が来る。

選択肢（現実順）

複数canisterへの水平分割（ただし決定性維持）

execution canister

state canister（KV専用）

rpc gateway canister

もしくは、単一のまま“負荷がかかる周辺”を外に逃がす（ログindex、分析など）

Phase5.6 “ICPから呼べる価値”の本格商品化

ここは技術じゃなく、勝ち筋の固定。

施策

execute_ic_tx を中心に SDK/権限制御/課金/レート制限テンプレを完成

“ワークフロー→EVM確定” の設計をライブラリ化

サンプルdappを複数（会員証/決済/業務承認/自動化）

Phase5は必要か？

Phase4だけでも「担保付き出金」ができるので技術的には完成に見えるけど、

速度

RPC互換

運用

接続（橋）
が弱いと開発者が定着しない。なので 現実に使われるチェーンにするならPhase5はほぼ必須。