---
name: verify-ui
description: platitude-gg の UI 動作確認・スクリーンショット検証をする時に必ず読む。cargo xtask verify-ui の使い方、PG_AUTO_ACT 動詞の全表(同ディレクトリの verbs.md — 使う動詞の項を Grep で引く)、headless(offscreen)起動と Windows での GUI 検証の罠(フォント・画面ロック・PrintWindow・PostMessage・hover)を全部ここに置く。hover 状態のスクショ(仮表示・強制表示)の標準手順は §hover の絵の撮り方。Done は両 OS — Linux 側は cargo xtask linux verify-ui で、差分は §Linux(コンテナ)での動確。「rebase して起動」等、起動だけの要求の手順も §起動 fast path が正(テストは起動報告の後ろへ)。
---

# UI 動作確認(ヘッドレス検証)

**ヘッドレス動確は `cargo xtask verify-ui <動詞> [引数]`** — release ビルド → 使い捨て demo リポジトリ生成 → offscreen 起動 → `PG_AUTO_ACT` → `screenshot saved=true` 判定と PNG 保存まで 1 コマンド。`--no-build` で連続実行、`--preset` / `--repo` で対象指定、素材だけ欲しければ `cargo xtask demo-repo <preset>`。**UI 配線の Done は、これと `cargo xtask linux verify-ui` が同じ動詞で PASS し、両方の PNG を目視するまで**(CLAUDE.md ビルド・テスト。Linux 側の差分は §Linux での動確)。

presets / options の一覧は `cargo xtask` の USAGE(引数なし実行)が正。

**動詞はまとめて撮る(1 動詞 1 ターンにしない)** — 検証 1 回の実コストはコマンド約 20 秒(実測平均)に加えて**毎回モデルの 1 ターン**(結果の読み取り・思考・応答)が乗り、ターン数がそのまま壁時計とコンテキスト消費になる(実測: 4 日で verify-ui 1,197 回 — セッション内で最多の反復)。同じビルドで撮れる動詞は 1 個の複合コマンドに並べて 1 ターンで実行し(2 発目以降は `--no-build`)、PNG の目視も 1 ターンに複数枚まとめて読む。段 2 と重なる時は `cargo xtask check --verb '<動詞と引数>' --verb …` の一括(ホスト / コンテナ並列)が最速。1 動詞ずつ「撮る→見る→直す」を回してよいのは、直前の絵が次の編集を決める時だけ。

**判定には書き込みの失敗も入る** — `write failed` が 1 行でもあれば FAIL(撮れた PNG は「届かなかった状態」のもの)。**拒否を見せるのが目的の動詞だけ `--allow-write-failure` を付ける**(`delete-branch-refused` / `commands-fail` / `commands-clear` / `fetch-fail` / `push-retry` は実測で拒否を出す。`fetch-resume` と、届かないリモートへ撃つ `push` / `publish-new-go` も同じ仕込み)。**付けてよいのは「その拒否がこの動詞の見せ物である」時だけ**(対象動詞の全列挙は `cargo xtask` の USAGE が正) — 引数の渡し忘れも同じ行に出る(`delete-branch-refused` を引数なしで撃つと `git branch -d -- ''` が拒まれ、`not merged` の絵は撮れていない)。

## 起動 fast path(「rebase して起動」等、起動だけの要求)

ユーザーが待っているのは**窓が出ること**で、Done パイプラインではない。前提は「rebase が通り、release exe がビルドできる」ことだけ — **fmt / clippy / test / verify-ui / linux 系を起動の前に置かない**。遅さの主因はビルドではなく手数(2026-08-09 の transcript 実測: 要求→窓 65〜93 秒のうち subprocess は 15〜45 秒。release ビルド単体は 0.4〜17 秒、rebase が Cargo.lock / profile を動かして qtbridge ごと巻き込んでも 26〜46 秒)。状態確認・生存確認を個別ターンに積まず、下の 1 コマンドに畳む。

1. **停止 → rebase → ビルド → 起動 → 生存確認まで 1 個の複合コマンド**(GUI はユーザーの起動指示があるから可 — CLAUDE.md の worktree 起動規約)。PowerShell 5.1 — `&&` は無い。native コマンドを `2>&1` で巻くと成功でも `$?` が false に化けるので巻かない:

   ```powershell
   Get-Process platitude-gg -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "*worktrees\<自分の worktree 名>*" } | ForEach-Object { $_.Kill() }; git rebase main; if ($LASTEXITCODE -eq 0) { cargo build --release -p platitude-app; if ($LASTEXITCODE -eq 0) { $env:PG_ALLOW_GUI = '1'; Start-Process "$PWD\target\release\platitude-gg.exe"; Start-Sleep -Milliseconds 900; $p = Get-Process platitude-gg -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "*worktrees\<自分の worktree 名>*" }; if ($p) { "launched pid=$($p.Id)" } else { "EXITED at once - Qt bin missing from PATH?" } } }
   ```

   qmake が見えないシェルでは先頭に `$env:PATH = "<Qt の bin>;" + $env:PATH` を足す(exe の**起動**にも要る — 無いと約 10ms で無言終了。下記の罠)。
2. 窓が出たら**即報告してターンを終える**。起動を待たせてよいのは rebase の衝突と build エラーだけ(衝突を解決したら、Done パイプラインへ寄り道せずこの fast path の続きで起動まで行く)。
3. 報告と同じターンで `cargo xtask check --verb '<触った動詞>'…` を **run_in_background で開始**し、結果が届いたら追報する。**Done の基準は CLAUDE.md ビルド・テスト(段 2)のまま不変** — 「マージ可」は check の green を確認してから言う。背景のテストビルドと次の release 再ビルドは cargo のロックで直列化されうる — 先に修正指示が来たら背景タスクを止めて修正を優先してよい。

## PG_AUTO_ACT 動詞表

**全動詞の正本は [verbs.md](verbs.md)** — 動確の前に、使う動詞の項を必ず読む(引数・preset・報告行の読み方・`must_say` が動詞ごとに違い、引数を省くと何も撮れない動詞がある)。一覧は 1 動詞 1 行 = 動詞名の Grep で該当行だけ引ける。動詞ごとの仕込み(preset・リポジトリの建て方・`--config-dir` の 2 回実行)と `--quit-ms` の狙い方も全部そこ。**動詞を足したら verbs.md へ追記する**(一覧は 1 行・仕込みは段落)。

**ダイアログ・メニューの見た目は headless で撮れる**: `PG_SHOT_DIR` 指定時、`Main.qml` のオーバーレイミラー(`ShaderEffectSource`)が **overlay.png** を app.png と並べて保存する(offscreen で成立・ロック状態と無関係 — 2026-08-05 実測)。オーバーレイ自体の grabToImage は "no QML engine" で不可、ミラーが唯一の経路。アンロック中の `PrintWindow` も引き続き可(実 hover 等、実ウィンドウが要る検証のみ)。

**overlay.png は閉じたポップアップを写したままにする** — `ShaderEffectSource` はソースアイテムが描くものを失うと**新しいフレームを渡さなくなり、最後のテクスチャが残る**。つまり「メニューが閉じたか」を overlay.png で判定できない(閉じた後の絵は閉じる前と同じ)。**閉じたことを見たいなら QML に言わせる** — `Popup.opened` を `AppBackend.report()` で出し、報告行で読む(2026-08-08 実測: 報告は `open=false`、同じ瞬間の overlay.png にはカードが写っていた)。この状態を絵にする時は **app.png だけを使う**(合成すると消えたはずのカードが甦る)。

## hover の絵の撮り方(仮表示)

hover はアプリへは注入できない(§Windows での実行・デバッグの罠)が、**絵は撮れる** — 入力(ポインタ)だけを迂回し、表示側は実 hover と同じ経路を通す。**「hover は再現できない」で止まってユーザーに実操作を頼まない** — 過去はそのたびにユーザー側が「強制表示なら出来るはず」と促し直すことになっていた。上から順に 3 つの道を検討する:

1. **既にその状態の動詞がある** — [verbs.md](verbs.md) の一覧を見た目の種類から逆引きする。ツールチップ(attached ToolTip): `identity-tip` / `signature-tip` / `stash-tip` / `path-tip`(いずれも overlay.png 側。**意匠は共有インスタンス 1 つ** = `Main.dressToolTip` なので、どれか 1 枚で全ツールチップの見た目が言える)。hover カード: `row-card`(グラフ行)/ `ref-list-card`(チップの一覧)/ `author-card-open` / `co-authors-open` / `eol-hover`(WIP 行の `!`)/ `eol-commit`(コミットボタンの上のポインタ)/ `avatar-hover`(ペンのバッジ)。ポインタの下で色・印・道具が変わる形: `tab-mark`(タブと `✕`)/ `line-tools` / `hunk-tools`(hunk の照明)/ `stage-many`(行末の `+` と仲間の印)/ `graph-divider`(仕切りの線と禁止の輪)/ `graph-bar`(レーンのバー)/ `nav-peek` 系(レールの peek)/ `avatar-row-lit`(設定一覧の行)。
2. **配線済みだが動詞が無い** — 動詞を 1 つ足すのが正道で、**安い**: xtask は動詞を検査せず `PG_AUTO_ACT` へ素通しする(許可リストは無い。判定が要る時だけ verify.rs の `must_say` に 1 行)ので、実装は QML の分岐 1 つ — ページ内は `RepoPage.qml` の `autoActTimer`、ウィンドウ横断(タブ・identity・窓)は `Main.qml`。作法は 2 点だけ: **実 hover が書くのと同じ 1 つのプロパティ / シグナルへ書く**(`eol-hover` / `pointAtTab` が手本。書き先が無い形なら先に `*PointedAt` / `pointed` の 1 本を切る — 形の一覧は rules-refs/app-ui.md)・**ページ側の判定を関数直呼びで迂回しない**(`row-card` の注意 = 出さないはずの場面まで開いて緑になる)。足したら verbs.md へ追記する。
3. **未配線・意匠検討の仮当て(強制表示)** — 使い捨てパッチで出す。**worktree で当て、コミットしない**(`repo::open` の TimedOut パッチと同じ扱い)。出したい状態は**仮の bool 1 本に束ねて、見た目の条件へ `|| <その bool>` を足す**(差分が最小で revert しやすく、意匠が採用されたらそのまま道 2 の書き先になる)。QML は release exe 埋め込みなので**パッチのたびにビルドが要る**(同じビルドの撮り直しだけ `--no-build`)。撮影は周囲の状態を作る既存動詞に乗せる — 仮表示は無条件に出るので、どの動詞の PNG にも写る(ツールチップ・ポップアップは overlay.png 側 = 残像の罠は上の項)。**複数の的の棚卸しは一括で強制表示して 1 枚に集める**(実績: ツールチップ全 35 件の棚卸し・スピナー差し替え・矢印の線)。往復しそうなら revert の前に `git diff > force-<何>.patch` で保存する。

どの道でも `SetCursorPos` / `WM_MOUSEMOVE` / `SendInput` を試さない(罠の項 — 実マウスに奪還され、成功と失敗が再現不能に混ざる)。実窓でしか見えないのは OS カーソルとの重なりだけ。hover の**配送規則そのもの**(どこに handler を置くと立つか)の検証は使い捨ての qmltestrunner シーン(rules-refs/app-ui.md)。

## Linux(コンテナ)での動確 — Done は両 OS

**`cargo xtask linux verify-ui <動詞> [引数]`** が Ubuntu 側の同じ 1 コマンド。オプションも動詞も **verbs.md の表がそのまま通る**(`--preset` / `--repo` / `--no-build` / `--quit-ms` / `--select` …)。**UI 配線の Done は両 OS で同じ動詞が PASS し、両方の PNG を目視するまで**(CLAUDE.md ビルド・テスト)。片方だけでは足りない理由は、フォントスタックも Qt のビルドも別物だから — 実際、この形にした最初の一巡で 5 件のレイアウト破損が出た。

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
- **画面ロック中は通常起動の GUI 検証がハングする**(プロセスは動きログも出るが、`grabToImage` の完了と `PG_AUTO_QUIT_MS` の自動終了が発生しない — 2026-08-03 ロック実測)。GUI 起動を伴う検証は必ず `WaitForExit(ms)` タイムアウト + 未終了なら `Kill()` のガード付きで実行し、無限待ち・無限ポーリングをしない
- **ヘッドレス検証の標準**(ロック状態と無関係に成立、2026-08-03 ロック実測): `QT_QPA_PLATFORM=offscreen` + `QT_QPA_FONTDIR=C:\Windows\Fonts` + 自動化 env(PG_AUTO_OPEN / PG_AUTO_QUIT_MS / PG_SHOT_DIR / PG_AUTO_SELECT 等)。成否は stderr の `screenshot saved=true` と保存 PNG の目視で判定する。**FONTDIR 指定が無いと全文字が豆腐**(offscreen は Windows のシステムフォントを自動検出しない)
- **FONTDIR の .ttc(TrueType Collection)は読み込まれない**(offscreen の FreeType フォント DB は TTC を登録せず、名指しでも豆腐 — 2026-08-08 実測)。Windows 標準の CJK フォント(Yu Gothic / MS Gothic / Meiryo / YaHei / SimSun)は全て .ttc なので、**スクショに日本語が出るのは FONTDIR に .ttf / .otf の CJK フォントが在る時だけ**(この開発機は `NotoSansJP-VF.ttf` が C:\Windows\Fonts に居るため出る)。実ウィンドウの GDI/DirectWrite では TTC は普通に使える — 検証環境だけの罠。日本語の字形検証は demo `basic` の日本語コミット(`docs: 利用案内の骨子を日本語で直す` — 直 / 骨 が中国語字形だと一目で分かる)を目視する
- fps 計測(PG_AUTO_SCROLL)は offscreen でも完走するが、値は疑似フレームループの上限で表示性能ではない — **性能実測はアンロック状態の通常起動でのみ行う**
- **ポップアップ(Popup / Dialog / Menu)は `grabToImage` に写らない** — ウィンドウのオーバーレイ層に描かれ、掴んだアイテムの部分木の外にいる。撮影は `PG_SHOT_DIR` の overlay.png(上記ミラー)で足りる。実ウィンドウが要る検証は OS 側から `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)` で撮る(GPU 描画のため flags 必須。アンロック中はダイアログも写る — 2026-08-03 設定ダイアログで実測)。キー入力の注入は `SendKeys` が届かない(このシェルはフォアグラウンドを取れず、ユーザーの操作中ウィンドウへ飛ぶ危険もある)。`PostMessage(hwnd, WM_KEYDOWN/UP)` を使う。クリックも `PostMessage(WM_LBUTTONDOWN/UP)` で確実に届くが、**hover は注入で検証不能**(`WM_MOUSEMOVE` 注入・`SetCursorPos` とも実マウスの動きに hover 状態を奪還され、成功と失敗が再現不能に混ざる — 2026-08-03 実測)。hover の絵は §hover の絵の撮り方で出す(実窓でしか見えないのは OS カーソルとの重なりだけ)。**ただし「Qt が hover をどう配るか」だけは測れる** — 使い捨ての QML シーンを書いて `qmltestrunner.exe -input tst_*.qml`(offscreen 可・Qt の bin に同梱)の `mouseMove()` で本物の hover を配れるので、**アプリでは注入できない代わりに、依存している配送規則の方を実測してから配線する**。この形で確かめた 1 つ: **親の `HoverHandler` は、子の hoverEnabled な `MouseArea`(や子自身の `HoverHandler`)の上にポインタが居ても hovered のまま**(handlers are passive。`GraphPane.pointerInside` = レーンの横スクロールバーがこれに乗っている)。**フォーカスは要アクティブ化**(非アクティブウィンドウでは `activeFocusItem` が null のまま。PostMessage はアクティブにしないが、フォアグラウンドスレッドへ `AttachThreadInput` してから `SetForegroundWindow` すれば奪えて検証可能 — 2026-08-04 実測)
- exe の**起動**にも Qt の bin ディレクトリが PATH に要る(ビルド時だけではない)。無いと**約 10ms で無言終了**する — ログもエラーダイアログも出ないので死因が判らない。検証スクリプトは PATH 設定込みで書く
