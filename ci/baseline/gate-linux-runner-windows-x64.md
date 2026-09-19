# gate の linux 側を runner から起動する代金(Windows x64)

`linux::runner` の効果。計測日: 2026-09-19、席 a。

- **BEFORE = `3c89270f`**(切り出しだけを終えた木)/ **AFTER = `4d35845d`**。
  この 2 点の `crates/platitude-core` / `crates/platitude-app` / `ui` は**バイト一致**なので、
  動詞が中で回す仕事は同じで、違うのは linux 側の動詞が何から起動するかだけ

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

`cargo xtask gate --all --fresh`(jobs 8、両側、4 本とも `steps 1054 / cached 0 / run 1049`)。
各 arm の前に `wsl --shutdown` → エンジン復帰待ち → `cargo xtask linux demo-repo basic` の暖機。
**wall clock は `target/gate-runs/` の記録の見出し行(gate 全体)**、sides はその `spent` 行。

| arm | gate 全体 | sides | linux 動詞 514 本 | host 動詞 |
|---|---|---|---|---|
| BEFORE #1 | 6m27s | 6m21s | 2742s | 2017s |
| BEFORE #2 | 5m43s | 5m42s | 2429s | 1738s |
| AFTER #1 | 6m02s | 5m55s | 2529s | 2230s |
| AFTER #2 | 14m34s ※ | 14m33s ※ | 2404s | 2125s(1 本赤) |

**BEFORE 2 本の幅の中に AFTER が入る** — gate 全体で 5m43–6m27 に対し AFTER #1 が 6m02s、
linux 動詞で 2429–2742s に対し AFTER が 2529s と 2404s。平均の差(2586 → 2467s、−4.6%)は
上の見積もり −8% と同じ向き・同じ桁だが、**n=2 では分離していない**。

※ **AFTER #2 の wall clock は比較に使えない**。この run は `verify settings-tools`(**host 側**の動詞)が
`settled=false loading=true choices=0` のまま **601.8 秒**待って watchdog で終わったもので、
**run が全体に遅かったのではない**: 同じ run の 2 番目に長い単位は 55.8s(ほかの arm の最長 1m14s / 1m24s より短い)、
linux 動詞の合計 2404s は 4 本中**最短**。これは P3 §マージエディタの探索 に記録のある既知の停止で、
**この変更が触らない側**(host)で起きている。**この停止を「機械が重かった」の根拠にしない**
(P3 に同型の否定あり)。並列実行なので全体時間から 600 秒を引くのも不適切。
停止した単位を除いた動詞合計は上表のとおり比較できる。

## VM のメモリ — **改善は確認できない**

2 秒ごとに `vmmemWSL` のワーキングセットと `wsl -d docker-desktop -- cat /proc/meminfo` を読む。
**メモリは名前空間で割れない**ので、この `/proc/meminfo` はコンテナを含む VM 全体。
`MemFree` は素の空きなので、回収できるキャッシュを含む `MemAvailable` も並べる。

| arm | vmmemWSL ピーク | Cached ピーク | AnonPages ピーク | MemFree 最小 | MemAvailable 最小 |
|---|---|---|---|---|---|
| BEFORE #1 | 7114 MB | 2962 MB | 1663 MB | 9115 MB | 13010 MB |
| BEFORE #2 | 5775 MB | 1976 MB | 1582 MB | 10564 MB | 13101 MB |
| AFTER #1 | 7015 MB | 3038 MB | 1818 MB | 9062 MB | 12823 MB |

**幅の中に入るのは vmmemWSL の列だけ**で、Cached・AnonPages・MemFree 最小・MemAvailable 最小の
4 列は AFTER が BEFORE 2 本の範囲の外、**わずかに多く使う側**に出ている。
**BEFORE が n=2・AFTER が n=1 なので、改善も悪化も断定できない。原因は未特定**。
上の 144MB の見積もりは 16GB の VM に対してこの分解能では見えない。
`MemAvailable` は 3 本とも 12.8GB を下回らない。

## 言えること / 言えないこと

- **実測で言える**: 起動器 1 本の時間とメモリ(範囲が重ならない)
- **算術で言える**: linux 側から起動器時間 200 秒強と、同時 8 本分の footprint が消える
- **言えない**: gate 全体の実時間の短縮。区間を揃えても BEFORE の幅の中
- **言えない**: VM のメモリの改善。4 列で AFTER が範囲外(わずかに多い側)、n が足りず原因も未特定
- **測り直すなら**: arm ごとの n を増やす。**ホストの空きメモリの低下は外部負荷の証拠にならない**
  —— gate 自身と vmmemWSL がその主な出所で、切り分けには arm ごとの同時刻サンプルが要る
