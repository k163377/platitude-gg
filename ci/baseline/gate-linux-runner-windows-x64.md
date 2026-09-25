# gate の linux 側を runner から起動する代金(Windows x64)

`linux::runner` の効果。BEFORE(切り出しだけを終えた木)と AFTER の `crates/platitude-core` / `crates/platitude-app` / `ui` は
**バイト一致**なので、動詞が中で回す仕事は同じで、違うのは linux 側の動詞が何から起動するかだけ。

## 起動器 1 本の代金 — **実測、範囲が重ならない**

同じ image・同じ mount で「コマンド一覧を出して終わる」だけを両方の道から。コンテナの
cgroup の `memory.peak` と、中で測った経過時間。3 回ずつ。

| | `cargo xtask` 経由 | 据えたコピーから |
|---|---|---|
| **ピークメモリ** | **23.9 / 24.3 / 25.0 MB** | **6.4 / 5.9 / 6.4 MB** |
| **経過** | **404 / 390 / 391 ms** | **1 / 1 / 1 ms** |

再現: 中身は `sh -c '…; cat /sys/fs/cgroup/memory.peak'` の 1 行で、片方は `cargo xtask`、
もう片方は同じ image で `cargo build -p xtask` して写したバイナリ。

## そこからの**算術上の見積もり**(実測ではない)

`--all` の linux 側は 523 ステップ中 **515 本**がこの動詞(`verify-linux` 514 + `qmltest-linux`)。

- 起動器の時間 **515 × 0.39–0.41s ≒ 200–210 秒**が linux 側の動詞の合計から消える
- 動詞は既定 8 並列なので、**gate の実時間では ≒ 25 秒**
- 同時に生きる起動器の footprint **8 × 約 18MB ≒ 144MB**

## gate 1 本 — **区間を揃えても分離しない**

`cargo xtask gate --all --fresh`(jobs 8、両側、`run 1049`)を BEFORE / AFTER 2 本ずつ。
各 arm の前に `wsl --shutdown` → エンジン復帰待ち → `cargo xtask linux demo-repo basic` の暖機。

- **BEFORE 2 本の幅の中に AFTER が入る** — gate 全体で 5m43–6m27 に対し AFTER 6m02s、
  linux 動詞で 2429–2742s に対し AFTER 2529s と 2404s。AFTER の 2 本目の gate 全体は、host 側の動詞 1 本が
  watchdog まで止まった run なので比較に使えない
- **VM のメモリは改善を確認できない** — `vmmemWSL` のピークは幅の中、Cached・AnonPages・MemFree 最小・MemAvailable 最小の
  4 列は AFTER が BEFORE 2 本の範囲の外(わずかに多く使う側)。n が足りず原因も未特定。
  上の 144MB の見積もりは 16GB の VM に対してこの分解能では見えない
