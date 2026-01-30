Phase2.3（DEXの作成）追記
対象範囲（スコープの明確化）

AMMは Constant Product (x*y=k) のみ

ルーティングは不要（単一プールで完結）

価格オラクル・TWAP・レンディング等は 非対象

フロントは最小（Swap / LP操作 / 残高表示）でOK
※エクスプローラー無しでも “使える感” を出す

トークン前提

ペアは (Native / Stable) または (ckBTC / Native) のように実需が出る組み合わせに限定

初期は 片方を基軸トークンに固定して運用簡略化（例：USDC相当 or ckBTC）

小数精度は ERC-20 decimals を尊重（内部計算は整数・桁固定で扱う）

料金設計（手数料）

Swap fee: 0.30%（Uniswap V2同等）をデフォルト

fee はプールに積み増し（= LPに帰属）

protocol fee（運営取り分）は初期は 0%（後でオンにできる設計だけ入れる）

重要な安全装置（最低限）

スリッページ保護：amountOutMin / amountInMax を必須にする

期限：deadline を必須にする（古い署名・古い見積もりを拒否）

0除算・不足残高・桁あふれ等の reject を明文化

Re-entrancy 相当の防止（外部呼び出しが絡むならガード必須）

仕様（必須の入出力）
Swap

swapExactIn(tokenIn, tokenOut, amountIn, amountOutMin, to, deadline)

swapExactOut(tokenIn, tokenOut, amountOut, amountInMax, to, deadline)（余裕があれば）

Liquidity

addLiquidity(tokenA, tokenB, amountADesired, amountBDesired, amountAMin, amountBMin, to, deadline)

removeLiquidity(tokenA, tokenB, liquidity, amountAMin, amountBMin, to, deadline)

LP token

ERC-20互換（name/symbol/decimals/balanceOf/transfer/approve/transferFrom）

初回mintは sqrt(amountA*amountB) - MINIMUM_LIQUIDITY を焼却（V2踏襲、プール保護）

状態管理・永続化（ICP/EVM前提の注意）

プール状態（reserves, k, blockTimestampLast 相当）は 決定的に保存

1 tx 内での順序：
transferIn → 計算 → reserve更新 → transferOut を固定

失敗時に中間状態が残らないように（atomicity）

観測（運用負荷を下げるための最低限）

取引イベント（ログ）を最小セットで出す

Swap(sender, amountIn, amountOut, to)

Mint(sender, amountA, amountB)

Burn(sender, amountA, amountB, to)

Sync(reserveA, reserveB)

管理用に “今の状態が一発で取れる” view API

getReserves()

quote()/getAmountOut()/getAmountIn()（計算だけ）

初期流動性の段取り（現実の足場）

初期LP（運営 or 指定アドレス）を決める

初期価格の決定方法を明記（固定 or 事前に決めた比率）

想定外価格になった時の方針（止める/放置/再投入）を決めておく

テスト（合格条件を支える最低限）

不変条件：swap 後に x*y >= k_before（fee分だけ増える）を検証

端数丸め：同じ入力で結果が安定する（決定性）

add/remove → LP比率が正しい

スリッページ・deadline でちゃんと落ちる

合格条件（追記）

連続swapしても reserves と価格が破綻しない

add/remove の後でも swap が正常に動き続ける

ログ or view API で 取引の追跡ができる

極端値（小額・大額・残高ギリギリ）での reject が正しい