# 依存更新の前後(Windows x64)

版を動かす更新(crate・toolchain・Qt)が性能を動かしたかの答え。**最後の更新の 1 本だけを置く** —
次の更新はこの節を置き換える。予算に対する数字の正本は [perf-windows-x64.md](perf-windows-x64.md)。

| | 更新前(A) | 更新後(B) |
|---|---|---|
| commit | main `6ec1e2e4d` | `9a404058a`(A に依存更新の commit だけを足した木) |
| 動いた版 | | rustc 1.99.0 → 1.99.1・`Cargo.lock` の版 29 本(tokio 1.53.2 → 1.54.1・mry 0.16・tree-sitter-php 0.25 ほか。増減は無し) |
| 共通 | Qt 6.12.0・`--features automation` の release・同じ台・同じコーパス | |

- 撃ち方: `PGG_ALLOW_GUI=1 cargo xtask perf --at 6ec1e2e4d --compare 9a404058a --corpus <token> --runs 3`
  (ABBA の 3 ブロック = 各側 6 回。各回が較正 run と捨て 1 を持つ)
- 合成コーパス(token `23da9ebd`)、warm、`\\.\DISPLAY2` 180Hz・D3D11 / RTX 3070、点灯・アンロック、
  `--allow-noisy` なし(machine の busy 9.9〜13.6%、対象以外 4.9〜8.4%)

値は min–中央–max(各側 6 回)。**どの行も両側の範囲が重なり、差は判定できない**:

| 項目 | 更新前 | 更新後 |
|---|---|---|
| net(MB) | 371.0–374.2–378.6 | 374.5–377.1–382.9 |
| private(MB) | 343.7–348.8–354.5 | 349.4–351.8–357.1 |
| 起動 → グラフの初回フレーム(ms) | 1580–1597–1710 | 1590–1625–1719 |
| スクロール(fps)/ frame p99(ms) | 179.6 / 7.6 | 179.6 / 7.7 |
| 詳細 / diff の初回フレーム(ms) | 109.7 / 119.2 | 96.0 / 111.1 |

- net は 3 ブロックとも B が 0.9〜6.1MB 上だが、範囲は重なる
- 両側とも net が予算 300MB を越え、較正 run の歩きがほぼ 0 なのは更新と無関係な main の状態
  ([P3-確認事項.md](../../internal-docs/P3-確認事項.md) §性能)
- 証跡は席 d の `target/perf/windows-x86_64/compare-1792820849772981400`(`*-A` / `*-B`、順は `order.tsv`)
