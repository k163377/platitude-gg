# verify-ui 各論: 動詞を足す・直す・反復する

## 壊れない動詞の実装と反復

- 完了の因果・owner・run の環境は rules/app-ui.md §UI 自動化 が正
- **注入はハンドラ本体そのものへ入れる — 隣に置いた「同じことをする関数」は嘘をつく**。`onReleased` / `onClicked` の中身を名前付き関数へ出し、**ハンドラは 1 行にして run もそこを呼ぶ**。フックが「ハンドラがやるはずのこと」を書き写していると、**ハンドラが別のことをしていても緑になる**。**前提の状態も実経路で作る** — 箱を `page.startRename()` で開けた run は「所作の記憶」を持たないので、出口の判定が全部素通りした
- **1 クリックは press と release の 2 つで、別々の場所が答える**。入力欄が開いている時の押下は **`FocusRelease` の `PointHandler` が press で先に受けて箱を閉じ**、その後で行の `MouseArea.clicked` が来る。run が click だけを注入すると**この順序が再現されず**、実機だけで壊れる。押下で何かが閉じる動詞は `page.releasePressedAway(null)` を先に撃つ
- **`--watchdog-ms` は診断用の外側の天井だけ** — 通常は既定 600s のまま使う(負荷で届く高さに置くと判定が負荷になる)。**watchdog 到達はそれだけで FAIL** — アプリは自分で `Qt.quit()` するので exit 0 で戻り、2 枚のうち app.png だけ書けた run は「screenshot saved=true」も持つ。判定は `auto-act watchdog expired` の行そのもの
- **アプリ側 watchdog が鳴った run**は、その時点で QML のイベントループが応答した。負荷の因果は `identity_tip step=` など対象動詞の到達記録で決める(単発の再実行の緑・終了時の `lanes:` は足りない)。**天井の行の `awaited <動詞> waiting=<条件,…> for Nms` が、揃わなかった条件をそのまま名指す**(名乗らない動詞は `awaited unsaid`)。`frames swapped=N last=…ms ago` は窓が描き続けていたか —— 静止した scene は swap しないので、`last` が古いだけでは描画の停止と言えない。`page missing=<項,…>` はページがまだ待っている物で、`page settled` なら止まったのは動詞固有の条件の側。**`write state … watch=armed id=0` は押下の無い動詞ではどこで止まっても同じ行**。`the stations it reached:` は watchdog 後の正常な後始末も含むため、`exiting` まで並んでも矛盾しない。
- **親の天井で回収された run**は `the stations it reached:` と `it got as far as …` を読む。記録された最後の段階と子の時計での到達時刻であり、親の経過時間との差はその段階の滞在時間ではない。`stations.txt` が無い／空／読めない場合の到達状況は不明。アプリ自身の account があれば同じ子の時計での診断と照合する。`wedge.txt` が無いことから deadline スレッドの到達位置は決められない。スレッド一覧の全文は shot dir の `threads.txt` に残り、報告行はその先頭 8 本。その下の `the main thread stands in:` が main の stack(**Windows のみ**)
- **その読み方が止まった run でも働くことは `cargo xtask wedge-check` が確かめる**(host のみ。gate は記録経路のファイルに触れた時に選ぶ)。`--fault-hang <駅>`・`--fault-no-deadline`・`--fault-hold-act`・`--fault-stall-look` で形を注文する。**形は必ず注文する**: 完了を追い越す天井はこちらの機械の話で、混んだ機械では完了の方が先に着く。**手で 1 つ撃つなら** `cargo xtask verify-ui band --watchdog-ms 60000 --fault-hang exiting --fault-no-deadline --no-board`(trail に `exiting` が出た時点で刈られる = 動詞 1 本ぶんの数秒。**必ず FAIL する run** で、読むのはその下の行)
- **読みが絵に着いた時に元から撮り直すことは `cargo xtask shot-check` が確かめる**(host のみ。gate は絵が通る物 — check の閉包・verify の Rust 半分・製品 — に触れた時に選び、鍵は code の id)。`--fault-owed-at-shot <app|scene>-<before|during|shown>` で 1 枚の読みに項 `fault` を差し込み、その間だけ `mainUi` に印を描く — 6 run とも PASS が条件で、読むのは `picture … attempt=` の行と絵の画素。**手で 1 つ撃つなら** `cargo xtask verify-ui commit-menu --fault-owed-at-shot app-during --no-board`(`attempt=1 held=fault` → `attempt=2 saved=true` と出る)
- **完了が書き込みの記録(GIT COMMANDS の行)を待つことは `cargo xtask records-check` が確かめる**(gate は host で選び、コンテナ側は全体の段。手で撃つなら Linux は `cargo xtask linux records-check`)。`--fault-hold-records` がログの記録を書き込みの最初の asked コマンドから保留し、待ちが記録だけで止まった時に解く — 3 run とも PASS し、`fault_records released=held` が完了より前、`fault_records op= recorded=true rows=` が完了の後に出るのが条件。**手で 1 つ撃つなら** `cargo xtask verify-ui push-tag v1 --preset tag-hidden --allow-write-failure --fault-hold-records --no-board`(`write_settled waiting=page` → `released=held … held=7` → `rows=3`)
- 起動は親監督付きの経路だけ: Windows は <!--cmd:verify.ui-->`cargo xtask verify-ui <verb>`、Linux は <!--cmd:linux.verify-->`cargo xtask linux verify-ui <verb>`
- **ハーネスは出荷ビルドの外の別ビルド**(Cargo feature `automation`)。**`cargo xtask` が起動するビルドは全部この feature 付き**(例外は出荷ビルドそのものを組む `shipped` と `perf --shipped` の 2 つ)だが、**手で `cargo build --release` して撃つと動詞が 1 つも効かない**(窓は普通に出る = 症状は「watchdog まで無言」)。その時は `--features automation` を付ける
- **sampler で完了する動詞は `AutoActCompletion.defersCompletion` に名前を足す** — 足し忘れると**動詞が何かする前の画面が撮られる**。報告行は後から出るので `must_say` は緑になり、**赤いのは絵だけ**。**新しい動詞の 1 発目は必ず PNG を開き、動詞が触ったものが写っているかを見る**
- **中間状態は実 edge を latch する**。busy/loading/error を撮る動詞は対応 signal で edge を観測し、非同期画像 callback が終わるまで専用の automation latch で表示を保つ。latch の根拠は出力側の edge だけ。既存例は `force-push-hold` と `settings-tools-loading`
- **回数押しは終わりが press — `driver.holdToEnd(row)` で走らせる**(`completeHold()` の直呼びは barrier が早すぎる)。**動詞が仕掛けた物は動詞が待ってから完了する** — sampler は `PageStage` / `WindowStage` の op、完了は `op.complete({ when, report })` で、共通の完了経路(`CompletionRoute`)が `when`・`PageSettled.settled(page)`(読みと、見える一覧の delegate)・page の保留を 1 回の読みで確かめてから報告を読む(書き方は下の §操作の書き方、段の移し方・予約名・機械の検査は rules-refs/app-ui.md の「観測・遷移・完了は分ける」)。開いた diff は `DiffPane.diffSettled()`、書いた後の読み直しは `RepoPage.diffSettling`、replay の途中の作業ツリーの行は `AutoActHistoryVerbs.replayRow`。仕掛けた物が届く前に完了した動詞は、絵と観測(`census.txt`)の部品を機械の順で載せたり落としたりする — 完了の後にページへ届く物が主題を変えるなら `PageSettled.settled(page)` を項に足し、絵の主張を `Awaited.claim` で置く(反映前テストの機械化.md §検証時点)。**時計が部品を出し入れする手前では撮らない** — hover の rest や replay の間のページの読みは run が握っていない時計で進み、絵と census の下で部品を出し入れする。その拍を待ってから撮る(replay = 読みを頼んで作業ツリーの行が立つまで待つ)か、拍の前の状態は同じターンで読んで後の段の報告で言う(hover の揺れの直後 = `eol-walk` / `eol-stay` / `card-latch` の `tremor_card=`。揺れの直後で止める段は作らない)
- **並行反復は build 後に `--no-build`**。既定の repo/config/shot は run ごとに一意かつ atomic claim される。明示した `--repo` / `--config-dir` / `--shot-dir` は同じ path の同時利用を fail fast するので、並行 batch には別 path を渡す。状態往復のように共有が目的の組は同じ path で直列実行する
- flaky 方針の合否を反復で決める時は **最低 10 run**。10 を超えて 1 回でも NG が出たら、修正後に **5 run 連続 OK** を取り直す。並行 batch は各 process を 1 run と数え、全 process の exit / `must_say` / screenshot を個別に判定する

## 操作の書き方(文脈・入力・証人・完了)

操作(op)の拍から走るコードは 3 つの文脈のどれかにあり、それぞれ 1 つの責務だけを持つ(`cargo xtask waits` の `reach` が呼び出しを辿って確かめる — 補助関数・別ファイルの関数へ出しても判定は同じ)。

- **beat**(op の `onTriggered` と、そこから呼ぶ関数): 製品を観測し(`Awaited.all`・`op.at`・`op.still` / `laidOutSince`・製品の読む関数)、op の API で段を動かす。**入力は出さない・ハーネスの状態を持たない**(`stage` / `input`)
- **press**(`op.input(段, () => 押す)` の押す関数と、そこから呼ぶ関数): 入力を出し、その効果を待つ op を起動してよい(起動の直前に書く property はその起動の条件)。**段を動かさない・状態を持たない**。効果を待つ要求を撃ち直すなら、押す関数が「効果が見えたか」を返す — false の間は段が動かず、次の拍でまた押す(冪等な物だけ。書き込みは答えで待つ)
- 書き込みは変える値の由来で判定される — そのコードが作った配列・object は組み立ててよく、ハーネスの property の値をローカルに受けて変えるのも、補助関数・引数・返り値越しに変えるのも、自分の配列・object に入れて(自分で・補助関数で・`Object.assign` 等で)中から変えるのも、製品の関数を通して変えるのも、コールバック(その場の arrow・名前で渡す関数)の返り値・引数を通して変えるのも、property を直接変えるのと同じ違反。組み込みの表に無い呼び出しの結果を変えるのは `unread`
- **read**(報告・完了の `when`・`Awaited.claim` の述語): 読むだけ。**読む時点にある物だけを読む** — 書いた場所の変数を持ち込まない(`stale`)。物を指す変数(`const view = diffPane.view`)は可、値を持つ変数(`const n = view.count`)は不可

証拠は役割で 3 つに分ける:

| 役割 | 書き方 | 機構がすること |
|---|---|---|
| 入力前・押下中・入力直後に成り立った事実(`was=` など履歴) | `op.witness("名", 値)`(beat か press で。後から届く答えの縁は届いた時に) / 報告で `op.witnessed("名")`、有無は `op.kept("名")` | 操作・世代・段と結び付けて保存し `witness run=… stage=… name=` を言う。完了の行が報告の読んだ証人を `witnesses=<owner>/<op>/<世代>/<段>/<名>` で名指す |
| 完了時にも要る条件(主題の状態) | `op.complete({ "when": () => ({ "項": 真偽 }), … })` | 拍で読んで真なら完了へ移り、保留の各回と報告を読む時にも読み直す。崩れたら既定は待ち直し(`"lost": "fail"` で breach)。入力は撃ち直さない |
| 完了時の状態(現在値の主張) | `"report": () => [行, …]`(読む関数) | 確定の読み(`when`・page の読み・page の保留)が揃ったターンで読み、同じターンで読み直して変わっていれば捨てて保留を続ける |

- 完了: `op.complete({ "name": "観測名", "when": () => ({ … }), "report": () => …, "arriving": ["項"], "lost": "fail" })`(`name` 既定 `<op>_done`、`arriving` / `lost` は要る時だけ)。拍の最後の `Awaited.all(名, 項)` がそのまま `name` / `when` になる。完了時に要らない前提(押す前の準備・レイアウトの静止)は `when` に入れず拍で読む
- **段で言う行(`advance` / `input` / `say` の報告、`op.note`)は `said run=… stage=…: <行>` として言われ、run の履歴であって主張ではない**。動詞の主張(`must_say` が照合する行)は完了の報告にだけ置く。診断の行は `op.note(行)`(変わった時だけ言う。`Awaited.note` は機構のもの)
- dispatch(各ファミリーの `run(act, arg)`)は `act` 操作の press — 入力と op の起動はここでよい。**dispatch で完了しない**: 報告は `driver.reportAtCompletion(() => …)` で渡し、完了は `act` が dispatch の後で言う。ファミリーの設定(arg から決まる値)は `Harness.autoActArg` から導く `readonly property` に、製品から読んだ値は証人に、op の引数は `op.start()` の直前に書く
- 機構の関数は呼べる文脈を宣言する(`// reach(beat, press; 1=press)`、機構のファイルだけ)。宣言の無い関数は呼び元のコードとして読まれる
