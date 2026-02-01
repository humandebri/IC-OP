// どこで: ローカルアーカイブ / 何を: 1ブロック分のpayloadをzstd圧縮保存 / なぜ: prune前の保険のため

import { createHash } from "crypto";
import { promises as fs } from "fs";
import path from "path";
import { compress } from "@mongodb-js/zstd";

export type ArchiveInput = {
  archiveDir: string;
  chainId: string;
  blockNumber: bigint;
  blockPayload: Uint8Array;
  receiptsPayload: Uint8Array;
  txIndexPayload: Uint8Array;
  zstdLevel: number;
};

export type ArchiveResult = {
  path: string;
  sha256: Buffer;
  sizeBytes: number;
  rawBytes: number;
};

export async function archiveBlock(input: ArchiveInput): Promise<ArchiveResult> {
  const raw = buildRaw(input.blockPayload, input.receiptsPayload, input.txIndexPayload);
  const compressed = await compress(raw, input.zstdLevel);
  const sha256 = createHash("sha256").update(compressed).digest();
  const outPath = buildPath(input.archiveDir, input.chainId, input.blockNumber);
  const dir = path.dirname(outPath);
  await fs.mkdir(dir, { recursive: true });
  const tmpPath = `${outPath}.tmp`;
  await fs.writeFile(tmpPath, compressed);
  await fs.rename(tmpPath, outPath);
  return {
    path: outPath,
    sha256,
    sizeBytes: compressed.length,
    rawBytes: raw.length,
  };
}

function buildRaw(block: Uint8Array, receipts: Uint8Array, txIndex: Uint8Array): Buffer {
  const blockLen = toU32(block.length, "block_len");
  const receiptsLen = toU32(receipts.length, "receipts_len");
  const txIndexLen = toU32(txIndex.length, "tx_index_len");
  const total = 4 + blockLen + 4 + receiptsLen + 4 + txIndexLen;
  const out = Buffer.allocUnsafe(total);
  let offset = 0;
  offset = writeLen(out, offset, blockLen);
  Buffer.from(block).copy(out, offset);
  offset += blockLen;
  offset = writeLen(out, offset, receiptsLen);
  Buffer.from(receipts).copy(out, offset);
  offset += receiptsLen;
  offset = writeLen(out, offset, txIndexLen);
  Buffer.from(txIndex).copy(out, offset);
  return out;
}

function writeLen(buf: Buffer, offset: number, len: number): number {
  buf.writeUInt32BE(len, offset);
  return offset + 4;
}

function toU32(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) {
    throw new Error(`${label} out of range`);
  }
  return value;
}

function buildPath(baseDir: string, chainId: string, blockNumber: bigint): string {
  const fileName = `${blockNumber.toString()}.bundle.zst`;
  return path.join(baseDir, chainId, fileName);
}
