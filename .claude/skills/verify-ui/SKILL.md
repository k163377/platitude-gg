---
name: verify-ui
description: platitude-gg の UI 動作確認・スクリーンショット検証・アプリ起動(「rebase して起動」含む)の前に必ず読む。verify-ui の使い方・PG_AUTO_ACT 動詞表(verbs.md を Grep)・Windows / Linux の検証の罠は全部ここ。
---

# UI 動作確認(ヘッドレス検証)

**ヘッドレス動確は `cargo xtask verify-ui <動詞> [引数]`** — release ビルド → 使い捨て demo リポジトリ生成 → offscreen 起動 → `PG_AUTO_ACT` の因果的完了 → `screenshot saved=true` 判定と PNG 保存まで 1 コマンド。`--no-build` で連続実行、`--preset` / `--repo` で対象指定、素材だけ欲しければ `cargo xtask demo-repo <preset>`。**UI 配線の Done は、これと `cargo xtask linux verify-ui` が同じ動詞で PASS し、両方の PNG を目視するまで**(CLAUDE.md ビルド・テスト。Linux 側の差分は §Linux での動確)。

presets / options の一覧は `cargo xtask` の USAGE(引数なし実行)が正。

**目視は等倍以上で、判定する所を切り出して見る。** 何枚も並べて縮小した一覧は「どの絵が撮れたか」の確認用で、**合否には使わない** — 縮小の補間は**上に載った数 px を消す**(2026-08-21: 箱の上に仕切りバー 5px が描かれて `?` が半分に切れていたのを、縮めた一覧で「通っている」と読み違えてユーザー報告に至った)。文字・枠・重なりを判定する時は、その行を 3 倍以上・**NEAREST**(補間しない)で切り出す。

**ユーザーに見せる絵は board へ出す — チャットに貼らない。** チャットのペインは画像を幅に合わせて縮めるうえ拡大できないので、1440x900 の全景は等倍で読めず、**大きく撮っても表示は変わらない**(効くのは視野を狭めるか、拡大できるビューアで開かせるかだけ)。`verify-ui` は撮った PNG を自動で board に載せる(`.shots/` — 本体 checkout の `.git` の隣、gitignore 済み、6 席で 1 枚)。見せる時は **`cargo xtask shots open`** で別ウィンドウが開く: ホイールで拡大・ドラッグで移動・`0` で全体・`1/2/4/8` で**正確な整数倍**、`image-rendering: pixelated` なので**補間しない**(OS の画像ビューアは補間するので 1px の判定に使えない)。閉じても同じページを開き直せる。**どの席が撮ったかは board が cwd から読んで自分で記録する** — 言い忘れが起きない。手で作った切り出し・比較シートは `cargo xtask shots add --label "<この絵で何を見るか>" [--verb <動詞>] <png>...`(**`--label` は必須** — 名前の無い複数枚は、どのファイルがどの変更か言えなくなる)。flaky 掃引のように同じ絵が何十枚も出る run は `verify-ui --no-board` で board に載せない。

**動詞はまとめて撮る(1 動詞 1 ターンにしない)** — 同じビルドで撮れる動詞は 1 個の複合コマンドに並べて 1 ターンで実行し(2 発目以降は `--no-build`)、PNG の目視も 1 ターンに複数枚まとめて読む。段 2 と重なる時は `cargo xtask check --verb '<動詞と引数>' --verb …` の一括(ホスト / コンテナ並列)が最速。1 動詞ずつ「撮る→見る→直す」を回してよいのは、直前の絵が次の編集を決める時だけ。

**判定には書き込みの失敗も入る** — `write failed` が 1 行でもあれば FAIL(撮れた PNG は「届かなかった状態」のもの)。**`--allow-write-failure` を付けてよいのは「その拒否がこの動詞の見せ物である」時だけ**(対象動詞の全列挙は `cargo xtask` の USAGE が正)。引数の渡し忘れも同じ行に出る(`delete-branch-refused` を引数なしで撃つと `git branch --delete -- ''` が拒まれ、`not merged` の絵は撮れていない)。

## 壊れない動詞の実装と反復

- **時間は成功条件にしない**。動詞は実際の入力経路を通し、対象の `loaded` / popup `visible` / tooltip `visible` / busy edge と終了 / write sequence / model output 等、その操作が生む観測可能な出力を待って `finishAutoAct()` する。25ms 等の Timer は状態 sampler であり、回数・経過時間で先へ進めない。`--quit-ms` は廃止済みで指定すると fail fast する
- **`--watchdog-ms` は診断用の外側の天井だけ**。性能が落ちても正しい run の撮影時点を変えないよう通常は既定 120s のまま使う。短くして「通信中」や「起動途中」を狙わない。**watchdog 到達はそれだけで FAIL** — アプリは自分で `Qt.quit()` するので exit 0 で戻り、2 枚のうち app.png だけ書けた run は「screenshot saved=true」も持つ。判定は `auto-act watchdog expired` の行そのもの
- **非因果の寿命管理を因果完了へ混ぜない**。性能測定の 12 秒窓は測定入力なので保持するが、起動からの固定 quit は成功条件にしない。`perf_done` と親 watchdog で perf の終了・kill を判定し、raw worktree app 起動は offscreen 環境変数だけでは許可しない。Windows の直接起動ではなく `cargo xtask verify-ui <verb>`、Linux では `cargo xtask linux verify-ui <verb>` を使う
- **owner は 1 run に 1 つ**。page 内は `AutoActDriver.qml`、window 横断は `WindowAutoActDriver.qml` が完了を持ち、後者の動詞は page completion を defer する。`AutoShotDriver.claimPageAct()` より前に page 動詞を始めず、新規 tab に同じ動詞を replay させない。最終撮影は `AutoShotDriver` だけが行う
- **中間状態は実 edge を latch する**。busy/loading/error を撮る動詞は対応 signal で edge を観測し、非同期画像 callback が終わるまで専用の automation latch で表示を保つ。固定時間の窓を探したり、入力側の bool だけ立てて出力を偽装しない。既存例は `force-push-hold` と `settings-tools-loading`
- **並行反復は build 後に `--no-build`**。既定の repo/config/shot は run ごとに一意かつ atomic claim される。明示した `--repo` / `--config-dir` / `--shot-dir` は同じ path の同時利用を fail fast するので、並行 batch には別 path を渡す。状態往復のように共有が目的の組は同じ path で直列実行する
- flaky 方針の合否を反復で決める時は **最低 10 run**。10 を超えて 1 回でも NG が出たら、修正後に **5 run 連続 OK** を取り直す。並行 batch は各 process を 1 run と数え、全 process の exit / `must_say` / screenshot を個別に判定する

## 起動 fast path(「rebase して起動」等、起動だけの要求)

ユーザーが待っているのは**窓が出ること**で、Done パイプラインではない。前提は「rebase が通り、release exe がビルドできる」ことだけ — **fmt / clippy / test / verify-ui / linux 系を起動の前に置かない**。**シェル呼び出しは 1 個**(状態確認・pid 確認を前にも後ろにも積まない)。

1. **停止 → rebase → ビルド → 起動 → 生存確認まで 1 個の複合コマンド**(ユーザーの指示が前提の操作なので、その指示を記録する escape を両方付ける)。**パイプにもコマンド置換にも通さない**(下記の罠。hook が deny する):

   ```bash
   PG_ALLOW_REBASE=1 git rebase main && PG_ALLOW_GUI=1 cargo xtask launch
   ```

   `launch` が自ツリーの居残りプロセス回収(`cargo xtask kill` 相当)→ release ビルド → 切り離し起動 → 生存確認(約 10ms の無言終了 = Qt bin 不在の検出)まで行い、Qt の PATH も自分で解決する。rebase 不要の要求なら `PG_ALLOW_GUI=1 cargo xtask launch` だけ。冷えたツリーの release ビルドは既定の 2 分を超えうるので、**前景のまま timeout を上げる**(background へ逃がすのは下の 3 で禁止)。**他席やユーザーの窓を殺さない** — 掴まれた exe・二重起動ゲートの原因は常に自ツリーの居残りで、`kill` / `launch` がそれだけを落とす(画像名 kill は hook が deny)。
2. **`launch` の行が最後のツール呼び出し**。窓が出たら報告してターンを終える。起動を待たせてよいのは rebase の衝突と build エラーだけ(衝突を解決したら、Done パイプラインへ寄り道せずこの fast path の続きで起動まで行く)。
3. **起動したら監視しない** — 報告の後ろに何も吊らない。禁止は全部同じ 1 つの理由で、**ターンが終わらない間ユーザーの次の指示はキューに載ったまま届かない**(実測: 起動の 36 秒後に打たれた指示が届いたのは 9 分後):
   - 起動そのものを run_in_background で撃つ(hook が deny)/ 報告と同じターンで `check` 等を run_in_background で始める。背景タスクが残る間セッションの表示は「処理中」のままで、完了通知がエージェントを起こし直す = 打ちっぱなしのはずの起動がトークンを食い続ける
   - `Get-Process` / `xtask seats` / ログの tail で生存や pid を確かめ直す(生存確認は `launch` が済ませている。その報告行より後のことはユーザーの窓の話で、このセッションの持ち物ではない)
   - 窓が閉じるのを待つ・ユーザーの操作を待つ・撃ち直す・スクショを撮る

   **Done の基準は CLAUDE.md ビルド・テスト(段 2)のまま不変** — 走らせるのはユーザーが検証・反映を指示した時で、起動の報告には**段 2 が未実行であることを 1 行添える**(「マージ可」は check の green を見てから言う)。

**`cargo xtask launch` をパイプ・コマンド置換に通すと、ターンが窓の寿命だけ返らない**(実測 2026-08-21)。`| tail -N` / `| Select-Object -Last N` を付けた 32 回は最短 23 秒・中央値 206 秒・最長 603 秒(= ツールのタイムアウト)で返り、パイプを外した回だけ 8.7 秒だった。原因は Windows の handle 継承で、`std::process::Command` は `bInheritHandles=TRUE` で起こすため、**切り離した窓がシェルの作ったパイプの書き込み端を掴んだまま生き続け**、読み手が EOF を見ない。`Stdio::null()` も `DETACHED_PROCESS` も `cmd /c start` も外れない(使い捨ての spawner で 3 つとも孫の寿命そのもの = 19s。外れるのは ShellExecute 経由の `Start-Process` だけ)。ハーネス自身の出力取り込みは継承されないので、**パイプを書かなければ掴まれない** — `launch` の出力は 3 行なので削る必要も無い(`2>&1` だけ・ファイルへの `>` は読み手が居ないので無害)。

## PG_AUTO_ACT 動詞表

**全動詞の正本は [verbs.md](verbs.md)** — 動確の前に、使う動詞の項を必ず読む(引数・preset・報告行の読み方・`must_say` が動詞ごとに違い、引数を省くと何も撮れない動詞がある)。一覧は 1 動詞 1 行 = 動詞名の Grep で該当行だけ引ける。動詞ごとの仕込み(preset・リポジトリの建て方・`--config-dir` の 2 回実行)も全部そこ。**動詞を足したら verbs.md へ追記する**(一覧は 1 行・仕込みは段落)。

**ダイアログ・メニューの見た目は headless で撮れる**: `PG_SHOT_DIR` 指定時、`Main.qml` のオーバーレイミラー(`ShaderEffectSource`)が **overlay.png** を app.png と並べて保存する(offscreen で成立・ロック状態と無関係 — 2026-08-05 実測)。オーバーレイ自体の grabToImage は "no QML engine" で不可、ミラーが唯一の経路。アンロック中の `PrintWindow` も引き続き可(実 hover 等、実ウィンドウが要る検証のみ)。

**overlay.png は撮影時点のオーバーレイそのもの** — ミラーは live ではなく、撮影時に `scheduleUpdate()` を 1 回だけ受け、その完了(`scheduledUpdateCompleted`)を待って grab する。**白紙の overlay.png は「その時点で何も開いていなかった」の意味**で、フレームに追い越された絵ではない(閉じたポップアップの残像も残らない)。報告行 `overlay saved=<bool> popups=<n>` の **`popups=` がオーバーレイ自身が抱えていた数**(メニュー 1 / サブメニューを開けば 2 / モーダルは dimmer を伴って 2 以上)。`commit-menu` / `reset-menu` はこの行が must_say なので、カードの写っていない run は緑にならない — **overlay が主題の動詞には同じ 1 行を足せる**(`verify/verbs.rs`)。「閉じたか」を絵ではなく `Popup.opened` の報告行で読むのは変わらない(白紙は「何も無い」としか言えず、どのカードが閉じたかは言えない)。

## hover の絵の撮り方(仮表示)

hover はアプリへは注入できない(§Windows での実行・デバッグの罠)が、**絵は撮れる** — 入力(ポインタ)だけを迂回し、表示側は実 hover と同じ経路を通す。**「hover は再現できない」で止まってユーザーに実操作を頼まない**。上から順に 3 つの道を検討する:

1. **既にその状態の動詞がある** — [verbs.md](verbs.md) の一覧を見た目の種類から逆引きする。ツールチップ(attached ToolTip): `signature-tip` / `stash-tip` / `path-tip`(いずれも overlay.png 側。**意匠は共有インスタンス 1 つ** = `SharedToolTip.dressToolTip` なので、どれか 1 枚で全ツールチップの見た目が言える)。hover カード: `row-card`(グラフ行)/ `ref-list-card`(チップの一覧)/ `author-card-open` / `co-authors-open` / `badges-hover`(帯の状態カード — `identity-tip` も同じカードを開く)/ `eol-hover`(WIP 行の `!`)/ `eol-commit`(コミットボタンの上のポインタ)/ `avatar-hover`(ペンのバッジ)。ポインタの下で色・印・道具が変わる形: `tab-mark`(タブと `✕`)/ `line-tools` / `hunk-tools`(hunk の照明)/ `stage-many`(行末の `+` と仲間の印)/ `graph-divider`(仕切りの線と禁止の輪)/ `graph-bar`(レーンのバー)/ `nav-peek` 系(レールの peek)/ `avatar-row-lit`(設定一覧の `Remove`)。
2. **配線済みだが動詞が無い** — 動詞を 1 つ足すのが正道で、**安い**: xtask は動詞を検査せず `PG_AUTO_ACT` へ素通しする(許可リストは無い。判定が要る時だけ `verify/verbs.rs` の `must_say` に 1 行)ので、実装は QML の分岐と完了 predicate — ページ内は `AutoActDriver.qml`、ウィンドウ横断(タブ・identity・窓)は `WindowAutoActDriver.qml`。作法は 3 点: **実 hover が書くのと同じ 1 つのプロパティ / シグナルへ書く**(`eol-hover` / `pointAtTab` が手本。書き先が無い形なら先に `*PointedAt` / `pointed` の 1 本を切る — 形の一覧は rules-refs/app-ui.md)・**ページ側の判定を関数直呼びで迂回しない**(`row-card` の注意 = 出さないはずの場面まで開いて緑になる)・**入力した同じ callback 内や固定 Timer で完了せず、出力側 predicate を観測して `finishAutoAct()` する**。足したら verbs.md へ追記する。
3. **未配線・意匠検討の仮当て(強制表示)** — 使い捨てパッチで出す。**worktree で当て、コミットしない**(`repo::open` の TimedOut パッチと同じ扱い)。出したい状態は**仮の bool 1 本に束ねて、見た目の条件へ `|| <その bool>` を足す**(差分が最小で revert しやすく、意匠が採用されたらそのまま道 2 の書き先になる)。QML は release exe 埋め込みなので**パッチのたびにビルドが要る**(同じビルドの撮り直しだけ `--no-build`)。撮影は周囲の状態を作る既存動詞に乗せる — 仮表示は無条件に出るので、どの動詞の PNG にも写る(ツールチップ・ポップアップは overlay.png 側 = 上の項)。**複数の的の棚卸しは一括で強制表示して 1 枚に集める**。往復しそうなら revert の前に `git diff > force-<何>.patch` で保存する。

どの道でも `SetCursorPos` / `WM_MOUSEMOVE` / `SendInput` を試さない(罠の項 — 実マウスに奪還され、成功と失敗が再現不能に混ざる)。実窓でしか見えないのは OS カーソルとの重なりだけ。hover の**配送規則そのもの**(どこに handler を置くと立つか)の検証は使い捨ての qmltestrunner シーン(rules-refs/app-ui.md)。

## Linux(コンテナ)での動確 — Done は両 OS

**`cargo xtask linux verify-ui <動詞> [引数]`** が Ubuntu 側の同じ 1 コマンド。オプションも動詞も **verbs.md の表がそのまま通る**(`--preset` / `--repo` / `--no-build` / `--watchdog-ms` / `--select` …)。フォントスタックも Qt のビルドも別物で、**片方の PASS はもう片方を保証しない**。

- **スクショはホスト側の一時ディレクトリに出る**。パスは実行時に `screenshots and settings: <path>` として印字されるので、そこを読む(コンテナ内の `/out` を見に行かない)。`--shot-dir` を明示した時はそちらが優先され、この橋渡しは行われない
- **offscreen はコンテナでは既定の姿**。Windows のような `QT_QPA_FONTDIR` の指定は要らず、**.ttc の罠も無い**(fontconfig 経由)。イメージが `fonts-noto-cjk` を持つので**日本語はそのまま出る** — デザイン規約が Ubuntu 側に名指ししている `Noto Sans CJK JP` が完全一致で解決することは実測済み
- **2 つの PNG を画素で突き合わせない**。フォントのラスタライズが違うので一致しないのが正常。判定は各 OS で `screenshot saved=true` + 目視
- **worktree から走らせると「not a git repository」が 1 回出る** — worktree の `.git` は Windows の絶対パスを書いたファイルで、コンテナ側の git が辿れないため。**無害**(アプリは渡されたリポジトリを開くだけで、この行が指すのは `/work`)
- **`--repo` にコンテナ内のパスを渡す時は Git Bash の変換を止める** — `--repo /work/…` は MSYS に `C:/Program Files/Git/work/…` へ書き換えられ、アプリは**開けないフォルダの画面**を撮る。そして `screenshot saved=true` は出るので **PASS する**(実測 2026-08-09。`write-failures 0` と、絵が「Not a git repository」であることだけが手掛かり)。`MSYS_NO_PATHCONV=1` を立てるか PowerShell から叩く。**`--preset` だけの run は当たらない**(パスを渡さないため)。届かないリモートが要る動詞(`fetch-fail` / 黄の `push`)を Linux で撮るには**リポジトリを worktree の中に建てる** — `demo-repo <preset> --at <worktree>/<名前>` で作れば origin が `file:///C:/…` を指すので、コンテナからは**そのまま届かないリモート**になる(撮り終えたら消す)
- イメージは自動で選ばれる(verify-ui は Qt を積んだ app ステージ)。初回だけ Qt の取得で時間がかかり、以後はキャッシュ
- **`cargo xtask linux bare` は verify-ui ではない** — 宣言した依存だけを入れた Ubuntu で起動するかを見る別物で、動詞を取らない(CLAUDE.md の段 2)

## Windows での実行・デバッグの罠

- Qt / QML のログ(console.*、QML ロードエラー含む)は既定で OutputDebugString 行き — **`QT_FORCE_STDERR_LOGGING=1` を付けないと stderr に出ず、QML の失敗が無音になる**
- release ビルドは GUI サブシステム(`windows_subsystem`)のため PowerShell から直接起動すると**待機されない**(即座に制御が返り、プロセスが残って exe をロックする)。検証は `Start-Process -PassThru` + `WaitForExit` で行う
- **ログを読みたい起動は `cargo run --release -p platitude-app [--features …]` で撃つ**(memprobe の `mem report` を読む時など、verify-ui に乗らない 1 回きりの計測)。**これは fast path の起動ではない** — cargo が子を待つので窓を閉じるまでターンが返らない。ユーザーの「起動」には使わず、この形を選ぶのは自分でログを読み切って窓を閉じるまでやる時だけ。GUI サブシステムの exe を PowerShell から直に撃つと**出力は自分のターンより後に届いて 1 行も掴めず**、`Start-Process -RedirectStandardError` は**空のファイルを残す**(2026-08-12 実測)。cargo は子を待って stdio をそのまま繋ぐので普通に読める。**Qt の bin を PATH に足すのを忘れない**(無いと約 10ms で無言終了 = 下の項)
- **raw worktree app 起動は親監督なしでは許可しない** — `QT_QPA_PLATFORM=offscreen` だけではプロセス寿命を保証しないため、親 watchdog を持つ verify-ui / linux verify-ui を使う。ログ確認でも直接 exe を放置せず、監督付きの起動経路を選ぶ
- **画面ロック中は通常起動の GUI 検証がハングする**(プロセスは動きログも出るが `grabToImage` callback が返らない — 2026-08-03 ロック実測)。GUI 起動を伴う検証は必ず `WaitForExit(ms)` タイムアウト + 未終了なら `Kill()` のガード付きで実行し、無限待ち・無限ポーリングをしない。`verify-ui` は app-side watchdog + parent kill guard の二重境界を持つ
- **ヘッドレス検証の標準**(ロック状態と無関係に成立、2026-08-03 ロック実測): `QT_QPA_PLATFORM=offscreen` + `QT_QPA_FONTDIR=C:\Windows\Fonts` + 自動化 env(PG_AUTO_OPEN / PG_AUTO_WATCHDOG_MS / PG_SHOT_DIR / PG_AUTO_SELECT 等)。成否は stderr の `screenshot saved=true` と保存 PNG の目視で判定する。**FONTDIR 指定が無いと全文字が豆腐**(offscreen は Windows のシステムフォントを自動検出しない)
- **FONTDIR の .ttc(TrueType Collection)は読み込まれない**(offscreen の FreeType フォント DB は TTC を登録せず、名指しでも豆腐 — 2026-08-08 実測)。Windows 標準の CJK フォント(Yu Gothic / MS Gothic / Meiryo / YaHei / SimSun)は全て .ttc なので、**スクショに日本語が出るのは FONTDIR に .ttf / .otf の CJK フォントが在る時だけ**(この開発機は `NotoSansJP-VF.ttf` が C:\Windows\Fonts に居るため出る)。実ウィンドウの GDI/DirectWrite では TTC は普通に使える — 検証環境だけの罠。**つまりヘッドレスの PNG は実窓と別の字で描かれている** — 字形も metrics も違うので、**行の中の 1px(枠と字の余白・ベースライン・印と語の高さ)をヘッドレスの絵で判定しない**(2026-08-20: チップの上下余白がヘッドレスでは 2/2、実窓では 1/2 だった)。その手の意匠は**実窓を `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)` で撮って画素を数える**のが正本で、ヘッドレスは「両 OS で壊れていないこと」の側を見る。日本語の字形検証は demo `basic` の日本語コミット(`docs: 利用案内の骨子を日本語で直す` — 直 / 骨 が中国語字形だと一目で分かる)を目視する
- fps 計測(PG_AUTO_SCROLL)は offscreen でも完走するが、値は疑似フレームループの上限で表示性能ではない — **性能実測はアンロック状態の通常起動でのみ行う**
- **ポップアップ(Popup / Dialog / Menu)は `grabToImage` に写らない** — ウィンドウのオーバーレイ層に描かれ、掴んだアイテムの部分木の外にいる。撮影は `PG_SHOT_DIR` の overlay.png(上記ミラー)で足りる。実ウィンドウが要る検証は OS 側から `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)` で撮る(GPU 描画のため flags 必須。アンロック中はダイアログも写る — 2026-08-03 設定ダイアログで実測)。キー入力の注入は `SendKeys` が届かない(このシェルはフォアグラウンドを取れず、ユーザーの操作中ウィンドウへ飛ぶ危険もある)。`PostMessage(hwnd, WM_KEYDOWN/UP)` を使う。クリックも `PostMessage(WM_LBUTTONDOWN/UP)` で確実に届くが、**hover は注入で検証不能**(`WM_MOUSEMOVE` 注入・`SetCursorPos` とも実マウスの動きに hover 状態を奪還され、成功と失敗が再現不能に混ざる — 2026-08-03 実測)。hover の絵は §hover の絵の撮り方で出す。**ただし「Qt が hover をどう配るか」だけは測れる** — 使い捨ての QML シーンを書いて `qmltestrunner.exe -input tst_*.qml`(offscreen 可・Qt の bin に同梱)の `mouseMove()` で本物の hover を配れるので、**アプリでは注入できない代わりに、依存している配送規則の方を実測してから配線する**。この形で確かめた 1 つ: **親の `HoverHandler` は、子の hoverEnabled な `MouseArea`(や子自身の `HoverHandler`)の上にポインタが居ても hovered のまま**(handlers are passive。`GraphPane.pointerInside` = レーンの横スクロールバーがこれに乗っている)。**フォーカスは要アクティブ化**(非アクティブウィンドウでは `activeFocusItem` が null のまま。PostMessage はアクティブにしないが、フォアグラウンドスレッドへ `AttachThreadInput` してから `SetForegroundWindow` すれば奪えて検証可能 — 2026-08-04 実測)
- exe の**起動**にも Qt の bin ディレクトリが PATH に要る(ビルド時だけではない)。無いと**約 10ms で無言終了**する — ログもエラーダイアログも出ないので死因が判らない。検証スクリプトは PATH 設定込みで書く
