# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。
「インストール済み git の CLI を実行するだけの薄い GUI」に徹する。
機能要件・性能要件・スコープ外の正本は [要望.md](internal-docs/要望.md) — 機能実装の前に必ず該当セクションを読むこと。
UI の色・タイポグラフィ・寸法の正本は [デザイン規約.md](internal-docs/デザイン規約.md) — QML を書く前に必ず読み、値は表から選ぶこと(**数値を検討・微調整しない**)。
本ファイルは技術決定と開発規約を定める。

## 絶対制約(変更には人間の明示承認が必要)

- git 操作は**システム git のサブプロセス実行のみ**。libgit2 / gitoxide(gix)等の git 実装ライブラリを導入しない
- ネットワーク通信は**git コマンド経由のみ**。HTTP クライアント・telemetry・forge API(GitHub API 等)のクレートを導入しない
- 認証(ssh / credential helper)・hooks・gitconfig・**署名(gpg / ssh)** は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- AI 機能を実装しない
- ライセンス: アプリ本体は MIT。依存追加は MIT / Apache-2.0 / BSD / Zlib / MPL-2.0 系のみ。**GPL 系依存は禁止**(Qt 本体と qtbridge は LGPL-3.0-only で利用 — 承認済みの例外)
- 対応 git の最低バージョンは [要望.md](internal-docs/要望.md) の定めに従う。それ未満向けのフォールバックコードを書かない
- UI はダークテーマ(青系)のみ。文言は英語のみ・ハードコード禁止(`qsTr()` 必須、将来の i18n に備える)
- **内部コマンドと UI 表記は意図して分ける**。内部は最新 git の適切なコマンドを選ぶ(`switch` / `restore` 等)が、UI 文言は git のコマンド名に引きずられず「その操作が何をするか」を最も適切に表す語を選ぶ。用語の正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表
- UI の値は [デザイン規約.md](internal-docs/デザイン規約.md) のトークンのみ使用。QML への数値・色・フォント名の直書き禁止。トークンの追加・変更には人間の承認が必要

## 技術スタック

| 項目 | 決定 |
|---|---|
| 言語 | Rust stable(最新) |
| UI | Qt Quick (QML)。Qt の必要バージョンは qtbridge の要求に従う |
| ブリッジ | **Qt Bridges**(`qtbridge` クレート)。通常の依存として追加、build.rs 不要 |
| フォールバック | CXX-Qt(スパイク不合格時。QML と core は無変更で移行できる構成を守る) |
| 並行処理 | tokio(公式 `host_monitor` example の構成を踏襲) |
| ログ | `tracing`(`println!` / `eprintln!` 禁止) |
| エラー | core は `thiserror` で型付き、app は `anyhow` 可 |
| スナップショットテスト | `insta` |

## ワークスペース構成(目標)

```
crates/
  platitude-core/   # git 実行・出力パース・ドメインモデル。Qt 依存ゼロ、cargo test で完結
  platitude-app/    # バイナリ。qtbridge ブリッジ + main + qml/
```

ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つためのルール:

- core の API は純 Rust 型(`String` / `Vec` / serde DTO)のみ。`Rc<RefCell>` パターンや qtbridge の型を core に漏らさない
- core → UI の通知は core 定義の trait / チャネルで抽象化し、app 側でブリッジ機構に接続する
- QML にビジネスロジックを書かない(表示とインタラクションのみ。JS でのデータ加工禁止)

## git サブプロセス規約

- コマンドは**引数配列**で組み立てる。シェル文字列の連結禁止
- パースは機械可読形式のみ: `--porcelain=v2` / `-z`(NUL 区切り)/ `--format=` を常用。人間向け・ローカライズされ得る出力をパースしない
- 実行時の環境変数: `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`(プロンプトでハングさせない。認証は credential helper に委譲)、status 等の読み取り系ポーリングは `GIT_OPTIONAL_LOCKS=0`
- **`GIT_LITERAL_PATHSPECS` は使わない** — git 内部の pathspec magic まで無効化し、`git stash push -u` が成功を報告しながら untracked を一切 stash しなくなる(実測)。パスは 1 件ずつ `:(literal)` で包む(`process::literal_pathspec`)。`--no-index` の引数は pathspec ではないので付けない
- 失敗メッセージは stderr 優先・空なら stdout(`git commit` の「nothing to commit」は stdout に出て exit 1 する)
- `git merge --continue` / `git rebase --continue` は**引数を一切受け付けない**(`--no-edit` も不可)。`--no-edit` を渡すのは cherry-pick / revert のみ
- **`git switch --merge` は使わない** — conflict しても exit 0 の「成功」で着地し、`MERGE_HEAD` を作らないので `merge --abort` が効かず(素の switch も「needs merge」で拒まれ、逃げ道は捨てる `switch --force` だけ)、さらに **staged が 1 つでもあると衝突ファイルでなくても拒否**する。未コミット変更を移動先へ運ぶのは **stash → switch → `stash pop --index`**(`session::checkout_merging`)。staged / unstaged の区別が残り、conflict 時は stash が残って戻れる
- **`git stash pop` の非ゼロ終了は「何も起きなかった」を意味しない** — 作業ツリー側の conflict なら**マージ済み**でマーカーを残し stash も残す(`Index was not unstashed`)が、staged 側が衝突すると**丸ごと拒否**して何もしない(`conflicts in index. Try without --index.` → `--index` 無しで再試行する)。判定は exit code ではなく status の unmerged 有無で行う(実測)
- 書き込みは**セッション単位のキューで直列化**する(ロックでは順序が保証されない — spawn したタスクが mutex を取る順は実行順と一致しない)
- `git config <key> <value>` に **`--` セパレータを付けない**(`--` 自体が値として保存される)。ダッシュ始まりの値はそのまま渡して通る
- **identity(`user.name` / `user.email`)に独自バリデーションを足さない** — git が拒むのは**空の name だけ**(空 email は通り author 行が `<>` になる)。`<` `>` と改行は author 行から黙って落とされ、前後の空白・句読点は削られる。config 書き込み時に改行は `\n` にエスケープされるので設定注入は起きない(実測)
- **署名の有無を `%G?` だけで判定しない** — SSH 署名は `gpg.ssh.allowedSignersFile` 未設定だと未署名と同じ `N` を返す。`git cat-file commit` のヘッダ(`gpgsig`)で存在を確認する。署名パスフレーズはアプリが扱わない(gpg-agent / ssh-agent の pinentry に委譲。**GUI pinentry 必須** — サブプロセスに端末が無い)
- 対話エディタを開かせない(`GIT_EDITOR` / `GIT_SEQUENCE_EDITOR` を非対話に固定して rebase 等を駆動する)
- 全実行にタイムアウトとキャンセルを付ける。auto fetch は多重起動を防ぐ
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド

## Qt Bridges の要点(罠)

- main は `QApp::new().register::<Backend>().load_qml(include_bytes!("qml/Main.qml")).run();` の形。QML はバイナリに埋め込む
- `#[qobject]` は `impl` ブロックに付ける。struct は `Default` 実装必須。プロパティは `qproperty!("name", Member = field, Notify = signal)`、メソッドは `#[qslot]` / `#[qsignal]`。`ConvertToCamelCase` で QML 側を camelCase に
- QML モジュール名は Cargo パッケージ名になる
- 全 QObject は `Rc<RefCell<_>>`(メインスレッド専有)。**QML からの呼び出し中に再入 borrow すると panic** — 借用は短く保つ
- バックグラウンド → UI は `QObjectHolder::get_qml_method_invoker()` で得た `QmlMethodInvoker` をスレッドへ移動し `invoke_method*`(queued 実行)する。それ以外の経路で UI を触らない
- `#[derive(QModelItem)]` は非衛生的展開 — 使用側ファイルに `use std::collections::HashMap;` が必須。ロール名はフィールド名そのまま(snake_case)。`display` という名前のフィールドは Qt の display ロールに割り当てられる
- `QListModelBase` にバッチ挿入は無い(push/insert は 1 行ずつ)。大量追記は `try_get_rust_proxy_ptr()` + `base_begin_insert_rows(first, last)` + `Vec::extend` + `base_end_insert_rows` のレンジ挿入で行う(P0 スパイク `spike/src/main.rs` の `extend_notified` 参照)
- `ConvertToCamelCase` はスロット/シグナルのメタ名を camel 化する — そのオブジェクトへの `invoke_method!` も camel 名で呼ぶこと(混在事故を防ぐため、invoker で呼ぶ対象には付けないのが安全)
- QML モジュール名は既定で Cargo パッケージ名(ハイフン不可)だが、`#[qobject(NoQmlElement)]` + 手動 `impl QmlRegister`(URI 定数)で任意にできる
- Qt Widgets 不可・C++ 混在不可。必要になった時点で CXX-Qt 移行を検討
- 参照実装は公式 examples(`hello_world` / `minimal_app` / `host_monitor` = tokio 連携 / `color_palette`)
- `include_bytes_qml!("dir/file", "prefix")` は **prefix にファイルの相対パス全体を連結**する(qrc:/prefix/dir/file)。ソースのディレクトリ構造 = qrc 構造として設計する。qmldir も埋め込めるので QML singleton(`platitude.ui` の `Theme` / `Metrics`)はこの方式で成立する
- QML は `ui/` 直下フラットに 1 ファイル 1 コンポーネント(`platitude.ui` モジュール)。**新規 QML は qmldir と main.rs の `include_bytes_qml!` の両方へ登録**(qmldir があるディレクトリでは列挙された型しか見えない)
- コンポーネント配線規約: データは親→子へ property、操作・状態変更は子→親へ signal。タブのモデル群は `RepoPage` が所有しペインへ渡す(ペインはモデルのスロットを呼んでよいが、ページ状態は signal で上げて page が変更する)。ウィンドウ横断(確認ダイアログ・タブを開く・settings)は Main まで signal で上げる。`GraphPane.view` は自動化フック専用の露出
- QML の font 値型に `families`(配列)は無い — フォールバックは `Qt.fontFamilies()` と照合して Theme 側で 1 家族に解決する
- `grabToImage` は `Window.contentItem` には使えない("no QML engine")— QML 宣言したアイテムを対象にする
- **画面全体の入力観測は最前面オーバーレイ + `PointHandler`**(passive grab のみが仕様保証)。TapHandler は DragThreshold でも press を消費して下のコントロールへ届かなくし、contentItem 直付けでは手前の MouseArea が accept した press が届かない(2026-08-04 実測)
- **Flickable(ListView 含む)に宣言した子は contentItem に養子入りする** — そのままではコンテンツと一緒にスクロールして流れ去る。ビュー枠に固定するオーバーレイ(スクロール位置で出入りする現在ブランチ行など)は `parent:` でビュー自身を指し、描画も入力もデリゲートより手前に来る(2026-08-04 実測)

## Windows での実行・デバッグの罠

- Qt / QML のログ(console.*、QML ロードエラー含む)は既定で OutputDebugString 行き — **`QT_FORCE_STDERR_LOGGING=1` を付けないと stderr に出ず、QML の失敗が無音になる**
- release ビルドは GUI サブシステム(`windows_subsystem`)のため PowerShell から直接起動すると**待機されない**(即座に制御が返り、プロセスが残って exe をロックする)。検証は `Start-Process -PassThru` + `WaitForExit` で行う
- **画面ロック中は通常起動の GUI 検証がハングする**(プロセスは動きログも出るが、`grabToImage` の完了と `PG_AUTO_QUIT_MS` の自動終了が発生しない — 2026-08-03 ロック実測)。GUI 起動を伴う検証は必ず `WaitForExit(ms)` タイムアウト + 未終了なら `Kill()` のガード付きで実行し、無限待ち・無限ポーリングをしない
- **ヘッドレス検証の標準**(ロック状態と無関係に成立、2026-08-03 ロック実測): `QT_QPA_PLATFORM=offscreen` + `QT_QPA_FONTDIR=C:\Windows\Fonts` + 自動化 env(PG_AUTO_OPEN / PG_AUTO_QUIT_MS / PG_SHOT_DIR / PG_AUTO_SELECT 等)。成否は stderr の `screenshot saved=true` と保存 PNG の目視で判定する。**FONTDIR 指定が無いと全文字が豆腐**(offscreen は Windows のシステムフォントを自動検出しない)
- fps 計測(PG_AUTO_SCROLL)は offscreen でも完走するが、値は疑似フレームループの上限で表示性能ではない — **性能実測はアンロック状態の通常起動でのみ行う**
- **ポップアップ(Popup / Dialog / Menu)は `grabToImage` に写らない** — ウィンドウの
  オーバーレイ層に描かれ、掴んだアイテムの部分木の外にいる。検証は OS 側から
  `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)` で撮る(GPU 描画のため flags 必須。
  アンロック中はダイアログも写る — 2026-08-03 設定ダイアログで実測)。
  キー入力の注入は `SendKeys` が届かない(このシェルはフォアグラウンドを取れず、
  ユーザーの操作中ウィンドウへ飛ぶ危険もある)。`PostMessage(hwnd, WM_KEYDOWN/UP)` を使う。
  クリックも `PostMessage(WM_LBUTTONDOWN/UP)` で確実に届くが、**hover は注入で検証不能**
  (`WM_MOUSEMOVE` 注入・`SetCursorPos` とも実マウスの動きに hover 状態を奪還され、
  成功と失敗が再現不能に混ざる — 2026-08-03 実測)。hover の見た目は実操作で確認する。
  **フォーカスは要アクティブ化**(非アクティブウィンドウでは `activeFocusItem` が
  null のまま。PostMessage はアクティブにしないが、フォアグラウンドスレッドへ
  `AttachThreadInput` してから `SetForegroundWindow` すれば奪えて検証可能 — 2026-08-04 実測)
- exe の**起動**にも Qt の bin ディレクトリが PATH に要る(ビルド時だけではない)。無いと**約 10ms で無言終了**する — ログもエラーダイアログも出ないので死因が判らない。検証スクリプトは PATH 設定込みで書く

## ビルド・テスト

前提: Qt(qtbridge が要求するバージョン以上)と C++ ツールチェーンがインストール済みで、`qmake` が PATH にあること。

- Windows: `C:\Qt\<バージョン>\<ツールチェーン>\bin` を PATH に追加
- Ubuntu(ディストリの Qt パッケージ利用時): `QMAKE=qmake6` を設定
- macOS: Qt の `bin` を PATH に、`DYLD_FRAMEWORK_PATH` に Qt の `lib`

```bash
cargo build                              # 開発は debug ビルド。--release は性能計測時のみ
cargo test -p <crate> <テスト名>          # まず最小スコープで
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

**Done の基準**: fmt / clippy / test が全て通ること。テストを実行していないコードは動かないものとして扱う。

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 依存追加は最小限(軽量が目標)。追加時はライセンス確認必須
- 識別子・コメント・ログ・コミットメッセージは英語(設計メモ等の docs は日本語可)
- スナップショット(insta)の手編集禁止。再生成して差分をレビューする
- パーサのテストは実 git の出力を fixture として保存して回す。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する

## 性能予算([要望.md](internal-docs/要望.md) 性能要件より)

`JetBrains/kotlin` 級(10万コミット超)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
コミット数に比例する同期処理を UI 操作の経路に置かない。遅延読み込みと差分更新を基本とする。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)、メッセージは英語
- force push しない

## 現在のフェーズ: **Phase 2 / 3 の日常操作まで配線済み**

- Phase 1(読み取り専用ビューア)は**完了**。Done 条件の性能 4 項目は完了時点の最終確認でもクリア(first chunk 79ms / 詳細 75ms / 178fps / peak 269MB): [ci/baseline/phase1-perf-windows-x64.md](ci/baseline/phase1-perf-windows-x64.md)
- **配線済み**: ステージング(ファイル / hunk / 行)・commit / amend・switch(ローカル / リモート / detach。未コミット変更は**まず持っていき**、git が拒否した時だけ「置いていく / merge して持っていく」を選ばせる)・fetch(手動 + auto)・push / force push・単体 cherry-pick・単体 squash・コミットメッセージ編集・identity・設定(auto fetch 間隔)・diff プレビュー(画像は Before/After 描画、非画像バイナリはサイズ表示 — `platitude-core::preview`)
- **未配線**: merge / rebase / revert・conflict ペイン・フル interactive rebase 画面・ブランチ作成 / 削除 / リネーム・discard / clean・stash push・リモートブランチ削除
- 残作業と要判断事項は [P2-確認事項.md](internal-docs/P2-確認事項.md) / [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読むこと**。配布準備期に検証する項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- **確認ダイアログは取り返しがつかない操作だけ**(force push)。**ローカルの履歴書き換え(amend / squash / メッセージ編集)は push 済みでも尋ねない** — switch / reset で戻せるし、広めるのは push 側が尋ねる。**未保存の入力を捨てる時は尋ねるが、ダイアログではなく編集枠の中で聞く**(編集中のメッセージを残して別コミットへ移動 → 保存行が質問に切り替わる。`DetailsPane.asking`)。日常操作は尋ねない。ダイアログの実体は `Main.qml` の `root.confirm()`
- **QML バインディングはプロパティにしか反応しない** — `#[qslot]` は呼び出し用。`enabled:` 等が値の変化を追う必要があるものは `qproperty!` にする(スロットのままだと初期値のまま固まる)
- 書き込み操作の headless 検証は **`PG_AUTO_ACT` = 動詞 / `PG_AUTO_ACT_ARG`**(commit / amend / switch / switch-leave / switch-merge / switch-remote / squash / reword / edit-message / edit-message-leave / edit-message-discard / cherry-pick / stage-hunk / stage-line / push / force-push / force-push-confirm / fetch / settings / preview / preview-unstaged / preview-staged)。クリックと同じ経路を通る
- ダイアログの見た目は**アンロック中の `PrintWindow` で検証可能**(設定ダイアログで実測済み。ロック中は git version gate で止まる)。メニューは未検証。機能は `PG_AUTO_ACT` で確認済み
- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- interactive rebase は `GIT_SEQUENCE_EDITOR` に**別実行ファイル `pg-todo-editor`** を差す方式。**配布物に同梱必須**(本体と同じディレクトリ)
- グラフは **2 段ストリーミング**(タグ無し即描画→タグ込みを単一 drain で無フリッカー置換。44k タグの topo フロンティア初期化コスト対策)+ `--max-count=2000` ウィンドウ。**WIP(未コミット)を HEAD の子の仮想行**として、**stash を walk 参加の実行行**として描く(合成親コミットは sift で除去)— いずれも `platitude-core::session` 参照。**バックグラウンド更新(auto fetch 後・write 後・dirty ⇄ clean 変化)は `refresh_log()`** = オフスクリーン構築→送信済み行と一致ならイベントを一切出さない(アイドル中のチラつき対策)。リセット→ストリーミングは open / Reload / タグ切替 / 件数変更のみ
- ブリッジは **Qt Bridges 採用で確定**(Phase 0 スパイク合格)。CXX-Qt へ差し替え可能な構成を維持。スパイクコードは `spike/` に残置
- UI は [internal-docs/デザイン規約.md](internal-docs/デザイン規約.md) が正本(Theme.qml と Main.qml 冒頭定数ブロックはその写し)。グラフ・インタラクション定数とレイアウト初期値は **2026-08-02 の UI 基準確定で規約へ昇格済み**
- 開発は **main 直コミット**(ユーザー指示)。release ビルドしないと QML(exe 埋め込み)は反映されない — 起動確認前に必ず `cargo build --release`
- CI(3OS + 完全オフライン job)は記述済みだが **GitHub リモート未設定のため一度も実行されていない**。push は相当先まで行わない方針(2026-08-02 ユーザー指示)のため、初回検証は**配布準備期(P5 目安)まで大幅後ろ倒し**
- 確定意匠: ブランチ状態バッジ = 無表示(ローカルのみ)/ 雲(リモートあり)/ **緑の PR アイコン**(PR中 — データ接続は P4、`PG_FAKE_PR=名前` で見た目をプレビュー可)。**タグの「ローカルのみ」区別は無表示が確定意匠**(バッジを足さない)
- ネットワーク非通信の baseline 実測: [ci/baseline/windows-x64.md](ci/baseline/windows-x64.md)。**Qt6Network は Qt6Qml のロード時依存として同梱が必須** — 「同梱しない」ではなく「アプリ自身の import table に通信系なし + ネットワーク系プラグイン除外」を主張する
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。200 行以下を維持する
- コードから読み取れるアーキテクチャ説明は書かない(陳腐化するため)。罠と決定事項のみを記す
- **バージョン番号をハードコードしない**。ツールチェーン・依存の正確なバージョンは Cargo.toml / ロックファイルを、製品要件は [要望.md](internal-docs/要望.md) を正とする(方針は「最新から開始」)

## 参照リンク

- qtbridge ドキュメント: https://doc-snapshots.qt.io/qtbridge-rust/qtbridge/index.html
- qtbridge リポジトリ / examples: https://github.com/qt/qtbridge-rust
- CXX-Qt book(フォールバック): https://kdab.github.io/cxx-qt/book/
