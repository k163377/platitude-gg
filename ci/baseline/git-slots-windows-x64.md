# git の実行枠の実測(Windows x64)

`process::Slots`(アプリ全体で共有する git プロセスの実行枠)の既定値 —
同時に走らせる本数 N(`settings::Defaults::git_concurrency`)と、そのうち背景の読み・
ネットワークに paced されたコマンドが分け合う幅 K(`Limits::shared()`)— を決めるための実測。
**読むのは変種どうしの差**で、予算行(操作応答 100ms 等)との絶対値は
[perf-windows-x64.md](perf-windows-x64.md) §判定 が正。

## 条件

- 対象: `cargo xtask corpus` の合成リポジトリに **`cargo xtask corpus --copies 8` で 8 つの
  作業コピーを建てた状態**(各 109,652 ファイルのチェックアウト + untracked 1 ファイル)。
  枝が 8 本増えるので **corpus token は `fb3d090c4261f4500bd61c8db7389485d515feab`(refs 50,012)**
  — perf 記録の token とは別物で、この記録の中でだけ比較する
- 撃ち方(1 変種 1 行。下の 3 つの表はどれも同じ exe — 予約の分け方の A/B だけが 2 本を交互に撃つ):

  ```
  PGG_ALLOW_GUI=1 cargo xtask perf --repo <corpus> --corpus fb3d090c… --at <commit> --runs 3 --no-font-walk --allow-noisy --cases <cases.tsv> --cycles 12 --setting git_concurrency=<N> --setting copies_interval_secs=<秒> --label slots-quiet-<N>
  ```

  `cases.tsv` は `cargo xtask corpus` が印刷する 2 行(`newest` = HEAD `e512482e`、76 ファイル変更・
  開くのは 34,059 バイトの java / `second` = その下の行 `336dd534`)。**1 変種 24 操作 × 3 run = 72 操作**
  (2 コミット × 12 周の行選択 → details → diff)。`copies_interval_secs` は 5(操作と巡回を
  重ねるための間隔)/ 30(出荷既定)/ 0(巡回なし)の 3 通り
- 機械: perf 記録と同じ台(Ryzen 9 9900X 12C/24T、DISPLAY2 180Hz、RTX 3070 D3D11、git 2.55、
  Qt 6.10.3)。**静かな机** — 他セッションのビルドもコンテナも無く、WSL は落としてある(空き 15GB
  = 6.4GB の pack が page cache に載る条件)。巡回なしの run で機械 10–13% busy・対象以外 5–7%、
  巡回 5 秒の run で 39–51% busy・対象以外 8–12%。`--allow-noisy` を外せないのは巡回そのものの
  kernel / Defender 側の代金が sampler の「対象以外」に数えられて閾を越えるため
- 読み方は **p50 と「100ms を越えた操作の割合」**。**p95 と max は変種の差を読めない** — 同じ N=8 を
  背中合わせに 2 回撮ると diff の p95 は 346 → 589ms 動く(p50 は 86.5 → 79.1)。n=72 の裾は
  run の揺れが変種の差より大きい

## 枠の床 — 巡回を止めた机で、クリックが詰まらないのは何本からか

`copies_interval_secs=0`(他コピーを読まない)。背景が 1 本も走らないので、**測っているのは
`total` だけ**(interactive かつ手元ペースのコマンドは reserve の外の枠も取れる = `State::eligible`)。

| 枠 N | details p50(newest / second)| diff p50(newest / second)| 100ms 超(72 操作中)| 起動→グラフ min–中央–max(うち walk)|
|---|---|---|---|---|
| 2 | 67.6 / 35.1 | **102.0 / 93.5** | details 4.2% / **diff 36.1%** | 1522–1525–1535(609–614–641)|
| 3 | 67.2 / 34.4 | 72.5 / 67.3 | details 1.4% / diff 13.9% | 1147–1150–1262(230–232–340)|
| 4 | 65.1 / 34.7 | 75.8 / 69.1 | details 1.4% / diff 8.3% | 1135–1171–1189(185–258–269)|
| 8 | 66.2 / 34.6 | 72.7 / 67.7 | details 0.0% / diff 2.8% | 1095–1115–1121(157–162–204)|

- **枠 2 本だけが中央値を壊す**。**待つのは毎回 `rev-parse --verify`**(`--log debug` の run: 枠待ち
  28–82ms が全 diff に付き、`diff-tree` / `check-attr` / `cat-file` は 0)= diff の直列 2 本の頭。
  **1 本(設定として受ける下限)は撃っていない**
- **3 本で中央値は戻り、裾は 4 本以上で収まる**(100ms 超 13.9% → 8.3% → 2.8%)
- **起動も枠 2 本で伸びる**(walk 614ms 対 162ms)— 開幕は refs / status / walk が同時に走る

## 巡回と重なった時(5 秒ごとに 8 コピー)

`copies_interval_secs=5`。1 周が 2.5〜3.6 秒なので**操作の半分以上が巡回と重なる**。

| N(背景の幅 K)| details p50(newest / second)| diff p50(newest / second)| 100ms 超 | 巡回 1 周 min–中央–max [ms] |
|---|---|---|---|---|
| 2 (1) | 77.4 / 40.1 | **174.4 / 161.9** | details 12.5% / **diff 94.4%** | 3263–3581–4123 |
| 4 (2) | 69.9 / 36.1 | 90.5 / 82.7 | details 13.9% / diff 37.5% | 2512–2596–2890 |
| 6 (3) | 69.3 / 37.7 | 83.6 / 80.2 | details 12.5% / diff 25.0% | 2379–2518–2732 |
| 8 (4) | 66.9 / 35.1 | 86.5 / 75.1 | details 13.9% / diff 22.2% | 2264–2473–3175 |
| 16 (8) | 69.3 / 42.9 | 79.0 / 72.8 | details 15.3% / diff 23.6% | 2300–2469–2653 |

- **巡回の幅は 2〜3 で頭打ち** — **枠より先に機械が尽きる**(1 本の `status -uall` が preload-index で
  約 11 コア幅)
- **N=8 ではクリックは枠を 1 度も待たない**(`--log debug`: probe 24 本中 23 本が待ち 0ms、
  `diff-tree` / `check-attr` / `cat-file` は 72 本すべて 0)。**巡回中の遅れは CPU**
- **N=2 は巡回の有無に関わらず崩れる**(diff p50 174ms)。出荷既定の 30 秒間隔でも
  **details p50 53.5 / diff p50 109.8・diff の 63.9% が 100ms 超**(機械 17% busy)

## 予約の分け方 — 1/2 対 1/4(ABBA の A/B、各 4 サンプル = 96 操作)

A = 背景と elsewhere が `max(1, n/2)`、B = `max(1, n/4)`。巡回 5 秒・同じ exe 対、交互に撃つ。

| N | 変種(背景の幅 / click の予約)| details p50 / p90 / max / 100ms 超 | diff p50 / p90 / max / 100ms 超 | 巡回 1 周 |
|---|---|---|---|---|
| 8 | A 4 / 4 | 63.9 / 115.0 / 477.9 / 11.5% | 76.3 / 260.4 / 609.9 / 18.8% | 2312–2535–2732 |
| 8 | B 2 / 6 | 62.9 / 139.3 / 444.1 / 13.5% | 78.4 / **140.0** / 424.2 / 22.9% | 2486–2728–2999 |
| 4 | A 2 / 2 | 63.5 / 117.3 / 817.6 / 13.5% | 85.7 / 277.9 / 585.8 / 36.5% | 2532–2712–2904 |
| 4 | B 1 / 3 | 60.6 / **87.5** / **106.4** / **2.1%** | 88.8 / **157.9** / **282.0** / 34.4% | 3318–3548–3793 |

- **効くのは予約が 3 本に届く所**(N=4)。details の 100ms 超が 13.5% → 2.1%、max が 818 → 106ms
- **N=8 では中立**(予約は元から 4 本)。裾の差は 1 サンプル 24 操作の揺れの中
- **代金は巡回の 1 周**(N=8 で +8%、N=4 で +31%)。出荷既定の 30 秒間隔に対して duty 9% → 12% で、
  **1 周のたびに 8 コピー全部を読み切ることは変わらない**(`carrying=8` が全 pass)

## 既定値(この実測で決めた)

- `git_concurrency` = **`process::default_concurrency()` = 機械のスレッド数 / 3 を 4..=8 に収めた数**
  — 床 4 は §枠の床、天井 8 は 16 との差が無いことから(§巡回と重なった時)
- **`Limits::of(n)` = 全体 n・click の予約 n − max(1, n/4)** — §予約の分け方
- `copies_interval_secs` = **30**(1 周 2.5–3.6s に対して 1 割の duty)
- `OVERTAKEN_LIMIT` = **4**(全 pass が 8 コピーを読み切っている = aging が足りない兆候は出ていない)

## 操作 1 回の内訳 — 枠待ち / プロセス起動 / git の仕事

巡回を止め(`copies_interval_secs=0`)、既定の N(この台で 8)で `--log debug` を付けた 2 run
(12 操作 × 2)。executor が 1 コマンドごとに残す `waited_ms`(枠待ち)・`spawn_ms`
(`CreateProcess` の呼び出し)・`elapsed_ms`(spawn → reap)。

| コマンド(操作経路)| 本数 | 枠待ち | 起動 平均 / max [ms] | 実行 平均 / max [ms] |
|---|---|---|---|---|
| `show`(details)| 24 | 0 | 3.4 / 22 | 37.8 / 64 |
| `diff-tree`(diff)| 24 | 0 | 2.2 / 18 | 43.2 / 59 |
| `rev-parse --verify`(blob の存在確認、diff の直列 1 本目)| 28 | 0 | 4.0 / 20 | 25.4 / 39 |
| `cat-file`(色の元テキストと commit、diff の直列 2 本目)| 48 | 0 | 2.7 / 22 | 23.0 / 40 |
| `check-attr`(改行の裁定)| 24 | 0 | 2.7 / 21 | 38.8 / 56 |
| `status --porcelain`(poll)| 4 | 0 | 3.8 / 9 | 446.8 / 478 |
| `for-each-ref`(poll)| 10 | 0 | 10.8 / 23 | 296.4 / 540 |
| `log`(walk)| 4 | 0 | 3.8 / 5 | 486.8 / 786 |

- **枠待ちは 0** — 既定の枠で巡回が無ければ、操作経路のコマンドは 1 本も並ばない
- **実行 = ほぼプロセスの寿命**: `rev-parse` も `cat-file` も git 自身の仕事は 1ms 級(下の probe で
  素のプロセスは 10–11ms)。**アプリ経由で 23–25ms** なのは tokio / pipe / observer の往復が乗るため
- **diff を開く 1 点の critical path は `rev-parse` → `cat-file` の直列 2 本**(`diff-tree` /
  `check-attr` と並走する 3 本の最長)= 静かな机で約 50ms。これが diff の p50 70ms の大半

## プロセス再利用 — `cat-file --batch` の常駐

`cargo xtask corpus --probe`(std だけの一発測定。corpus に対して 20 回、min / 中央、max)。
**製品と同じ条件**(`FIXED_ARGS` / `FIXED_ENV` を写している)。

| 読み方 | 1 回あたり [ms] |
|---|---|
| `cat-file blob <HEAD:path>`、プロセス 1 本ずつ | 10 / 11(max 27)|
| `rev-parse --verify -q <spec>`、プロセス 1 本ずつ(存在確認)| 10 / 10(max 12)|
| `show --no-patch --format=%H HEAD`、プロセス 1 本ずつ(最小の `show`)| 35 / 36(max 37)|
| **`cat-file --batch` 常駐 1 本に stdin で 20 回** | **0 / 0(max 8 = 最初の 1 回)** |
| **`cat-file --batch-check` 常駐 1 本に 20 回** | **0 / 0(max 8)** |
| `--batch-check` に無いパス(`HEAD:no/such/path`)| 0 バイト・0ms で「missing」— **終了コードは変わらず、プロセスは生きたまま** |

常駐 1 本の常駐メモリ: 20 読みの後で **548 KB**(`tasklist`。pack の写像は触ったページだけが乗る)。

**結論: 保留**。設計と採否の条件は [P3-確認事項](../../internal-docs/P3-確認事項.md) §core
「`cat-file --batch` の常駐」。

## `status` の高速化機構 — untracked cache / fsmonitor

同じ probe。**製品と同じ `GIT_OPTIONAL_LOCKS=0` + `--no-optional-locks`** で、コピー 1(index を
書く変種のため)に対する `status --porcelain=v2 -z --branch -uall` を 5 回。機構を入れる書き込み
(`update-index --untracked-cache`、初回の status)は錠ありで 1 度だけ行い、その後は製品条件で測る。

| 変種 | min / 中央 [ms](max)|
|---|---|
| 製品条件のまま | **372 / 382**(395)|
| untracked cache on(錠ありの status で 1 度満たしてから)| 365 / 377(407)|
| fsmonitor on(初回の status = daemon 起動・錠あり: 398ms)、その token のまま | 369 / 372(387)|
| fsmonitor + untracked cache、どちらも 1 度満たしてから | 383 / 393(418)|

**結論: 現状のまま**。**どれも製品条件の誤差の中**。製品条件では index が二度と書かれないので
token も cache も更新されない。効かせるには status のたびに index を書く = 他の作業コピーの
`index.lock` を握る側へ戻ることになり、それは巡回が他所の木を読める前提(rules-refs/core.md の
`--no-optional-locks` の項)を崩す。

## ペインが立っているコピーの読み

**ペインが 1 つのコピーを開いている間も、tick ごとの読みはコピー 1 つに `status` 1 本** —— 行の集計の
pass(全コピー)が、ペインの立つコピーの `status --porcelain=v2 -z --branch -uall` をペインの一覧へも
配る(`carried::Pane`)。ペインが自分で読む(`RepoSession::read_carried_status`)のは、選んだ瞬間・
フォーカス復帰・pass が始まらなかった tick だけ。

- **ペインが自分で読む 1 本の代金は status 1 本ぶん**: 製品条件のコピー 1 で **405 / 418ms**(5 回の
  min / 中央、max 442 = §`status` の高速化機構 と同じ probe)。**フォーカス復帰は pass も撃つ**ので、
  その時だけそのコピーは 2 度読まれる(8 コピーなら 9 本)
- **verify-ui の run が持つのは別の時間**: あちらは `GIT_CONFIG_NOSYSTEM=1` で system の
  gitconfig を読まない環境なので、**同じコピーの status が 6.1–6.9s**(同じ日・同じ機械・
  `carried-read` を `--repo <corpus>` で撃った 2 run の全 status。8 コピーの pass は 25–26s)。
  この機械の system config は `core.fscache = true` を持つが、**どの設定がこの差かの切り分けは
  していない**

## 再実行

**入口で `cargo xtask corpus --copies 8`**(8 × 109,652 ファイルのチェックアウトで数分)→ §条件 の 1 行 →
**出口で `cargo xtask corpus --copies 0`**(立てたままだと corpus token がこの記録の側に固定され、
[perf-windows-x64.md](perf-windows-x64.md) §判定 を撃ち直せない)。内訳は同じ行に `--log debug --runs 2 --cycles 6`、probe は
`cargo xtask corpus --probe`(コピー 1 が要る = 入口を通っていれば立っている)。**変種を跨いで比べるなら
同じ座りで背中合わせに撮る**(§条件 の最後 — p95 と max は run の揺れで動く)。
