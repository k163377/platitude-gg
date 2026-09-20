# gate の linux 側のコンテナを gate 単位で再利用する代金(Windows x64)

`--all --fresh` の gate は linux 側 523 ステップのうち **515 本が task runner 自身の動詞**
(`verify-linux` 514 + `qmltest-linux`)で、いまはその 1 本ごとに `docker run --rm` でコンテナを
作って捨てる([wsl-memory 実測](wsl-memory-windows-x64.md): 1 本の gate で延べ 513–516 コンテナ)。
この記録は「その 515 本を gate 所有の 1 コンテナへ `docker exec` で入れる」方式(B)を、
現行の「ステップごとのコンテナ」(A)と同じ仕事で比べたもの。

計測日: 2026-09-21、席 c。**A = `951b9d64`**(main)/ **B = この記録と同じ枝の実装 commit**
(SHA は §gate の比較 の表)。B の差分は `crates/xtask/src/linux*` と `gate/runner.rs` /
`gate/mod.rs` の起動経路だけで、`platitude-core` / `platitude-app` / `ui` はバイト一致。
テスト対象・回数(1049 run)・並列度(jobs 8)・予算・キャッシュ条件(`--fresh`)・image・volume は
両 arm で同じ。

## 仮説と採否基準(**実装前に書いた**。結果を見て動かさない)

判定するのは 3 つで、**別々に**判定する。コンテナ数が減ったことは、どれの証拠にもならない。

| # | 問い | 指標(読む場所) | 「意味のある改善」の幅 |
|---|---|---|---|
| 1 | 起動・終了の所要時間 | 小経路: 同じ動詞(`verify-ui … --no-build`)を A / B で逐次 10 本・8 並列 16 本、1 本あたりの wall clock の差。gate: `target/gate-runs/<run>.txt` の見出しの wall clock と、`units.tsv` の linux 動詞 `ran_ms` の和 | 小経路で **1 本 −0.3 s 以上**(515 本 ÷ 8 並列 ≒ gate 20 秒に相当)。gate 全体は **A→B・B→A の両順で B が A より短く、その差が同セッションの A–A の幅を超える**時だけ「短縮」。幅の中なら **未判定** |
| 2 | cgroup の作成・破棄に伴うカーネルメモリ | VM の `Percpu` と `SUnreclaim` の **gate 開始 → gate 終了**の増分(`vm.txt`。終了後の窓は `initd` の走査が混ざるので含めない) | A の `Percpu` 増分(既存実測 +25 MB/gate)に対し **B が半分以下、両順で**。差が 10 MB 未満なら分解能の外 = **未判定** |
| 3 | Docker Desktop の処理とホストの低空き | gate + 終了後 1200 s の窓で: ホスト空き最小・2.5 GB 未満の継続秒数(`host.csv`)、`vmmemWSL` の最大と終値、`/initd services` の `read_bytes` 増分(`vm.txt`)、Docker の `init.log` の同じ窓の `fstrim` 行数 | **両順で** ホスト空き最小が **+500 MB 以上**高く、低空きの継続が短く、**かつ機構の証人**(`fstrim` 行数と `initd` の読み取り量)が同じ向きに動いた時。空きだけ動いて証人が動かなければ開始状態の差 = **未判定** |

- **時間だけ改善した場合**は「時間改善・WSL メモリは未改善／未判定」と書き、その利益(≦ 20 秒/gate)が
  gate 所有のコンテナの寿命管理を抱える代金に見合うかで採否を決める
- **B の cgroup `memory.peak`** はコンテナの生存期間全体の値。従来のステップ単位のピークと並べない
- **`read_bytes` の停止を走査の停止と断定しない**(既存記録と同じ)
- 手順に **`wsl --shutdown` / `drop_caches` / Docker 再起動を入れない**。連続 gate の区間は途中でリセットしない
- 対は **A → B** を 1 対、採用に値する差が出た時だけ **B → A** を足す。開始状態やばらつきから分離できなければ
  未判定と書いて止める(full gate を無制限に繰り返さない)

## 開始条件(2026-09-21、席 c、計測前)

- VM は 00:48 に起動した素の状態(`init.log` の `wsl-bootstrap` の起動時刻)。計測前の VM:
  MemFree 15.5 GB / Buffers 158 MB / SReclaimable 49 MB / **SUnreclaim 81 MB / Percpu 8.7 MB** /
  AnonPages 246 MB / PSI `some` 累計 0 / コンテナ 0
- ホスト空き 6643 MB、`vmmemWSL` 1950 MB。`~/.wslconfig` は既存記録と同じ(上限なし)
- 席 d / e / f は別セッションが保持(同時負荷になり得る。run ごとに `cargo xtask budget` の行を添える)
- image: `pgg-linux:app-3306fab788288e20` / `core-f12dad52a4b651e6`。volume: `pgg-linux-target-c`(11 GB、
  release ビルド済み)/ `pgg-linux-demo-c`

## B の方式(比較した実装)

**gate 1 本につきコンテナ 1 つ**(`linux::container`)。並列枠ごとの複数コンテナは、隔離で得るものが無い
(`/tmp/pgg-demo` は元々 volume で全コンテナ共有、`/out` は葉で割れる)のに寿命管理が枠の数だけ増えるので採らない。

- **起動**: 準備ステップ(`linux runner <run> --gate <pid>`)がコピーを据えた後、同じ木の古い gate コンテナ
  (label `pgg.tree`)を **note の pid をその場で読み直して** live のものだけ spare して消し、
  `pgg-linux-gate-<checkout>-<run>` を app image から `docker run --detach --rm --init` で起こす
  (label `pgg.gate=<pid>` / `pgg.run`)。mount は動詞用コンテナと同じ 4 本 + `<temp>/pgg-linux` を丸ごと
  `/out` に + `target/gate-logs` を `/pgg-gate-logs` に。起動は 120 s の天井付き、docker の答えは
  `gate-logs/linux-container.start.txt`
- **各動詞**: `linux --runner <run> --container <name> --step <mark> verify-ui …` → `docker exec`。
  env は `PGG_STILL_UNDER` / `PGG_BUDGET_HELD` / `PGG_IN_CONTAINER` / **`PGG_STEP=<run>-linux-NN`**、
  workdir `/work`、`--shot-dir /out/<葉>`(葉はホストが claim した `shots-<pid>-<ns>-<n>`)。
  ラッパーはコピーの有無を確かめ、`/run/pgg-gate/steps/<mark>` を置いて動詞を回し、終了時に
  **自分の印を `unset` してから** `/proc` を歩き、印を持つ残りを殺して名を印字する
- **停止**: ホスト側の天井で reap した後、gate が `docker exec <c> sh -c <stop>` を 30 s の天井で撃ち、
  印を持つプロセスを殺す → 1 秒後に数え直す → 印のファイルを消す → `took N, left M` を失敗文へ足す。
  **pid でなく印で名指す**ので、隣の動詞には届かない
- **回収**: 側が終わると `docker rm --force` を 60 s の天井で撃ち、`removed in N s` / `already gone` /
  消せなかった理由と次の手を印字する。**gate が外から殺された時**はコンテナ自身の idle loop が、
  動詞の印が 90 分以内に無く 5 分間動詞が入らなければ自分で抜け(`--rm` で消える)、
  理由を `gate-logs/linux-container.log` に書く。**次の gate の準備が残りを消して印字する**
- **変わらないもの**: cargo ステップ(`test-linux` / `clippy-linux`)・`bare`・`offline` は従来の
  `docker run --rm`(image・mount・network が違う)。Linux ホストは `here` のまま。
  全 Linux ステップが stamp で答えられる側は準備もコンテナも起こさない(`a_copy_is_wanted`)

## 小経路の比較(2026-09-21、席 c)

同じコピー(`xtask-1789920973-36364`、B の実装で gate が据えたもの)から同じ動詞
`verify-ui app-menu --no-build` を、A = `linux --runner <copy> …`(動詞ごとに `docker run --rm`)、
B = `linux --runner <copy> --container <手起こしのコンテナ> --step <印> …`(`docker exec`)で撃った。
起動器は `target/debug/xtask.exe` 直(cargo の起動を挟まない)。ホスト側の wall clock は `Measure-Command`、
中の時間は動詞自身の `spent … whole=` 行。B のコンテナは gate と同じ mount・env・`--init` で手で起こし、
主プロセスだけ `sleep 3600`(idle loop の代わり。exec の道には関係ない)。

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
host 側の `linux` は HELD なので、この待ちは無い。差(A 5.1–5.2 s → B 3.1–4.2 s)は両 arm に同じ待ちが
乗った上での差。

**隔離と停止(同じ手起こしコンテナで、実経路)**:

- 10 本の後の中身: `/root` に書かれたファイル 0、`/tmp` は `pgg-verify-locks/*.lock`(claim の残骸、
  liveness で判定される)だけ、`/run/pgg-gate/steps` は空(印は全部外れた)、残るプロセスは
  `docker-init` と主プロセスだけ。**HOME は書かれず、ステップは前のステップの何にも触れていない**
- 赤いステップ: 無い動詞(`verify-ui no-such-verb`)は app の watchdog まで宙吊りになる形の赤で、
  exec の終了状態がそのまま host に返る(`the run in <c> exited 1`、ラッパーの `pgg-probe ran` 行付き)
- **停止は印で当たる — ただし最初の実装は app に届いていなかった**: verify の harness が `PGG_*` を子の
  環境から落とすので app は `PGG_STEP` を持たず、`took 2 process(es), 0 left` の後に `platitude-gg` が
  PID 1 の下で生き残った(実測)。`PGG_STEP` を両側の pass-through(`NOT_AUTOMATION`)へ足した後は
  `took 3 (platitude-gg / sh / xtask), 0 left` で残りなし(実測)
- コンテナ消失: 動詞が走っている最中に `docker rm --force` → host には `exited 137` が返り、ステップは赤。
  以後の exec は `No such container` で赤(黙って `docker run` に落ちる道は無い)
- 次の gate の準備は同じ木の古いコンテナ(label `pgg.tree=c`、`pgg.gate=0`)を見つけて消す
  (§gate の比較 の B の準備ログ)

## gate の比較

`cargo xtask footprint --settle 1200 gate --all --fresh`、席 c、jobs 8、両 arm とも直前に host と container の
release ビルドを 1 動詞(`verify-ui commit`)で暖めてから。A の footprint は main の xtask なので
docker events と timeline を持たない — events は隣で `docker events` を別に収集(gate 開始の約 20 秒後から)、
gate の終了時刻は `target/gate-runs/<run>.txt` の見出し(run 名の秒 = 終了時刻)。生の行は
`target/footprint/<run>/`(`host.csv` / `vm.txt` / `docker-*.txt`)と `target/events-<arm>.txt`。

**窓の条件が既存記録と違う**: この機械の Docker Desktop は **Resource Saver が有効**で、コンテナが
1 つも無いまま約 4〜5 分経つと VM を落とす(`init.log` に `poweroff` 一式、`vmmemWSL` が消える)。
gate 終了後の窓は両 arm ともそこで VM が消えるので、**「終了後 +6〜11 分の低空き」は今日の条件では
起きようがなく、窓の後半はホストの空きが VM 分だけ戻る**。既存記録の窓とは並べない。
`init.log` は gate 中 40〜50 秒で 1 MB ずつ回転し、残るのは末尾の数分だけなので、`fstrim` の回数は
**残った区間の per-minute** でしか比べられない。

### A → B の 1 対

| | **A** = main `951b9d64` | **B** = `462b04b9` |
|---|---|---|
| 開始時刻(ローカル) | 01:42 | 02:13 |
| 開始 ホスト空き / `vmmemWSL` | 2241 / 2255 MB(warm-up 直後) | 3130 / 3406 MB |
| 開始 VM: Percpu / SUnreclaim / Cached | 10 / 113 / 2364 MB(2 本の gate の後) | 8.5 / 81 / 1922 MB(**Resource Saver で再起動した素の VM**) |
| gate | **PASS 8m02s**(sides 7m56s、run 1095) | **PASS 7m24s**(sides 7m18s、run 1095) |
| linux 動詞 537 本の `ran_ms` の和 / host 動詞 | 2889 s / 3465 s | **1827 s**(−37%)/ 3263 s |
| longest | `test it (all)` 3m13s(host)/ `test xtask linux` 1m10s | `test it (all)` 2m31s(host)/ `test xtask linux` 1m03s |
| コンテナ 延べ(events destroy)/ 同時最大 / 残留 | **481** / 9 / 0(footprint の cgroup 数 549) | **11** / 2 / 0(exec 538 本。cgroup 数 12) |
| ホスト空き 最小(時刻)/ 2.5 GB 未満の秒数 | **53 MB**(01:43:41、gate 開始 +75 s)/ 214 s(全部 gate 中) | **170 MB**(02:14:12、gate 開始 +37 s)/ **348 s**(gate 中 324 s) |
| `vmmemWSL` 最大 / gate 終了時 | 5684 / 3375 MB | **6920** / 3935 MB |
| VM gate 開始→終了: **Percpu / SUnreclaim** | **+8 / +19 MB** | **+1 / +28 MB** |
| VM gate 開始→終了: SReclaimable / Buffers / Cached | +88 / +114 / +2654 MB | +104 / +103 / +3117 MB |
| VM AnonPages 最大 / MemAvailable 最小 | 3179 / 11615 MB | 2597 / 12269 MB |
| PSI `some` の増分 / swap / oom | +1.6 ms / 不変 / 0 | 0 / 不変 / 0 |
| `/initd services` 読み取り: gate 中 / 窓(VM が落ちるまで) | 1054 MB(pid 40)/ 934 MB(pid 69) | **1663 MB**(pid 41)/ 1033 MB(pid 62) |
| `fstrim` 行(残った区間) | 35 行 / 16:46:32–16:50:12 の 3m40s ≒ **9.5 /分**、窓は 0 | 9 行 / gate の 7m25s 全部 ≒ **1.2 /分**、窓は 0 |
| VM が落ちた時刻 | gate 終了 +4m08s(01:54:36) | gate 終了 +2m32s(02:23:32) |

**この 1 対で言えること(基準の 3 つを別々に)**:

1. **時間**: linux 動詞の和は **−1062 s(−37%)**で、小経路の 1 本 −0.42 s × 537 本(≒ −225 s)を
   **桁で超える** — gate の中では 8 並列の `docker run` が互いに待つ(小経路の 8 並列で A の起動器の外側は
   ≈ 3.7 s/本)のに対し exec は待たないぶん。**gate 全体は −38 s(8m02s → 7m24s)** — この日の壁時計は
   **host 側**(`test it (all)` 3m13s と host 動詞の和 3465 s ÷ 8 ≒ 7m13s)が決めていて、linux 側は
   先に終わる。**A–A の幅はこの日は測っていない**(既存記録では 44〜76 s)ので、gate 全体の −38 s は
   基準の「幅を超える」を満たさず、**未判定**。linux 側の −37% は起動器の代金がそのまま消えた形で、
   linux 側が壁時計になる機械状態(既存記録: linux 2400〜2700 s > host 1700〜2200 s)では gate 全体に出る
2. **cgroup のカーネルメモリ**: Percpu は **+8 → +1 MB**(向きは仮説どおり)だが A の増分自体が 8 MB で、
   基準に書いた分解能 10 MB の内側 = **未判定**(効果があっても最大 8 MB)。SUnreclaim は **+19 → +28** で
   改善なし。**既存記録の「Percpu +25 MB/gate」は連続 3 本の蓄積で、1 本の素の VM では 8 MB**
3. **ホストの低空きと Docker Desktop**: 機構の証人は動いた(`fstrim` 9.5 → 1.2 /分、コンテナ 481 → 11)。
   **しかし症状は動いていない**: ホスト空きの最小 53 → 170 MB(+117、基準の +500 に届かない)、
   2.5 GB 未満の継続 214 → **348 s(悪化)**、`vmmemWSL` 最大 5684 → **6920(悪化)**、
   `initd` の gate 中の読み取り 1054 → **1663 MB(増)**。**未改善**。低空きの底はどちらも gate 開始
   1 分前後(host 側の runner ビルド + `test it` + clippy が同時に立つ所)で、コンテナの作成・破棄の
   有無で動く場所ではない

### B → A(2 本目の A、順序を逆にした対)

B の直後、同じ手順(warm-up → 素の VM から)で main をもう 1 本。events は開始から収集できたので延べ数はこちらが正
(1 本目の A の 481 は収集の遅れによる過小 = footprint の cgroup 数 549 / 541 と同じ桁)。

| | A(1 本目) | **B** | A(2 本目) |
|---|---|---|---|
| gate | 8m02s | **7m24s** | **7m27s** |
| linux 動詞の和 / host 動詞の和 | 2889 / 3465 s | **1827** / 3263 s | 2619 / 3285 s |
| コンテナ 延べ / 同時最大 | 481(過小)/ 9 | 11 / 2 | **548** / 9 |
| ホスト空き 最小 / 2.5 GB 未満の秒数 | 53 MB / 214 s | 170 MB / 348 s | 190 MB / 334 s |
| `vmmemWSL` 最大 | 5684 | **6920** | 5309 |
| VM gate 中 Percpu / SUnreclaim | +8 / +19 | **+1** / +28 | +9 / +39 |
| VM gate 中 SReclaimable / Buffers / Cached | +88 / +114 / +2654 | +104 / +103 / +3117 | +110 / +183 / +3303 |
| MemAvailable 最小 / AnonPages 最大 | 11615 / 3179 | 12269 / 2597 | 12484 / 2333 |
| `initd` gate 中の読み取り / 窓 | 1054 / 934 MB | 1663 / 1033 MB | 1727 / 1020 MB |
| `fstrim`(残った区間の per-minute) | 9.5 /分 | **1.2 /分** | 5.6 /分(31 行 / 5.5 分) |
| VM が落ちた時刻 | 終了 +4m08s | 終了 +2m32s | 終了 +4m05s |
| 開始 VM の状態 | gate 2 本の後 | 素(再起動後) | 素(再起動後) |

## 実経路の確認(gate と手起こしのコンテナで、2026-09-21)

| 経路 | 確認 |
|---|---|
| 連続・並行ステップの隔離 | 手起こしコンテナで 10 本逐次 + 16 本 8 並列(§小経路): `/root` 書き込み 0、`/tmp` は claim の lock だけ、印は全部外れる、プロセスの残り 0。gate では 537 本 × 4 run(fresh)が全部緑で、settings / gitconfig は葉の下(`/out/shots-…/config`) |
| ステップ失敗 | 無い動詞は app の watchdog までの赤で、`exited 1` と `pgg-probe ran` 行が host に返る |
| タイムアウト(印による停止) | 宙吊りの動詞へ `linux --container … --step … stop`: **`took 3 (platitude-gg / sh / xtask), 0 left`**、印のファイルも消えた。**app に印が届いていなかった版では `took 2, 0 left` の後に app が生き残った**(harness の `PGG_*` 落とし。修正済み) |
| 親(gate)の死 | `gate --fresh` を動詞 8 本が走っている最中に `Stop-Process -Force`(03:14:33)。12 秒後には走っていた動詞は全部終わり(印 0 / app 0 / host 側の子 0)、新しい動詞は入らず、**最後の動詞から 300 s でコンテナが自ら抜けた**(`linux-container.log`: `18:19:34Z leaving: no verb for 300s and none running`)。`--rm` で消えて `docker ps -a` は空。木の gate note は litter として残り、次の gate の `lanes::sole` が片付ける |
| コンテナ消失 | 動詞の最中に `docker rm --force` → host に `exited 137`、以後の exec は `No such container` で赤。フォールバック無し |
| 別 gate / 別席への非干渉 | コンテナ名と label は木ごと(`pgg-linux-gate-<checkout>-<run>`、`pgg.tree`)。準備の掃除は同じ木の label だけを見て、note の pid を読み直して live を spare する(手起こしの `pgg.gate=0` は次の gate の準備が消した: `an earlier gate's container (Up 2 minutes): … removed in 0.4s`) |
| キャッシュのみの gate | `a_copy_is_wanted` が偽なら準備もコンテナも起こさない(単体テスト)。実測: docs だけの commit への素の `gate`(1m56s、run 7 / cached 1087、動詞 0 本)を `docker events` で見ると、起きたコンテナは cargo ステップの 4 本だけで `pgg-linux-gate-*` は 0、exec も 0 |
| ログ・スクショ・設定・失敗時の証拠 | 動詞のログは従来どおり `gate-logs/linux-NN.log`、PNG は `<temp>/pgg-linux/shots-…/`(host から開ける)。コンテナの起動 / 一覧 / 回収 / 停止の docker の答えは `gate-logs/linux-container.{start,list}.txt` / `linux-remove-<コンテナ名>.txt` / `linux-stop-<印>.txt`、コンテナ自身の up / leaving は `gate-logs/linux-container.log` |
| 起動と終了の代金 | 起動 0.3〜0.5 s、回収 0.4〜0.6 s(gate 1 本につき 1 回ずつ) |

## 言えること / 言えないこと

- **実測で言える(時間)**: 起動器の代金は 1 本 **−0.42 s**(逐次、範囲が重ならない)。gate の中では linux 動詞の和が
  **A 2619〜2889 s に対し B 1827 s** — 両順で A の範囲の外(−30〜−37%)。**8 並列の `docker run` は互いに待ち、
  exec は待たない**のがその差(小経路の 8 並列で A の起動器の外側 ≈ 3.7 s/本)
- **言えない(時間)**: **gate 全体の壁時計**。A 8m02s / 7m27s に対し B 7m24s で、A–A の幅(35 s)の中。
  この日の壁時計は host 側(`test it (all)` 2m31〜3m13s + host 動詞の和 3265〜3465 s ÷ 8)が決めていて、
  linux 側の −800〜−1060 s は host 側が終わるのを待つ間に消える。linux 側が壁時計になる機械状態
  (2026-09-19 の記録: linux 2400〜2700 s > host 1700〜2200 s)でなら gate 全体に出るはずだが、**それは測っていない**
- **未判定(cgroup のカーネルメモリ)**: Percpu の gate 中の増分は A +8 / +9 に対し B +1 で向きは仮説どおりだが、
  A の増分そのものが実装前に決めた分解能(10 MB)の内側。SUnreclaim は A +19 / +39 に対し B +28 で範囲の中。
  **既存記録の「Percpu +25 MB/gate」は連続 3 本の蓄積で、素の VM に 1 本なら 8〜9 MB**
- **未改善(ホストの低空きと Docker Desktop)**: 機構の証人(コンテナ 548 → 11、`fstrim` 5.6〜9.5 → 1.2 /分)は
  動いた。**しかし症状は 1 つも動いていない**: ホスト空きの最小(53 / 190 に対し 170)、2.5 GB 未満の継続
  (214 / 334 に対し **348 s**)、`vmmemWSL` の最大(5684 / 5309 に対し **6920**)、`initd` の gate 中の読み取り
  (1054 / 1727 に対し 1663 MB)は全部 A の範囲の中か悪い側。**低空きの底はどちらも gate 開始 40〜75 秒**
  (host 側の runner ビルド・`test it`・clippy とコンテナ側のビルドが同時に立つ所)で、コンテナの作成・破棄が
  無くなっても動かない場所。**`initd` の読み取りはコンテナ数に比例しない**(11 本の gate で 1663 MB)
- **言えない**: 終了後 +6〜11 分の低空き(既存記録)への効果。この機械は Resource Saver が gate 終了 2.5〜4 分後に
  VM を落とすので、その区間は今日の条件では存在しない
- **言えない**: 「cgroup を作らないこと」が Docker Desktop の走査(`/initd services`)を減らすか。読み取り量は
  減っていないので、走査の条件はコンテナの削除ではない(既存記録の「削除ごとに `fstrim` を仕掛け直す」は
  今日も観測できたが、それと走査は別の物)
- **採否の読み**: **WSL メモリの改善を目的とするなら、この方式は効かない**(3 本の実測で 1 つも動かず)。
  効くのは linux 側の時間だけで、その利益が gate の壁時計に出るのは linux 側が壁時計の時だけ。
  管理の複雑化(gate 所有の寿命・印による停止・3 段の回収)は `linux::container` 1 モジュールに閉じ、
  実経路で隔離・赤の伝播・停止・消失・掃除・idle 天井を確かめてある(§小経路の比較、§実経路の確認)
