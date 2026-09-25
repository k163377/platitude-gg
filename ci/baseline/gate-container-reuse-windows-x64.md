# gate の linux 側のコンテナを gate 単位で再利用する代金(Windows x64)

`--all --fresh` の gate の linux 側の動詞(`verify-linux` + `qmltest-linux`)を、1 本ごとに `docker run --rm` で
コンテナを作って捨てる形(A)と、gate 所有の 1 コンテナへ `docker exec` で入れる形(B = `linux::container`)で、
同じ仕事で比べたもの。B の差分は xtask の linux 側と gate の起動経路だけで、`platitude-core` / `platitude-app` / `ui` は
バイト一致。テスト対象・並列度(jobs 8)・予算・キャッシュ条件(`--fresh`)・image・volume は
両 arm で同じ。`~/.wslconfig` は上限なし。

## 仮説と採否基準(**実装前に書いた**。結果を見て動かさない)

判定するのは 3 つで、**別々に**判定する。コンテナ数が減ったことは、どれの証拠にもならない。

| # | 問い | 指標(読む場所) | 「意味のある改善」の幅 |
|---|---|---|---|
| 1 | 起動・終了の所要時間 | 小経路: 同じ動詞(`verify-ui … --no-build`)を A / B で逐次 10 本・8 並列 16 本、1 本あたりの wall clock の差。gate: `target/gate-runs/<run>.txt` の見出しの wall clock と、`units.tsv` の linux 動詞 `ran_ms` の和 | 小経路で **1 本 −0.3 s 以上**(515 本 ÷ 8 並列 ≒ gate 20 秒に相当)。gate 全体は **A→B・B→A の両順で B が A より短く、その差が同セッションの A–A の幅を超える**時だけ「短縮」。幅の中なら **未判定** |
| 2 | cgroup の作成・破棄に伴うカーネルメモリ | VM の `Percpu` と `SUnreclaim` の **gate 開始 → gate 終了**の増分(`vm.txt`。終了後の窓は `initd` の走査が混ざるので含めない) | A の `Percpu` 増分(既存実測 +25 MB/gate)に対し **B が半分以下、両順で**。差が 10 MB 未満なら分解能の外 = **未判定** |
| 3 | Docker Desktop の処理とホストの低空き | gate + 終了後 1200 s の窓で: ホスト空き最小・2.5 GB 未満の継続秒数(`host.csv`)、`vmmemWSL` の最大と終値、`/initd services` の `read_bytes` 増分(`vm.txt`)、Docker の `init.log` の同じ窓の `fstrim` 行数 | **両順で** ホスト空き最小が **+500 MB 以上**高く、低空きの継続が短く、**かつ機構の証人**(`fstrim` 行数と `initd` の読み取り量)が同じ向きに動いた時。空きだけ動いて証人が動かなければ開始状態の差 = **未判定** |

- **B の cgroup `memory.peak`** はコンテナの生存期間全体の値。従来のステップ単位のピークと並べない

## B の方式(比較した実装)

**gate 1 本につきコンテナ 1 つ**(`linux::container`)。並列枠ごとの複数コンテナは、隔離で得るものが無いのに
寿命管理が枠の数だけ増えるので採らない。

## 小経路の比較

同じコピーから同じ動詞 `verify-ui app-menu --no-build` を、A = 動詞ごとに `docker run --rm`、
B = gate と同じ mount・env・`--init` で手で起こしたコンテナへ `docker exec` で撃った。
起動器は `target/debug/xtask.exe` 直(cargo の起動を挟まない)。ホスト側の wall clock は `Measure-Command`、
中の時間は動詞自身の `spent … whole=` 行。

**逐次 10 本**(ms。中央値 / 範囲):

| | wall clock | 中の whole | 差 = 起動と終了の代金 |
|---|---|---|---|
| A(コンテナごと) | **1840**(1789–2377) | 1143(1141–1449) | **≈ 0.70 s** |
| B(exec) | **1428**(1349–1645) | 1149(1146–1392) | **≈ 0.28 s** |

**1 本あたり −0.42 s**(基準の −0.3 s を満たす)。中の時間は同じ(動詞の仕事は変わっていない)。

**8 並列 × 2 周**(1 周 = 8 本を `Start-Process` で同時に起こし全部の終了まで):

| | 1 周目 | 2 周目 | 中の whole の平均 |
|---|---|---|---|
| A | 5106 ms | 5221 ms | 1439 ms |
| B | **4244 ms** | **3139 ms** | 1373 ms |

並列の絶対値には **手撃ちならではの代金が両 arm 共通で乗っている**: 手で撃った `linux` は HELD の印を持たないので
1 本ずつ機械の予算から weight 4 の ticket を取り(`verify-ui` は `--no-build` でも `weight_of` に
`no_build=false` で問われる)、8 本 × 4 = 32 > 24 で末尾の 2 本が部屋を待つ。gate の中では動詞は LIGHT で
host 側の `linux` は HELD なので、この待ちは無い。

## gate の比較

`cargo xtask footprint --settle 1200 gate --all --fresh`、jobs 8、両 arm とも直前に host と container の
release ビルドを 1 動詞(`verify-ui commit`)で暖めてから。生の行は
`target/footprint/<run>/`(`host.csv` / `vm.txt` / `docker-*.txt`)と `target/events-<arm>.txt`。

**窓の条件が既存記録と違う**: この機械の Docker Desktop は **Resource Saver が有効**で、コンテナが
1 つも無いまま約 4〜5 分経つと VM を落とす(`init.log` に `poweroff` 一式、`vmmemWSL` が消える)。
gate 終了後の窓は両 arm ともそこで VM が消えるので、**「終了後 +6〜11 分の低空き」は起きようがなく、
窓の後半はホストの空きが VM 分だけ戻る**。既存記録の窓とは並べない。
`init.log` は gate 中 40〜50 秒で 1 MB ずつ回転し、残るのは末尾の数分だけなので、`fstrim` の回数は
**残った区間の per-minute** でしか比べられない。

| | A(1 本目) | **B** | A(2 本目) |
|---|---|---|---|
| gate | 8m02s | **7m24s** | **7m27s** |
| linux 動詞の和 / host 動詞の和 | 2889 / 3465 s | **1827** / 3263 s | 2619 / 3285 s |
| コンテナ 延べ / 同時最大 | 481(過小)/ 9 | 11 / 2 | **548** / 9 |
| ホスト空き 最小 / 2.5 GB 未満の秒数 | 53 MB / 214 s | 170 MB / 348 s | 190 MB / 334 s |
| `vmmemWSL` 最大 | 5684 | **6920** | 5309 |
| VM gate 中 Percpu / SUnreclaim | +8 / +19 | **+1** / +28 | +9 / +39 |
| `initd` gate 中の読み取り / 窓 | 1054 / 934 MB | 1663 / 1033 MB | 1727 / 1020 MB |
| `fstrim`(残った区間の per-minute) | 9.5 /分 | **1.2 /分** | 5.6 /分(31 行 / 5.5 分) |
| 開始 VM の状態 | gate 2 本の後 | 素(再起動後) | 素(再起動後) |

## 実経路の確認

| 経路 | 確認 |
|---|---|
| 連続・並行ステップの隔離 | 手起こしコンテナで 10 本逐次 + 16 本 8 並列: `/root` 書き込み 0、`/tmp` は claim の lock だけ、印は全部外れる、プロセスの残り 0 |
| ステップ失敗 | 無い動詞は app の watchdog までの赤で、`exited 1` と `pgg-probe ran` 行が host に返る |
| タイムアウト(印による停止) | 宙吊りの動詞へ `linux --container … --step … stop`: **`took 3 (platitude-gg / sh / xtask), 0 left`**、印のファイルも消えた |
| 親(gate)の死 | `gate --fresh` を動詞 8 本が走っている最中に `Stop-Process -Force`。走っていた動詞は全部終わり、新しい動詞は入らず、**最後の動詞から 300 s でコンテナが自ら抜けた**(`--rm` で消える) |
| コンテナ消失 | 動詞の最中に `docker rm --force` → host に `exited 137`、以後の exec は `No such container` で赤。フォールバック無し |
| 別 gate / 別席への非干渉 | 準備の掃除は同じ木の label だけを見て、note の pid を読み直して live を spare する(手起こしのコンテナは次の gate の準備が消した) |
| 起動と終了の代金 | 起動 0.3〜0.5 s、回収 0.4〜0.6 s(gate 1 本につき 1 回ずつ) |

## 言えること / 言えないこと

- **実測で言える(時間)**: 起動器の代金は 1 本 **−0.42 s**(逐次、範囲が重ならない)。gate の中では linux 動詞の和が
  **A 2619〜2889 s に対し B 1827 s** — 両順で A の範囲の外(−30〜−37%)。**8 並列の `docker run` は互いに待ち、
  exec は待たない**のがその差(小経路の 8 並列で A の起動器の外側 ≈ 3.7 s/本)
- **言えない(時間)**: **gate 全体の壁時計**。A 8m02s / 7m27s に対し B 7m24s で、A–A の幅(35 s)の中。
  この計測の壁時計は host 側(`test it (all)` 2m31〜3m13s + host 動詞の和 3265〜3465 s ÷ 8)が決めていて、
  linux 側の −800〜−1060 s は host 側が終わるのを待つ間に消える
- **未判定(cgroup のカーネルメモリ)**: Percpu の gate 中の増分は A +8 / +9 に対し B +1 で向きは仮説どおりだが、
  A の増分そのものが実装前に決めた分解能(10 MB)の内側。SUnreclaim は A +19 / +39 に対し B +28 で範囲の中。
  **既存記録の「Percpu +25 MB/gate」は連続 3 本の蓄積で、素の VM に 1 本なら 8〜9 MB**
- **未改善(ホストの低空きと Docker Desktop)**: 機構の証人(コンテナ 548 → 11、`fstrim` 5.6〜9.5 → 1.2 /分)は
  動いた。**しかし症状は 1 つも動いていない**: ホスト空きの最小、2.5 GB 未満の継続、`vmmemWSL` の最大、
  `initd` の gate 中の読み取りは全部 A の範囲の中か悪い側。**低空きの底はどちらも gate 開始 40〜75 秒**
  (host 側の runner ビルド・`test it`・clippy とコンテナ側のビルドが同時に立つ所)で、コンテナの作成・破棄が
  無くなっても動かない場所。**`initd` の読み取りはコンテナ数に比例しない**(11 本の gate で 1663 MB)
- **言えない**: 「cgroup を作らないこと」が Docker Desktop の走査(`/initd services`)を減らすか。読み取り量は
  減っていないので、走査の条件はコンテナの削除ではない
- **採否の読み**: **WSL メモリの改善を目的とするなら、この方式は効かない**(3 本の実測で 1 つも動かず)。
  効くのは linux 側の時間だけで、その利益が gate の壁時計に出るのは linux 側が壁時計の時だけ。
  管理の複雑化(gate 所有の寿命・印による停止・3 段の回収)は `linux::container` 1 モジュールに閉じ、
  実経路で隔離・赤の伝播・停止・消失・掃除・idle 天井を確かめてある(§小経路の比較、§実経路の確認)
