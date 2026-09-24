# gate の負荷の内訳(Windows x64)

gate が何に時間を払っているかの実測。**出典は gate 自身の記録**(`target/gate-runs/<epoch>-<pid>.txt` と
隣の `.units.tsv` = ステップごとの ledger。動詞の行の `spent` 列が run の内訳)と、この機械での使い捨ての A/B。

- 台: [code-costs-windows-x64.md](code-costs-windows-x64.md) と同じ(Ryzen 9 9900X 12C/24T / 32GB / Windows 11
  build 26200 / git 2.55.0 / Qt 6.10.3)。**他の仕事が載る台** —— 常に Chrome・IntelliJ・10 本を超える
  Claude のセッション・Docker Desktop が同居している
- **全席の窓**: 2026-09-20〜09-23 の席 a〜f の「重い gate」= 実行したステップが 50 本を超える run(171 本。
  他の gate と重なった run を含む)
- **単独の窓**: 2026-09-24 の席 a、他の gate が居ない機械(WSL `memory=6GB`)で PASS した
  `gate --fresh --keep-going` 3 本(1235 ステップ、動詞は census の 608 行 × 2 側を全部)

## 仕事の 94% は動詞

全席の窓のうち 2026-09-22〜23 の重い gate の、ステップの実行時間(ledger の `ran_ms`)の合計:

| 区分 | 合計 | 割合 | 1 本の平均 |
|---|---|---|---|
| host の動詞(`verify …`) | 96.0h | 65.2% | 9.7s |
| Linux の動詞(`verify-linux …`) | 41.8h | 28.4% | 3.9s |
| host の test(`test it` 以外) | 3.7h | 2.5% | 72.5s |
| host の `test it` | 1.8h | 1.2% | 368s |
| 残り(clippy・shipped・qmltest・bare・常時ステップ・Linux の test) | 3.9h | 2.7% | — |

- 単独の窓でも動詞が 94〜96%(host 56〜60%・Linux 36〜38%)で、gate 1 本のステップの実行時間の合計は 1.6〜1.7 時間
- **この窓の gate は動詞を census の全行 × 2 OS で回していた**。製品の QML・app・core のどれを変えても reach に
  `Main` が入り、`Main` は全行の census に居る。終了時 census で絞った場合の数(`shadow candidate`)も、transcript に
  残る gate の大半で全行と同数 —— 1 run の census は窓のほぼ全部品(約 230 種)を持つので、部品で絞っても削れる行が
  無い。**反映前に回る行は段分け表が決める**(§反映前の gate 1 本)
- **census の行は機能と一緒に増える**: 164(09-06)→ 369(09-10)→ 521(09-17)→ 602(09-23)→ 614(09-24)。
  gate 1 本の仕事は変更の大きさではなく、その時点の機能の数に比例する
- 全席の窓の 1 日の重い gate は 34〜65 本、動詞の run は 3.5〜6.8 万、ステップの実行時間の合計は 56〜82 時間

## 動詞 1 run の中身

単独の窓の 3 本(ledger の `spent` 列。launcher = ledger の `ran_ms` − `whole`):

| | fixture | app | launcher | 計(`ran_ms`) |
|---|---|---|---|---|
| host | 0.29–0.32s(中央値 0.2s) | 4.91–5.05s | 0.21–0.32s | 5.63–5.76s |
| Linux | 0.02–0.03s | 2.56–2.82s | 0.61–1.20s | 3.50–3.82s |

- **host の fixture は雛形のコピー**。3s を超える run は 1 本の gate で 4〜6 本 = 雛形を組む run(雛形は 1 日 1 回と
  preset のソースが変わった時に組む = [code-costs](code-costs-windows-x64.md) §テストとハーネス の雛形の行)
- **host の app の中身は git の待ち**: 1 run で git を約 38 本起こす。16 run 同時の中では git の spawn が中央値
  177ms・所要が中央値 367ms(単独では 5〜7ms / 23ms)。その差の大半は `CREATE_NO_WINDOW` が git 1 本ごとに
  立てる `conhost.exe`(code-costs §git のプロセス代 の `CREATE_NO_WINDOW` の行)
- **律速は CPU ではない**: 16 run 同時の 11 秒間(2026-09-23)、CPU 使用率は約 25%、空き物理メモリは 143MB まで落ち、
  ページングは平均 2.5 万ページ/秒、Defender(MsMpEng)は CPU 20 秒を使った
- **この機械の常態**: gate が 1 本も無い時点(2026-09-23)で空き物理メモリ 1.3GB・commit 57GB / 77GB・ページング
  6,300 ページ/秒。MsMpEng の累積 CPU は起動から 4.7 日で 64.7 時間
- Linux の launcher は gate の container への `docker exec` 1 回ぶん

## 壁時計

- **律速は host の動詞**: 単独の窓で host の側は Linux の側より 154〜180s 遅れて終わる
- 他の gate が居ない機械で PASS した `gate` / `gate --fresh`(2026-09-24、席 a、動詞 608 行 × 2 側を全部)14 本:
  **6m31s〜9m01s(中央値 7m24s)**
- 重なった gate は機械を分け合う —— 予算は機械全体で gate 1 本ぶん(`budget::demand`)。雛形の鍵と QML の
  キャッシュが入った後の並走 3 本(2026-09-24 12:11〜12:33、席 c と f が重なり、e はその後に単独): f **5m43s**
  (同居の代金は予算待ちだけ = host 156 本 1m05s / linux 151 本 50s)、c は天井 1 本を除けば **6m11s**(残りの
  全ステップはそこで終わり、`verify-linux identity-tip` の 607s だけが 12m57s まで残った)、e **7m11s**(demo の
  ソースが動いて雛形を組み直した回: fixture の合計 443s、立っていた回は 140〜175s)。host の動詞 1 本は
  **4.2〜4.9s**、Linux 4.1〜5.4s —— 単独の窓と同じ帯(それ以前の並走は host の動詞 1 本が 13〜19s、gate が 15〜25 分)
- **赤い gate の壁時計は「残り全部の終わり + 天井 1 本」**: 動詞の天井は 600s で、天井に当たった run が最後の
  1 本なら、最初の赤で止める機構には止める物が無い。天井の原因は口ごとに読む(P3 §check ハング調査)

## 反映前の gate 1 本

`Main.qml` のコメント 1 行を差分にした `gate --fresh --main <土台>` —— 全行が reach に入る、反映前の gate で一番
重い形(2026-09-24、席 f、他の gate が居ない機械。暖機の 1 本は捨てた):

| 回る物 | 壁時計 | host の動詞 1 本(中央値) |
|---|---|---|
| host の動詞 398 行・コンテナの動詞 25 行(反映前テストの機械化.md §段ごとに何を回すか)・`test gate` 無し | 177s / 181s | 2.73s / 2.74s |

- 動詞の run は 1 本の gate で 423。段 3(`gate --all`)は twin 以外の census の全行(569 行)を両側で回す
- `test gate` は gate 自身の道具に触れた時だけ選ばれる(製品のファイルを読む到達は統合バイナリの手前で止まる ——
  反映前テストの機械化.md `graph::Carried`)。1 本は静かな機械で 34〜36s、重なった gate を含む台帳(09-21〜09-24 の
  重い gate 120 本)では中央値 131s で、走っている間 host の動詞 1 本が中央値で 1.89 倍になる

## Defender

gate 1 本(`gate --fresh`・16 分・全 preset の雛形を組んだ回、2026-09-23)の `New-MpPerformanceRecording`:
走査は計 396 秒・44 万回、うち `%TEMP%` が 280 秒(`pgg-demo` 139 秒 = うち拡張子無し 81 秒・`.sample` 22 秒 /
`pgg-linux` 49 秒 = Linux 側の run の絵と報告を `dllhost.exe` が書き戻す分 / `pgg-tests` 42 秒 / `pgg-verify` 39 秒)、
除外(`IdeaProjects` の下 = 各席の `target\` を含む)の内は 3 万回で 2 秒。**平均 0.4 コアぶんで、CPU が律速でない
gate の壁時計には乗らない**。run の置き場を除外の内へ移しても速くならない(code-costs §テストとハーネス の
置き場の行)

## git の console と QML のキャッシュの代金(A/B)

同じ動詞 8 種 × 4 = 32 run を `--no-build` で幅 8 / 16 の窓に流し、2 つの形を交互に 2 回ずつ(B→A→B→A、
2026-09-23)。**A** = git を隠れ console(`CREATE_NO_WINDOW`)で起こし、QML を起動のたびにコンパイルする /
**B** = git の読みを console 無しで起こし、QML のディスクキャッシュが効く(`platitude-app` の `qrc::embed!`):

| 幅 | A | B | 1 run の平均(A → B) |
|---|---|---|---|
| 8 | 18.5s / 17.7s | 12.2s / 11.6s | 3.89 / 3.60s → 2.38 / 2.21s |
| 16 | 17.9s / 16.0s | 13.3s / 12.8s | 7.62 / 6.72s → 5.35 / 5.05s |

- 幅 8(gate の 1 側の動詞の幅)で 32 run が約 −35%、1 run が約 −39%。**その大半は conhost**: git の console だけを
  入れ替えた対(QML キャッシュは両側で有効)でも幅 8 で 16.2s → 11.2 / 10.8s
- QML のキャッシュだけの寄与は、1 run のアプリの CPU 時間で 1.61–2.06s → 1.39–1.67s(直列の 3 対)。verify の run は
  キャッシュを使う(`target/release/qmlcache`)
- **console 無しの git は採らない** —— MSYS の子(mergetool のスクリプト・フック)が自分で console を作り、
  Windows Terminal が既定の機械では見える窓が立つ(rules-refs/core.md の `CREATE_NO_WINDOW` の行)。conhost を
  払わずに済ませる形は P3 の要判断
