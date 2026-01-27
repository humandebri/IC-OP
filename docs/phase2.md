Phase2 Spec + 実装計画（HTTP JSON-RPCノード機能）
Phase2の目的

Canisterが HTTPリクエストを受け、Ethereum風 JSON-RPC 2.0で応答する

viem / ethers / foundry 等のツールが「最低限」動く

読み取りは query、書き込みは update（HTTP upgrade）で分離し、決定性と安全性を壊さない

非目的

mempool / pending / eth_subscribe（WS）

eth_getLogs / filter（ログインデックス沼）

OP互換やL1投稿（Phase3以降）

1) HTTPインタフェース（IC仕様に合わせる）
1.1 エントリポイント

#[query] http_request(req: HttpRequest) -> HttpResponse

#[update] http_request_update(req: HttpRequest) -> HttpResponse

方針：

読み取りRPCは http_request 内で処理して返す

状態変更RPC（eth_sendRawTransaction など）は http_request が upgrade=true を返し、http_request_update で処理する

これで「HTTPで受けたけど中身はupdate」できる。IC的に正しい。

1.2 ルーティング

POST /：JSON-RPC（本命）

GET /healthz：軽いヘルス（query）

OPTIONS /：CORS（必要なら）

2) JSON-RPC 形式（互換の地雷を避ける）
2.1 リクエスト

単発：{jsonrpc:"2.0", id, method, params}

batch：[{...},{...}]（Phase2で対応推奨。意外と投げられる）

2.2 レスポンス

成功：{jsonrpc:"2.0", id, result}

失敗：{jsonrpc:"2.0", id, error:{code,message,data?}}

2.3 エラーコード（固定）

-32700 parse error

-32600 invalid request

-32601 method not found

-32602 invalid params

-32603 internal error

-32000〜：ノード固有（例：queue full、tx too large）

3) Hexエンコード規約（ここを間違えるとクライアントが死ぬ）

DATA（bytes）系：0x + even-length hex（空は0x）

QUANTITY（数値）系：0x0 か 0x + 先頭ゼロなし（ただし0だけ例外）

アドレス：0x + 40 hex（小文字で統一推奨、checksumは任意）

4) 実装するRPC（Phase2で“ノード”になる最小集合）
4.1 基本（query）

web3_clientVersion（固定文字列でOK）

net_version（chain_idを文字列化でもOK）

eth_chainId（Phase1の固定値）

eth_blockNumber（head）

eth_syncing：常に false

4.2 ブロック/Tx（query）

eth_getBlockByNumber(blockTag, fullTx)

blockTag は latest と 0x...number をサポート（pending は未対応にするか latest に落とす）

fullTx=false なら tx hash 配列

true なら tx object（最低限フィールドでOK）

eth_getTransactionByHash(txHash)

eth_getTransactionReceipt(txHash)

※ Phase1で tx_store / tx_index / receipts / blocks が揃ってる前提。

4.3 State（query）

eth_getBalance(address, blockTag)

eth_getCode(address, blockTag)

eth_getStorageAt(address, slot, blockTag)

ブロック指定は Phase2では：

latest のみ厳格対応（過去ブロック state は後回し）

0x... が来たら unsupported で返してもいい（ただしクライアントによっては困る）

4.4 実行系（query）

eth_call(callObject, blockTag)

Phase1の eth_call_like（overlay REVM）に直結

eth_estimateGas(callObject, blockTag)

最低限：一度overlayで実行し gas_used を返す

失敗時の扱いはクライアント依存があるので、revert の場合は error に data: revert_data を入れる（可能なら）

4.5 送信系（update via upgrade）

eth_sendRawTransaction(rawTx)
実装モードは2つ（Phase2で決め打ち）

モードA（推奨：UX最強）

http_request_update で execute_eth_raw_tx を呼び、即ブロック化

返すのは txHash（tx_id）

モードB（スループット寄り）

submit_eth_tx で enqueue だけして txHash を返す

別途 evm_produceBlock（独自RPC）で確定させる

「ノードとして動いた感」を重視するならA。POCはAでいい。

5) “ノードとして返すデータ”の最低限仕様（固定）

eth_getBlockByNumber で返すBlock objectは、最初はこれで十分：

number（QUANTITY）

hash（block_hash）

parentHash

timestamp（Phase1の決定的timestamp）

transactions（hashes or objects）

stateRoot（あなたのstate_root）

gasLimit / gasUsed（固定値でも可）

baseFeePerGas（固定値でも可：0）

Tx object（fullTx=true）は最小で：

hash

from

to

nonce

input

value

blockNumber

transactionIndex

Receipt object（logs空でも可）：

transactionHash

blockNumber

transactionIndex

status

gasUsed

contractAddress（create時のみ）

logs: []

6) DoS/制限（HTTP面で必須）

HTTPは入口が広いのでPhase2で必ず入れる。

max_http_body_size（例：256KB）

max_batch_len（例：20）

max_json_depth（深い入れ子拒否）

eth_call の max_gas（固定）

sendRawTx の max_tx_size（Phase1と同じ）

CORS：必要最低限（POCなら POST, OPTIONS と必要ヘッダだけ許可）

7) 実装タスク分解（そのままチケット化できる粒度）
7.1 HTTP骨格

 http_request / http_request_update 実装

 ルータ（/healthz、/）

 CORS/OPTIONS

7.2 JSON-RPCコア

 JSON parse（単発/batch）

 request validation（jsonrpc/id/method）

 error整形（コード固定）

 response serialize

7.3 RPCハンドラ（query）

 chainId / blockNumber / clientVersion / syncing

 getBlockByNumber（BlockData→JSON）

 getTxByHash（tx_store→JSON）

 getReceipt（receipts→JSON）

 getBalance/getCode/getStorageAt（StableState参照）

 eth_call / estimateGas（overlay REVM）

7.4 RPCハンドラ（update）

 sendRawTransaction（モードAかBで実装固定）

 （必要なら）独自 evm_produceBlock を足す

7.5 互換テスト

 viemで publicClient.getBlockNumber 等が通る

 ethersで provider.getBlockNumber 等が通る

 foundry（anvil代替用途）で最低限のcall/sendが通る

8) Phase2 合格条件

curl で eth_chainId / eth_blockNumber が返る

eth_sendRawTransaction → eth_getTransactionReceipt が通る（即時または確定後）

eth_call が動く（state変化なし）

同一stateなら同一レスポンス（決定性）

9) 明確に“やらない”宣言（Phase2を守る）

pending（latestのみ）

eth_getLogs / filter（ログ索引を作らない限り高コスト）

EIP-1559周りを真面目に（basefee固定で逃げる）

過去ブロックのstate参照（スナップショットが無いと無理）


Phase2（RPCノード）実装計画：canhttp前提に修正
1) 入口を2本に分ける（ICのHTTP仕様に合わせる）

http_request(req)（query）

GET/ヘルスチェック/静的な eth_chainId などはここで即返す

JSON-RPC の POST を受けたら、中身の method を見て

read系 → queryで処理して返す

write系（eth_sendRawTransaction, evm_produceBlock 等）→ upgrade = opt true を返して HTTP Gateway に http_request_update を呼ばせる

http_request_update(req)（update）

write系をここで実行（submit/produce_block へ接続）

これで「普通のHTTPクライアントからPOSTできない問題」を正攻法で回避できます（HTTP Gatewayの“upgrade to update”が前提）。

2) JSON-RPC のパースは canhttp 型を流用（作業削減ポイント）

serde_json::from_slice::<JsonRpcRequest<serde_json::Value>>() でまず受ける

methodごとに params を構造体へ再デシリアライズ（失敗→JSON-RPC error）

3) 最小実装メソッド（POC）

read/query：

web3_clientVersion / net_version / eth_chainId

eth_blockNumber

eth_getBlockByNumber（あなたの blocks から）

eth_call（overlay実行）

write/update：

eth_sendRawTransaction → submit_eth_tx → txhash返す

evm_produceBlock（独自）→ produce_block → block番号返す
※Ethereumクライアント互換を強くしたいなら “自前RPC名前空間” を切って混乱を避ける

4) DoS制限（Phase2でも必須）

body最大、JSONネスト深さ、paramsサイズ

1リクエストあたりの計算上限（特に eth_call）

upgrade（update化）するのは write系だけ（乱発されると重い）

「canhttp が使える部分のソース」完全出力（あなたが貼った流用対象）

これは あなたが提示した canhttp 由来の3ファイルを、そのまま“コピペで使える形”にまとめ直しただけです（新規に canhttp 本体の別コードは足してません）。

version.rs
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::{Display, Formatter};

/// Protocol Version
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Version {
    /// JSONRPC 2.0
    V2,
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Version::V2 => f.write_str("2.0"),
        }
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Version::V2 => serializer.serialize_str("2.0"),
        }
    }
}

impl<'a> Deserialize<'a> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_identifier(VersionVisitor)
    }
}

struct VersionVisitor;

impl<'a> Visitor<'a> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match value {
            "2.0" => Ok(Version::V2),
            _ => Err(serde::de::Error::custom(
                "expected JSON-RPC version \"2.0\"",
            )),
        }
    }
}

id.rs
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// An identifier established by the Client that MUST contain a String, Number, or NULL value if included.
///
/// If it is not included it is assumed to be a notification.
/// The value SHOULD normally not be Null.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Id {
    /// Numeric ID.
    Number(u64),

    /// String ID
    String(String),

    /// Null ID.
    ///
    /// The use of `Null` as a value for the id member in a Request object is discouraged,
    /// because this specification uses a value of Null for Responses with an unknown id.
    /// Also, because JSON-RPC 1.0 uses an id value of Null for Notifications this could cause confusion in handling.
    Null,
}

impl Id {
    /// Zero numeric ID.
    pub const ZERO: Id = Id::Number(0);

    /// Return `true` if and only if the [`Id`] is [`Id::Null`].
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl<T> From<T> for Id
where
    u64: From<T>,
{
    fn from(value: T) -> Self {
        Id::Number(value.into())
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Number(id) => Display::fmt(id, f),
            Id::String(id) => Display::fmt(id, f),
            Id::Null => f.write_str("null"),
        }
    }
}

request.rs
use serde::{Deserialize, Serialize};

// 上記で定義した Version, Id をインポートしてください
// use crate::http::json::{Id, Version};

use super::{Id, Version};

/// Body for all JSON-RPC requests, see the [specification](https://www.jsonrpc.org/specification).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    jsonrpc: Version,
    method: String,
    id: Id,
    params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    /// Returns the request ID, if any.
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// Returns the JSON-RPC method.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Return the JSON-RPC params, if any.
    pub fn params(&self) -> Option<&T> {
        self.params.as_ref()
    }
}

mod.rs（まとめ用：任意）
pub mod id;
pub mod request;
pub mod version;

pub use id::Id;
pub use request::JsonRpcRequest;
pub use version::Version;
