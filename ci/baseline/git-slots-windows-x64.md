# git の実行枠の実測(Windows x64)

計測日 2026-09-13。`process::Slots`(アプリ全体で共有する git プロセスの実行枠)の
既定値 — 同時に走らせる本数 N(`settings::Defaults::git_concurrency`)と、他の作業コピーを
読む巡回の幅 K = N/2(background の上限)— を決めるための実測。**読むのは N どうしの差**で、
予算行(操作応答 100ms 等)との絶対値は [perf-windows-x64.md](perf-windows-x64.md) §判定 が正。

## 条件

- 対象: `cargo xtask corpus` の合成リポジトリに **`cargo xtask corpus --copies 8` で 8 つの
  作業コピーを建てた状態**。コピーは各 109,652 ファイルのチェックアウト + untracked 1 ファイルで、
  枝 `pgg-copy-1..8` に立つ(detached にすると開いた時の listing が walk を頼み、開幕の stream が
  最初の chunk の前に取られて harness が「walk が無い」と refuse する = `session::joins::WorktreeNews`)。
  枝が 8 本増えるので **corpus token は `fb3d090c4261f4500bd61c8db7389485d515feab`(refs 50,012)**
  — perf 記録の token とは別物で、この記録の中でだけ比較する。コピーを `git worktree remove` +
  `git branch -D pgg-copy-<n>` で下ろせば元の token に戻る
- 実行手順(N ごとに 1 行):

  ```
  PGG_ALLOW_GUI=1 cargo xtask perf --repo <corpus> --runs 3 --no-font-walk --allow-noisy --cases <cases.tsv> --cycles 12 --setting git_concurrency=<N> --setting copies_interval_secs=5 --label slots-<N>
  ```

  `cases.tsv` は `cargo xtask corpus` が印刷する 2 行(`newest` = HEAD `e512482e`、76 ファイル変更・
  開くのは 34,059 バイトの java / `second` = その下の行 `336dd534`)。**24 操作**(2 コミット × 12 周の
  行選択 → details → diff)を実行しながら、**5 秒ごとの巡回**(8 コピーの `status --porcelain=v2 -z
  --branch -uall`)を走らせる = 操作と巡回を重ねるための間隔で、既定(30 秒)ではない。
  1 run ≈ 50 秒、捨て 1 + 3 run。`--setting` / `--log` / `--allow-noisy` / `corpus --copies` は
  この実測で足した口
- 機械: perf 記録と同じ台(Ryzen 9 9900X 12C/24T、DISPLAY2 180Hz、RTX 3070 D3D11、git 2.55、
  Qt 6.10.3)。**静かではない** — `--allow-noisy` で実行した理由は、巡回そのものの kernel /
  Defender 側の代金(`System` が 1 コア・`MsMpEng` がその 1/3)が sampler の「対象以外」に数えられて
  35% の閾を毎回超えるため(job object は子 git を数えるが、その git のファイル走査に付く
  フィルタドライバの時間は数えない)。N=16 の頭には別セッションの `cargo xtask linux test` が
  重なった(perf が待って始めている)
- 読み方: 3 run の **min–中央–max**。details / diff は case ごと 36 操作の **p50 / p95**(ms)。
  巡回 1 周は app.log の `carried pass … elapsed_ms=` を kept 3 run 全部(11〜13 周)から min–中央–max

## K = 1 / 2 / 4 / 8(N = 2 / 4 / 8 / 16)

| N (bg) | 起動→グラフ [ms] | うち walk [ms] | 巡回 1 周(8 コピー)[s] | details newest p50 / p95 | details second p50 / p95 | diff newest p50 / p95 | diff second p50 / p95 | fps / 16.7ms 超 | WS max [MiB] | machine busy / 対象以外 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 (1) | 10.7–13.8 s(読みが成立しない — 下記) | — | 5.0–7.9 | —(最初の diff 6.5 s) | — | — | — | — | — | — |
| 2 (1) | 2239–2485–3434 | 1023–1289–2217 | 3.5–4.1–5.5 | 97 / 155 | 39 / 70 | 189 / 1397 | 173 / 346 | 176.5–177.0 / 1 | 315.8 | 57–58–69% / 26–32–42% |
| 4 (2) | 2295–2771–3484 | 1272–1397–1932 | 2.4–3.6–6.4 | 107 / 883 | 49 / 206 | 147 / 1071 | 99 / 647 | 168.1–177.2 / 1–3–27 | 314.5 | 67–81–82% / 31–43–50% |
| 8 (4) | 2120–2167–2320 | 1002–1023–1084 | 2.4–2.9–3.4 | 73 / 227 | 38 / 171 | 80 / 624 | 74 / 564 | 176.0–177.8 / 1 | 310.3 | 67–69–76% / 28–30–38% |
| 16 (8) | 1794–2006–2258 | 777–963–982 | 2.7–2.9–4.0 | 78 / 316 | 40 / 193 | 102 / 535 | 70 / 460 | 176.2–177.8 / 1–2–3 | 317.7 | 68–75–87% / 25–32–43% |
| 参照: 巡回なし(perf 記録 §判定、5 run) | 1093–1157–1205 | 120–127–147 | — | 78–85–89(max) | — | 102–106–115 | — | 177.6–178.0 / 1–2 | 300.7–303.1(gross) | 12–16% / 8–12% |

N=1 は 4 回実行して 4 回とも harness が読みとして refuse した(捨て run の app.log から: グラフの
初回フレーム 10.7–13.8 s、最初の diff 6.5 s、巡回 5.0–7.9 s)。1 枠を interactive と background が
aging(`OVERTAKEN_LIMIT` = 4)で交互に取り、開幕の stream が rebuild に取られて `first_chunk_ms` が
出ない。設定は 1 を受けるが、使う値ではない。

## 読み

- **巡回の幅は 4 で頭打ち** — 1 周 2.9 s は K=8 でも同じ。1 本の `status -uall` が preload-index で
  約 11 コア幅なので(perf 記録 §計測条件)、枠より先に機械が尽きる
- **操作応答は N=8 が最良か同等**。N=2 / 4 は interactive の枠が 1〜2 本で、行選択の `show` +
  diff の 3 本 + poll の status が並ぶと枠待ちが p95 に出る(N=4 の 883 / 1071 ms)。N=16 は 8 と
  変わらない = 枠を足しても CPU は増えない
- **起動は巡回の CPU に食われる**: 2.0–2.8 s(参照 1.16 s)。開いた瞬間の巡回(`refresh_quick`)が
  walk と同じ機械を取り合う。予算 3 s の内側だが縁 — **開幕の巡回をグラフの初回描画の後へ遅らせる余地**
  (残件 = P3-確認事項 §別 worktree の未コミット行)
- **メモリは N に依らない**(WS 292–318 MiB — git は別プロセスで、アプリの WS には乗らない。
  参照の gross 300–303 と同じ帯で、差は run の揺れ)
- **machine busy は巡回が支配**(5 秒ごとに 8 × 109k ファイルの status)。既定の 30 秒間隔ならこの 1/6 で、
  perf 記録の座り(12–16%)に近づく

## 既定値(この実測で決めた)

- `git_concurrency` = **`process::default_concurrency()` = 機械のスレッド数 / 3 を 2..=8 に収めた数**
  — この台(24T)で 8、background(巡回の幅)はその半分の 4。8 スレッドの laptop は床の 2(background 1)。
  `MAX_CONCURRENCY` = 32
- `copies_interval_secs` = **30**(1 周 2.9 s に対して 10% の duty。5 秒は実測のための重ね方で、既定にしない)
- **`Limits::of(n)` = 全体 n・click の予約 n − max(1, n/2)**(interactive かつ手元ペースだけが取れる)。
  背景の読みと elsewhere(fetch / push / ls-remote / clone / mergetool / 署名検証)は残りの max(1, n/2)
  を分け合う。計測時の実装は Here n / Background max(1, n/2) / Elsewhere を別プール n で持っていたが、
  この計測では elsewhere のコマンドが 1 本も走っていない(corpus にリモート無し・auto fetch 無し)ので、
  巡回の幅も click の枠も今の形と同じ数字になる。認証待ちで枠に座る形は実測でなく設計で切った = 予約の外

## 操作 1 回の内訳 — 枠待ち / プロセス起動 / git の仕事 / 結果反映

巡回を止め(`--setting copies_interval_secs=0`)、既定の N(この台で 8)で `--log debug` を付けて
実行した 2 run(12 操作 × 2)。executor が 1 コマンドごとに残す `waited_ms`(枠待ち)・`spawn_ms`
(`CreateProcess` の呼び出し)・`elapsed_ms`(spawn → reap)の 3 つと、harness の `details data`
(要求 → drain)/ `details frame`(→ フレーム)。別セッションの `cargo xtask shipped` が頭に重なり、
machine は 32–41% busy(対象以外 26–35%)。

```
PGG_ALLOW_GUI=1 cargo xtask perf --repo <corpus> --runs 2 --no-font-walk --allow-noisy --log debug --cases <cases.tsv> --cycles 6 --setting copies_interval_secs=0 --label slots-breakdown
```

| コマンド(操作経路) | 本数 | 枠待ち | 起動(spawn)平均 / max [ms] | 実行(spawn→reap)平均 / max [ms] |
|---|---|---|---|---|
| `show`(details)と `diff-tree`(diff)= `-c` で始まる 2 本 | 48 | 0 | 2.2 / 23 | 46.0 / 89 |
| `rev-parse --verify`(blob の存在確認、diff の直列 1 本目) | 34 | 0 | 2.0 / 6 | 25.6 / 43 |
| `cat-file blob`(色の元テキスト、diff の直列 2 本目) | 48 | 0 | 2.8 / 25 | 25.6 / 45 |
| `check-attr`(改行の裁定) | 24 | 0 | 1.6 / 3 | 42.8 / 83 |
| `status -uall`(poll) | 4 | 0 | 3.5 / 5 | 495.8 / 578 |
| `for-each-ref`(poll) | 10 | 0 | 2.9 / 5 | 345.1 / 713 |
| `log`(walk) | 4 | 0 | 3.5 / 4 | 581.5 / 1082 |

- **枠待ちは 0** — N=8 で巡回が無ければ、操作経路のコマンドは 1 本も並ばない(枠は代金でなく保険)
- **起動(spawn の呼び出し)は 2–3ms**で、負荷の瞬間だけ 20〜90ms に跳ねる(`config` 平均 8.9 / max 91、
  `worktree` 平均 24 / max 89)— 揺れの正体は CreateProcess の側
- **実行 = ほぼプロセスの寿命**: `rev-parse` も `cat-file` も git 自身の仕事は 1ms 級(perf 記録
  §判定「往復はプロセス生成が主」)で、25ms はこの負荷での 1 本の値段。静かな台の 10–11ms
  (code-costs §git のプロセス代)の 2 倍強
- **結果反映**: details は data(要求 → drain)22–43.5–75ms に対し frame(→ フレーム)34–62–116ms で、
  **描く側は 20–40ms**(参照 27–38)。diff の frame は 63–84–140ms
- **diff を開く 1 点の critical path は今も `rev-parse` → `cat-file` の直列 2 本**(diff-tree ∥ check-attr
  と並走する 3 本の最長。perf 記録の P3 の項と同じ)= 負荷時 ~50ms、静かな台で ~20ms

## プロセス再利用 — `cat-file --batch` の常駐

`cargo xtask corpus --probe`(std だけの一発測定。corpus に対して 20 回、min / 中央、max)。
**製品と同じ条件**(`FIXED_ARGS` / `FIXED_ENV` を写している)。

| 読み方 | 1 回あたり [ms] |
|---|---|
| `cat-file blob <HEAD:path>`、プロセス 1 本ずつ | 11 / 17(max 19) |
| `rev-parse --verify -q <spec>`、プロセス 1 本ずつ(存在確認) | 10 / 11(max 19) |
| `show --no-patch --format=%H HEAD`、プロセス 1 本ずつ(最小の `show`) | 37 / 45(max 57) |
| **`cat-file --batch` 常駐 1 本に stdin で 20 回** | **0 / 0(max 13 = 最初の 1 回)** |
| **`cat-file --batch-check` 常駐 1 本に 20 回** | **0 / 0(max 11)** |
| `--batch-check` に無いパス(`HEAD:no/such/path`) | 0 バイト・0ms で「missing」— **終了コードは変わらず、プロセスは生きたまま** |

常駐 1 本の常駐メモリ: 20 読みの後で **360 KB**(`tasklist`。pack の写像は触ったページだけが
乗る)。対応 OS / 最低 git: `--batch` / `--batch-check` は最低版(2.43)にあり、`--batch-command`
(1 本で info と contents を切り替える形)は 2.36 から = 3OS とも最低版の内側。

**結論: 保留(設計は確定、実装は別の変更)**。効くのは diff を開く 1 点の直列 2 本
(`preview::blob_is_there` の `rev-parse --verify` + `cat-file blob`)で、常駐なら **1 回 <1ms ×
2 = 静かな台で −20ms / この負荷で −50ms**、しかも「読む前に `rev-parse --verify` で訊く」規則
(rules-refs/core.md の `answers_by_code` の項 — `cat-file` の 128 は答えにできない)を
`--batch-check` の "missing" 行が置き換える。**採用根拠は操作経路での改善でなければならない**
(この表は往復の単発値)ので、結論は実装して `perf_diff_frame` で撮ってから。設計は
[P3-確認事項](../internal-docs/P3-確認事項.md) §core「`cat-file --batch` の常駐」。`show`(details)は
diff の機械が要るので常駐で置き換わらず、1 本のまま。

## `status` の高速化機構 — untracked cache / fsmonitor

同じ probe。**製品と同じ `GIT_OPTIONAL_LOCKS=0` + `--no-optional-locks`** で、コピー 1(index を
書く変種のため。corpus 本体は読むだけ)に対する `status --porcelain=v2 -z --branch -uall` を 5 回。
機構を入れる書き込み(`update-index --untracked-cache`、初回の status)は錠ありで 1 度だけ行い、
その後は製品条件で測る = 製品が「誰かが入れた機構」を見つけた時の姿。

| 変種 | min / 中央 [ms](max) |
|---|---|
| 製品条件のまま | **367 / 383**(400) |
| untracked cache on(錠ありの status で 1 度満たしてから) | 378 / 601(625) |
| fsmonitor on(初回の status = daemon 起動・錠あり: 648ms)、その token のまま | 611 / 641(705) |
| fsmonitor + untracked cache、どちらも 1 度満たしてから | 401 / 520(725) |

**結論: 保留(採用しない)**。製品条件では index が二度と書かれないので token も cache も更新されず、
fsmonitor は「最初の token からの差分」を毎回 daemon に訊く分だけ遅くなり、untracked cache は
中央値で悪化(min は同じ)。効かせるには status のたびに index を書く = 他の作業コピーの
`index.lock` を握る側へ戻ることになり、それは巡回が他所の木を読める前提(rules-refs/core.md の
`--no-optional-locks` の項)を崩す。加えて `core.fsmonitor` / `core.untrackedCache` は利用者の
リポジトリ設定と index への書き込みなので、入れるなら明示の設定操作でなければならず、
その代金に見合う数字が無い。daemon は probe が自分で止めている(`fsmonitor--daemon stop`)。

## 再実行

コピーが立っていれば §条件 の 1 行だけ。立っていなければ `cargo xtask corpus --copies 8`(8 × 109,652
ファイルのチェックアウトで数分)。`--allow-noisy` は外せない(理由は §条件)。内訳は §操作 1 回の内訳
の 1 行、probe は `cargo xtask corpus --probe`(コピー 1 が要る)。
