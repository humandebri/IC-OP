Phase3 Spec + 実装計画（L1 Anchor + Trusted Bridge）
目的

L1に ブロック出力（state_root等）を投稿して監視・監査の基準点を作る

L1↔L2（あなたのEVM canister）間で 資産移動（まずERC20） を実現する

セキュリティモデルは Trusted（DAO/マルチシグ/Relayer） を明示し、ガードレールで現実運用できる形にする

非目的

異議申し立て（Fault Proof）/ ZK証明（= L1担保の本体）はやらない

完全なOP Stack互換は狙わない

全体アーキテクチャ
コンポーネント

EVM canister（既存）：Phase1/2で出来たチェーン本体（順序確定・実行・state_root）

Relayer（外部プロセス推奨）：L1イベント監視＆L2への適用、L2出金要求のL1反映

※「全部canisterで」も将来的に可能（threshold ECDSA + HTTPS outcalls）だが、Phase3は外部relayerで十分

L1 Contracts

OutputOracle（アンカー）

L1BridgeVault（入金ロック・出金解放）

（任意）Timelock / AccessControl（権限・遅延・停止）

L2 Contracts（あなたのEVM上）

L2Bridge（mint/burn or escrow）

WrappedERC20（L1 tokenごとのラップ）

1) L1 Anchor（OutputOracle）
1.1 L1コントラクト仕様（最小）

postOutput(uint256 l2BlockNumber, bytes32 l2BlockHash, bytes32 stateRoot, bytes32 txListHash)

event：OutputPosted(l2BlockNumber, l2BlockHash, stateRoot, txListHash)

権限：PROPOSER_ROLE（DAO/マルチシグが管理）

ガード：

l2BlockNumber は単調増加（またはスキップ可だが戻れない）

l2BlockHash は前回の l2BlockHash とつながることを Relayer側で検査（L1側で強制は必須ではない）

1.2 投稿ルール（運用仕様）

投稿頻度：every N blocks（例: 10〜100）または手動

冪等性：同じ (l2BlockNumber, l2BlockHash) は再投稿拒否（or no-op）

ここは「担保」じゃなく「掲示板」。でも監査とブリッジ監視の足場になる。

2) Trusted Bridge（ERC20から始める）
2.1 用語

Deposit（L1→L2）：L1でロック → L2でmint（Wrapped）

Withdraw（L2→L1）：L2でburn → L1で解放

2.2 L1BridgeVault（仕様）

deposit(address l1Token, address toL2, uint256 amount)

transferFrom でVaultにロック

event：DepositInitiated(l1Token, msg.sender, toL2, amount, depositId)

finalizeWithdrawal(address l1Token, address toL1, uint256 amount, bytes32 withdrawId)

RELAYER_ROLE（trusted）

withdrawId の二重実行防止（mapping）

event：WithdrawalFinalized(...)

depositId / withdrawId の定義（凍結）

depositId = keccak256(l1TxHash || logIndex)（L1イベント単位で一意）

withdrawId = keccak256(l2TxId || withdrawalIndex)（L2側で一意）

2.3 L2側（あなたのEVM上）
Option A（EVM互換寄り）：イベントで出金要求を出す

L2Bridge.withdraw(l1Token, toL1, amount) が WithdrawalInitiated(...) を emit

RelayerがL2 RPCでログを拾い、L1 finalizeWithdrawal を叩く

この場合、Phase3で “logsをreceiptに保存” が必要（後述）。

Option B（実装簡単寄り）：canisterがOutboxを持つ（ログ不要）

L2Bridge.withdraw が 決まったストレージスロットに要求を書き、Relayerは eth_call で読み取る（or canister専用query）

もしくは L2Bridgeは単なる目印で、出金要求は canister の withdraw_queue に直接書く（ただしEVM互換性は下がる）

Phase3のおすすめ：Option A（イベント）＋「ログの保存だけ」
フィルタ/インデックスは不要。Relayerが“ブロック範囲走査”すればいい。

3) Phase3で追加するL2側機能（EVM canister内部）
3.1 Receiptにlogsを保存（最低限）

Phase1ではreceiptが最小だったはずなので、Phase3で：

ReceiptLike.logs: Vec<LogEntry>

address(20), topics(Vec<32>), data(bytes)

gasUsed/status は既存

インデックスは作らない（沼回避）。代わりにRelayer向けに簡易APIを用意：

query get_logs(from_block, to_block, address?, topics?) -> Vec<LogEntryWithContext>

実装は単純：

blocksを走査 → そのブロックのtx_ids → receipts → logs をフィルタ

これでブリッジ用途には十分。

3.2 Bridge Inbox/Outbox（stable）— 冪等性の核

processed_deposits: Set<depositId>（stable）

processed_withdrawals: Set<withdrawId>（stable）

（任意）bridge_events: Log（監査用、後述）

3.3 L1→L2 deposit 適用API（update）

Relayerが canister を叩く入口（trusted）。

update apply_deposit(deposit: DepositMessage) -> Result

DepositMessage{ depositId, l1Token, fromL1, toL2, amount }

depositId 未処理なら、L2側で mint（or escrow）を実行

processed_deposits.insert(depositId)（冪等）

mintの実装方法

L2に WrappedERC20 をデプロイしておき、canisterは system権限のfrom（例: 0x000…bridge）として execute_ic_tx で mint(to, amount) を呼ぶ
→ EVMの通常トランザクションとして履歴に残る（監査が楽）

3.4 L2→L1 withdraw の検出

Relayerは get_logs で WithdrawalInitiated を拾う

withdrawId を計算してL1 finalize

L1 finalize 成功後、Relayerが（任意で）L2に mark_withdrawal_finalized(withdrawId) を書く（監査用）

4) セキュリティモデル（Phase3の“正直な強さ”）
4.1 Trustedの定義

L1BridgeVault の RELAYER_ROLE を持つ主体（DAO/マルチシグ管理）が、出金を最終的に許可できる

つまり L1担保ではなく、DAO運用担保

4.2 ガードレール（Phase3で必須）

L1コントラクトに入れる：

pause()（deposit/withdraw止める）

withdrawal_daily_limit（上限）

token_allowlist（対応トークン制限）

timelock（role変更や上限変更を遅延）

emergency_withdraw（最悪時の救済：運用手順が必要）

L2側（canister）にも：

apply_deposit のレート制限（per token / per relayer）

processed_* による冪等性

5) 実装タスク分解（Phase3チケット）
5.1 L1コントラクト

 OutputOracle（postOutput + role + event）

 L1BridgeVault（deposit / finalizeWithdrawal + pause + limits + idempotency）

 Timelock/AccessControl（既製のを使う）

5.2 L2コントラクト（あなたのEVM上）

 WrappedERC20（mint/burn、bridgeのみmint可）

 L2Bridge（withdrawイベント emit、(l1Token,to,amount)）

5.3 EVM canister側（Phase1/2の上に追加）

 receiptsにlogs保存

 get_logs(from,to,filters) query

 stable: processed_deposits / processed_withdrawals

 apply_deposit(DepositMessage) update（system-fromでmint txを実行）

5.4 Relayer（外部プロセス）

 L1: DepositInitiated監視 → apply_deposit 呼び出し

 L2: WithdrawalInitiated監視（get_logs）→ L1 finalizeWithdrawal

 L1: OutputOracleへ定期post（block meta取得はPhase2 RPCで）

6) Phase3 合格テスト（統合）

最低限これが通れば “接続できた” と言える。

Anchor

L2がNブロック進む

RelayerがOutputOracleに投稿し、イベントが並ぶ

再投稿が冪等

Deposit

L1でdeposit → Relayer検知 → L2 mint

同じdepositIdを二度送ってもL2は二重mintしない

Withdraw

L2でwithdraw → Relayer検知 → L1解放

同じwithdrawIdを二度叩いてもL1が拒否

停止

pauseでdeposit/withdrawが止まる

7) Phase3で“やらない宣言”（重要）

出金の正当性をL1が検証する（＝proof/challenge）

L2 state の過去スナップショット証明

高性能ログインデックス（eth_getLogs互換）