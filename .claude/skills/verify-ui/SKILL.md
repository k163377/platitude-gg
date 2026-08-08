---
name: verify-ui
description: platitude-gg の UI 動作確認・スクリーンショット検証をする時に必ず読む。cargo xtask verify-ui の使い方、PG_AUTO_ACT 動詞の全表、headless(offscreen)起動と Windows での GUI 検証の罠(フォント・画面ロック・PrintWindow・PostMessage・hover)を全部ここに置く。
---

# UI 動作確認(ヘッドレス検証)

**ヘッドレス動確は `cargo xtask verify-ui <動詞> [引数]`** — release ビルド → 使い捨て demo リポジトリ生成 → offscreen 起動 → `PG_AUTO_ACT` → `screenshot saved=true` 判定と PNG 保存まで 1 コマンド。`--no-build` で連続実行、`--preset` / `--repo` で対象指定、素材だけ欲しければ `cargo xtask demo-repo <preset>`。**UI 配線の Done はこれが PASS し PNG を目視するまで**(CLAUDE.md ビルド・テスト)。

presets / options の一覧は `cargo xtask` の USAGE(引数なし実行)が正。

## PG_AUTO_ACT 動詞表

書き込み操作の headless 検証は **`PG_AUTO_ACT` = 動詞 / `PG_AUTO_ACT_ARG`**(commit / amend / amend-reset-author / stash / stash-staged / stash-file / switch / switch-remote / squash / reword / edit-message / edit-message-leave / edit-message-discard / cherry-pick / stage-hunk / stage-line / push / force-push / force-push-hold / push-retry(**ボタンが黄 + `!` になる形は push を届かないリモートへ撃つ** — `fetch-fail` と同じ仕込み。push-retry は `--preset diverged` で実際の非早送り拒否を受けてから force で着地させ、報告行 `push_retry refused= branch=` が印の立った側を言う。**印が消えた側は最後の絵で見る**)/ reset-soft / reset-mixed / reset-hard / reset-hard-confirm(hard はサブメニューの長押し行 — -confirm はメニューと行を出した所で止まり、reset-hard が長押しを完走させて実行)/ commit-menu / reset-menu / fetch / settings / open-picker(**ダイアログはプラットフォームの窓なのでどちらの PNG にも写らない** — 判定は報告行 `picker folder=` = 開いているリポジトリの親フォルダの `file:` URL)/ preview / preview-unstaged / preview-staged / dbl-local / dbl-remote / move-branch / name-branch / commands / commands-fail / fetch-fail(引数は連続で失敗させる回数 — 1〜2 で警告の形、3 で auto fetch が止まった形。**リモートが届かないリポジトリが要る**: demo repo の origin を存在しないパスへ向けておく)/ fetch-resume(3 回失敗させてから、止まったボタンの長押しを完走させる)/ nav-dbl(引数 `<section>:<name>`)/ nav-fold(引数 `no-tags` でグラフからタグを外した状態にしてから畳む = レールの旗の消えた側が撮れる)/ nav-peek(引数はセクション名 `branch` / `remote` / `worktree` / `stash` / `tag`)/ nav-peek-rename / nav-unfold(左メニューを畳む / 畳んだレールの 1 セクションを開く / 畳んで戻す。**hover は注入できないのでセクションを名指しする**。開いたセクションはポップアップなので overlay.png に写る。報告行は `nav_rail collapsed= width= peek= editing= diff=`)/ diff-fold / diff-unfold / diff-fold-by-hand / diff-fold-by-rename / diff-keep-folded(引数はパス。diff が左メニューを畳む / `✕` で閉じて戻す / **一覧を戻すと diff が閉じる**のを帯のブロックと改名の入力欄の 2 経路で / 先に手で畳んであれば畳んだまま。同じ `nav_rail` 行の `collapsed=` と `diff=` で判定する)/ rename-branch / rename-tag / rename-stash / branch-at-tag(いずれも引数は新しい名前)/ delete-branch(メニューを開いたまま `-d`。マージ済みなら消えてメニューが閉じ、拒まれたら行が `Delete anyway` に化ける)/ delete-branch-refused(その化けた行を出したまま止める)/ delete-branch-go(化けた行の長押しまで走らせて `-D`)/ delete-tag / delete-stash / delete-remote / delete-stash-row(**質問は無く**メニューが開いたまま止まり、`-go`(delete-stash-row は引数 `go`)が行の長押しを走らせる)/ stash-apply-row / stash-pop-row(グラフ行の Apply / Pop)/ delete-force / delete-tag-go / delete-stash-go / delete-remote-go / discard-file / delete-file / discard-staged(引数はパス。同じ 1 行を unstaged / 未追跡 / staged の行から入る。メニューを出したまま止まり、行の文言を報告する)/ discard-file-go / delete-file-go / discard-staged-go(行の長押しを走らせる)/ diff-file / line-tools / hunk-tools / pick-lines / stage-lines / keep-place(引数はパス。diff を開く / 行がポインタの下で出す `+` を出す / **1 つ目の hunk 見出しの 2 語をポインタの下の色に戻し、その hunk 全体を照らす** / **1 つ目の hunk の変更行を 2 本選ぶ**(`picked_lines <件数>` を報告。見出しが `Stage 2 lines` になる)/ 選んでから書き込む / **contentY 400 まで読み進めてから partial stage し、戻った位置を `diff_place` で報告する**(20 hunk 級の diff が要る = 300 行超のファイルに 20 か所の変更)。hover は注入できないので行を名指しする。休息時の語は `textSecondary` なので、色の付いた形はこれらの動詞でしか撮れない)/ discard-hunk(hunk 見出しの長押しボタンを出したまま止まる)/ discard-hunk-go(そのボタンを完走させる)/ discard-many / discard-many-go(先頭行 + 引数のパスを Ctrl クリックで選んでから同じ 1 行)/ wip(作業ツリーの一覧)/ op-exit(止まった操作の出口カード。`--preset rebase-conflict` / `rebase-staged` / `rebase-empty` / `cherry-pick-conflict` / `conflict` で全状態が撮れる)/ op-exit-go(引数は `--skip` 等のフラグ。その行の長押しを完走させる)/ take-side-ours / take-side-theirs(引数はパス。conflict 行のメニューから片側を採る)/ open-mergetool(引数はパス。同じメニューから外部ツールへ渡し、`merge_tool <名前>` を報告する。**待ちの表示を撮るには、閉じないツールを設定したリポジトリが要る** — demo repo の `.git/config` に `merge.guitool = <名前>` + `[mergetool "<名前>"] cmd = sleep 30` / `trustExitCode = true` を書いて `--repo` で渡す。未設定なら行が `Choose merge editor…` になり何も起動しない)/ merge-branch / rebase-onto(引数はブランチ名)/ revert-commit(引数はリビジョン)/ integrate-menu(引数はブランチ名。ref メニューを立てたまま止める)/ drop-commit / drop-commit-go(引数は**完全な oid**、省略で HEAD のもの。`plan_edit` は symbolic name を取らない。-go が長押しを完走させる)/ publish / publish-taken / publish-remotes / publish-add / publish-go / publish-new-go(**`--preset unpublished`**。upstream の無いブランチで `push` を押した所で止まる / 名前を `taken`(引数で上書き可)にして長押しの形にする / 行き先のリストを開く(overlay.png)/ `Add a remote…` のダイアログを出す(引数 `<name>|<url>`。overlay.png)/ 答える / ダイアログを確定してから答える。報告行は `publish state= remote= branch=` と `publish answering taken= unsure= answerable=`。**届かないリモートの絵は `publish-new-go` に `file:///nowhere/…` を渡し `--quit-ms 3600`**(ダイアログ確定 1800ms → 答え 3000ms の間))/ rename-remote(引数 `<remote>/<old>:<new>`。REMOTES の行は畳まれているので ref を名指しする。質問の手前で止まる)/ rename-remote-go(長押しを最後まで進めて実行)/ rename-local-upstream(引数は新しい名前。手元の改名 → 続く質問まで)/ delete-remote / delete-remote-go(引数 `<remote>/<branch>`。同じく ref を名指しする。**質問は無く**畳みが開いてメニューが立ったまま止まり、-go が行の長押しを完走させる))。クリックと同じ経路を通る。

**開いたまま止める撮影用**は settings-tools(設定ダイアログを開いて Merge editor の候補一覧を開いたまま止める。`merge_editor wanted= shown= configured=` を報告する。**候補は 2 波で来る** — 自前定義は即・同梱分は `--tool-help` が Windows で約 8 秒なので、**リングが回っている絵は既定の `--quit-ms` を 3000 程度に詰めて**、**出揃った絵は 14000 に伸ばして**撮る。demo repo の `.git/config` に `[mergetool "名前"] cmd = …` を 2 つほど書いておくと 1 波目に中身が入る)/ stash-dialog(引数 staged-only でそのチェックをクリック済みの状態)/ file-menu / file-menu-untracked / file-menu-staged / file-menu-conflict(引数はパス。バケツごとに出る行が変わるので 4 つ。conflict は種別の文言を `conflict_kind` として報告する — hover は注入できないため。`--preset conflict-kinds` に 4 種が揃っている)/ amend-author / settings / commit-menu / reset-menu / stash-menu / name-box(引数は行番号)/ ref-list(引数は行番号)/ fetch-ref-list(引数は行番号。fetch を撃ってからその行のチップを展開する — タグの雲は fetch 後にしか出ない。`--preset tags` に 4 状態が揃っている)/ signature(引数は行番号。その行を選び、gpg / ssh-keygen が返すまで待って印を報告する。`--preset signed` に verified / signed / 無署名の 3 行がある)/ nav-rename / rename-remote-box(引数 `<remote>/<old>:<入れておく名前>`。畳みを開いて入力欄を出す — 既にリモートに在る名前を渡せば拒否された枠が撮れる)

**通信中(リング)は `--quit-ms` で狙う**: 撮影は `quit-ms - 800`、動詞は 1200ms の
固定タイマで走る(`Main.qml` の `shotTimer` / `RepoPage.qml` の `autoActTimer`。どちらも
ほぼ同じ瞬間から数え始める)ので、**その差が撮影窓**。既定の 10000 では通信はとうに
終わっている。リングは `PG_SHOT_DIR` があると止まる(`ActionButton.still`)ので毎回
同じ絵になる。実測で当たった値(2026-08-07):

| 撮りたい状態 | 動詞 | `--quit-ms` |
|---|---|---|
| fetch 通信中(通常) | `fetch` | 2100 |
| fetch 通信中(1〜2 回失敗の黄) | `fetch-resume` | 2450 |
| fetch 通信中(停止後の赤) | `fetch-fail 30` | 2600 |
| push 通信中 | `push` | 2100 |
| force push 通信中 | `force-push-hold` | 3400 |

`file://` の fetch は 50〜150ms しか走らないので、**窓は数十 ms**。外したら 10〜20ms
刻みで振る(**当たり外れは PNG を見るまで判らない** — stderr の `screenshot saved` の
時刻は grab のコールバックが走った時刻で、掴んだ瞬間より後)。**黄は `fetch-fail 2` では
撮れない**(失敗が記録されてから次の fetch が busy になるまでの間に必ず 1 フレーム
入る)。`fetch-resume` なら 3 回失敗させた後の resume の fetch が「失敗数 > 0 かつ
停止解除済み」= 黄のまま走るので、そこを狙う。**赤は逆に何回でも撃てる**(3 回で停止
した後も `fetch-fail N` は N まで走り続け、その間ずっと赤 + busy)。

**ヘッドレスで色を確かめる時はデモリモートの URL を疑う** — 届かないリモートを使う
検証(`fetch-fail` / `fetch-resume` / 黄の `push`)で通信が成功してしまうと失敗の記録が
消え、黄も赤も出ない。実験で `remote set-url` を触ったら戻すこと。**URL は demo repo の
`.git/config` を直接書き換えるのが早い**(`git -C` が通らない worktree セッションでも
届く)。`--preset diverged` は `push -f` の形をそのまま出すので、**枠のある状態と
`!` の同居**はこれで撮る。

**ダイアログ・メニューの見た目は headless で撮れる**: `PG_SHOT_DIR` 指定時、`Main.qml` のオーバーレイミラー(`ShaderEffectSource`)が **overlay.png** を app.png と並べて保存する(offscreen で成立・ロック状態と無関係 — 2026-08-05 実測)。オーバーレイ自体の grabToImage は "no QML engine" で不可、ミラーが唯一の経路。アンロック中の `PrintWindow` も引き続き可(実 hover 等、実ウィンドウが要る検証のみ)。

## Windows での実行・デバッグの罠

- Qt / QML のログ(console.*、QML ロードエラー含む)は既定で OutputDebugString 行き — **`QT_FORCE_STDERR_LOGGING=1` を付けないと stderr に出ず、QML の失敗が無音になる**
- release ビルドは GUI サブシステム(`windows_subsystem`)のため PowerShell から直接起動すると**待機されない**(即座に制御が返り、プロセスが残って exe をロックする)。検証は `Start-Process -PassThru` + `WaitForExit` で行う
- **画面ロック中は通常起動の GUI 検証がハングする**(プロセスは動きログも出るが、`grabToImage` の完了と `PG_AUTO_QUIT_MS` の自動終了が発生しない — 2026-08-03 ロック実測)。GUI 起動を伴う検証は必ず `WaitForExit(ms)` タイムアウト + 未終了なら `Kill()` のガード付きで実行し、無限待ち・無限ポーリングをしない
- **ヘッドレス検証の標準**(ロック状態と無関係に成立、2026-08-03 ロック実測): `QT_QPA_PLATFORM=offscreen` + `QT_QPA_FONTDIR=C:\Windows\Fonts` + 自動化 env(PG_AUTO_OPEN / PG_AUTO_QUIT_MS / PG_SHOT_DIR / PG_AUTO_SELECT 等)。成否は stderr の `screenshot saved=true` と保存 PNG の目視で判定する。**FONTDIR 指定が無いと全文字が豆腐**(offscreen は Windows のシステムフォントを自動検出しない)
- **FONTDIR の .ttc(TrueType Collection)は読み込まれない**(offscreen の FreeType フォント DB は TTC を登録せず、名指しでも豆腐 — 2026-08-08 実測)。Windows 標準の CJK フォント(Yu Gothic / MS Gothic / Meiryo / YaHei / SimSun)は全て .ttc なので、**スクショに日本語が出るのは FONTDIR に .ttf / .otf の CJK フォントが在る時だけ**(この開発機は `NotoSansJP-VF.ttf` が C:\Windows\Fonts に居るため出る)。実ウィンドウの GDI/DirectWrite では TTC は普通に使える — 検証環境だけの罠。日本語の字形検証は demo `basic` の日本語コミット(`docs: 利用案内の骨子を日本語で直す` — 直 / 骨 が中国語字形だと一目で分かる)を目視する
- fps 計測(PG_AUTO_SCROLL)は offscreen でも完走するが、値は疑似フレームループの上限で表示性能ではない — **性能実測はアンロック状態の通常起動でのみ行う**
- **ポップアップ(Popup / Dialog / Menu)は `grabToImage` に写らない** — ウィンドウのオーバーレイ層に描かれ、掴んだアイテムの部分木の外にいる。撮影は `PG_SHOT_DIR` の overlay.png(上記ミラー)で足りる。実ウィンドウが要る検証は OS 側から `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)` で撮る(GPU 描画のため flags 必須。アンロック中はダイアログも写る — 2026-08-03 設定ダイアログで実測)。キー入力の注入は `SendKeys` が届かない(このシェルはフォアグラウンドを取れず、ユーザーの操作中ウィンドウへ飛ぶ危険もある)。`PostMessage(hwnd, WM_KEYDOWN/UP)` を使う。クリックも `PostMessage(WM_LBUTTONDOWN/UP)` で確実に届くが、**hover は注入で検証不能**(`WM_MOUSEMOVE` 注入・`SetCursorPos` とも実マウスの動きに hover 状態を奪還され、成功と失敗が再現不能に混ざる — 2026-08-03 実測)。hover の見た目は実操作で確認する。**フォーカスは要アクティブ化**(非アクティブウィンドウでは `activeFocusItem` が null のまま。PostMessage はアクティブにしないが、フォアグラウンドスレッドへ `AttachThreadInput` してから `SetForegroundWindow` すれば奪えて検証可能 — 2026-08-04 実測)
- exe の**起動**にも Qt の bin ディレクトリが PATH に要る(ビルド時だけではない)。無いと**約 10ms で無言終了**する — ログもエラーダイアログも出ないので死因が判らない。検証スクリプトは PATH 設定込みで書く
