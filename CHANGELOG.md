# Changelog

## 2026-02-06

- fix(prune): remove `seen_tx` entries when pruning blocks and during journal recovery.
- fix(time): align block timestamps to wall-clock time.
- fix(gas): reduce default block gas limit to 3_000_000 to avoid IC instruction overruns.
- test: add coverage for seen_tx pruning and receipt encode fallback.

## 2026-02-03

- breaking(ops): `ic-evm-wrapper` install now requires `Some(InitArgs)`; empty args and `opt none` are not supported.
- ops: added `scripts/lib_init_args.sh` and updated install scripts to always pass generated `InitArgs`.
- test: aligned rpc e2e install flow with mandatory `InitArgs` and shared caller->EVM derivation.
