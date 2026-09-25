# verify-ui 各論: 動詞を足す・直す・反復する

## 壊れない動詞の実装と反復

- 完了の因果・owner・run の環境は rules/app-ui.md §UI 自動化 が正
- **注入はハンドラ本体そのものへ入れる — 隣に置いた「同じことをする関数」は嘘をつく**。`onReleased` / `onClicked` の中身を名前付き関数へ出し、**ハンドラは 1 行にして run もそこを呼ぶ**。フックが「ハンドラがやるはずのこと」を書き写していると、**ハンドラが別のことをしていても緑になる**。**前提の状態も実経路で作る** — 箱を `page.startRename()` で開けた run は「所作の記憶」を持たないので、出口の判定が全部素通りした
- **1 クリックは press と release の 2 つで、別々の場所が答える**。入力欄が開いている時の押下は **`FocusRelease` の `PointHandler` が press で先に受けて箱を閉じ**、その後で行の `MouseArea.clicked` が来る。run が click だけを注入すると**この順序が再現されず**、実機だけで壊れる。押下で何かが閉じる動詞は `page.releasePressedAway(null)` を先に撃つ
- **`--watchdog-ms` は診断用の外側の天井だけ** — 通常は既定 600s のまま使う(負荷で届く高さに置くと判定が負荷になる)。**watchdog 到達はそれだけで FAIL** — アプリは自分で `Qt.quit()` するので exit 0 で戻り、2 枚のうち app.png だけ書けた run は「screenshot saved=true」も持つ。判定は `auto-act watchdog expired` の行そのもの
- **アプリ側 watchdog が鳴った run**は、その時点で QML のイベントループが応答した。負荷の因果は `identity_tip step=` など対象動詞の到達記録で決める(単発の再実行の緑・終了時の `lanes:` は足りない)。**天井の行の `awaited <動詞> waiting=<条件,…> for Nms` が、揃わなかった条件をそのまま名指す**(名乗らない動詞は `awaited unsaid`)。`frames swapped=N last=…ms ago` は窓が描き続けていたか —— 静止した scene は swap しないので、`last` が古いだけでは描画の停止と言えない。`page missing=<項,…>` はページがまだ待っている物で、`page settled` なら止まったのは動詞固有の条件の側。**`write state … watch=armed id=0` は押下の無い動詞ではどこで止まっても同じ行**。`the stations it reached:` は watchdog 後の正常な後始末も含むため、`exiting` まで並んでも矛盾しない。
- **親の天井で回収された run**は `the stations it reached:` と `it got as far as …` を読む。記録された最後の段階と子の時計での到達時刻であり、親の経過時間との差はその段階の滞在時間ではない。`stations.txt` が無い／空／読めない場合の到達状況は不明。アプリ自身の account があれば同じ子の時計での診断と照合する。`wedge.txt` が無いことから deadline スレッドの到達位置は決められない。スレッド一覧の全文は shot dir の `threads.txt` に残り、報告行はその先頭 8 本。その下の `the main thread stands in:` が main の stack(**Windows のみ**)
- **その読み方が止まった run でも働くことは `cargo xtask wedge-check` が確かめる**(host のみ。gate は記録経路のファイルに触れた時に選ぶ)。`--fault-hang <駅>`・`--fault-no-deadline`・`--fault-hold-act`・`--fault-stall-look` で形を注文する。**形は必ず注文する**: 完了を追い越す天井はこちらの機械の話で、混んだ機械では完了の方が先に着く。**手で 1 つ撃つなら** `cargo xtask verify-ui band --watchdog-ms 60000 --fault-hang exiting --fault-no-deadline --no-board --no-census`(trail に `exiting` が出た時点で刈られる = 動詞 1 本ぶんの数秒。**必ず FAIL する run** で、読むのはその下の行)
- 起動は親監督付きの経路だけ: Windows は <!--cmd:verify.ui-->`cargo xtask verify-ui <verb>`、Linux は <!--cmd:linux.verify-->`cargo xtask linux verify-ui <verb>`
- **ハーネスは出荷ビルドの外の別ビルド**(Cargo feature `automation`)。**`cargo xtask` が起動するビルドは全部この feature 付き**(例外は出荷ビルドそのものを組む `shipped` と `perf --shipped` の 2 つ)だが、**手で `cargo build --release` して撃つと動詞が 1 つも効かない**(窓は普通に出る = 症状は「watchdog まで無言」)。その時は `--features automation` を付ける
- **sampler で完了する動詞は `AutoActCompletion.defersCompletion` に名前を足す** — 足し忘れると**動詞が何かする前の画面が撮られる**。報告行は後から出るので `must_say` は緑になり、**赤いのは絵だけ**。**新しい動詞の 1 発目は必ず PNG を開き、動詞が触ったものが写っているかを見る**
- **中間状態は実 edge を latch する**。busy/loading/error を撮る動詞は対応 signal で edge を観測し、非同期画像 callback が終わるまで専用の automation latch で表示を保つ。latch の根拠は出力側の edge だけ。既存例は `force-push-hold` と `settings-tools-loading`
- **長押しは終わりが press — `driver.holdToEnd(row)` で走らせる**(`completeHold()` の直呼びは barrier が早すぎる)。**動詞が仕掛けた物は動詞が待ってから完了する** — 開いた diff は `DiffPane.diffSettled()`、書いた後の読み直しは `RepoPage.diffSettling`。仕掛けた物が届く前に完了した動詞は census の行を載せたり落としたりする
- **並行反復は build 後に `--no-build`**。既定の repo/config/shot は run ごとに一意かつ atomic claim される。明示した `--repo` / `--config-dir` / `--shot-dir` は同じ path の同時利用を fail fast するので、並行 batch には別 path を渡す。状態往復のように共有が目的の組は同じ path で直列実行する
- flaky 方針の合否を反復で決める時は **最低 10 run**。10 を超えて 1 回でも NG が出たら、修正後に **5 run 連続 OK** を取り直す。並行 batch は各 process を 1 run と数え、全 process の exit / `must_say` / screenshot を個別に判定する
