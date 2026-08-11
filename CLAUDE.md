# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。**Intel Mac は対象外**。
機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に必ず該当セクションを読むこと(性能要件は本ファイル §性能予算)。
本ファイルは全セッション共通の不変条件だけを持つ。**規約の本体は分割配置**: core の git サブプロセス規約・セッション実装は [.claude/rules/core.md](.claude/rules/core.md)、app の Qt Bridges / QML / 意匠実装は [.claude/rules/app-ui.md](.claude/rules/app-ui.md)(いずれも該当クレートのファイルに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — rules 本体の指示に従い、触る項を Grep で引く)、ヘッドレス動確の手順と `PG_AUTO_ACT` 動詞表は **verify-ui スキル**。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md) の定めに従う。それ未満向けのフォールバックコードを書かない
- ライセンス: **公開前提 = 同梱・再配布するものが全て著作権上クリアであること**。コードだけでなく**フォント・アイコン・テーマ等の意匠素材も対象**(素材は許可集合がコード依存と違うので個別に人の承認を取る)。アプリ本体は MIT。依存追加は MIT / Apache-2.0 / BSD / Zlib / MPL-2.0 系のみ。**GPL 系依存は禁止**(Qt 本体と qtbridge は LGPL-3.0-only で利用 — 承認済みの例外)
- ライブラリ新規導入について、以下に関するものは明示的禁止。他ライブラリ追加やバージョン変更はライセンス確認と人の許可が必須
  - git 実装（git 操作は**システム git のサブプロセス実行のみ**）
  - ネットワーク通信（**通信はgit コマンド経由のみ**）
- UI はダークテーマ(青系)のみ。文言は英語のみ・ハードコード禁止(`qsTr()` 必須、将来の i18n に備える)
- **内部コマンドと UI 表記は意図して分ける**。内部は最新 git の適切なコマンド(`switch` / `restore` 等)、UI 文言はコマンド名に引きずられず「その操作が何をするか」で選ぶ。用語とコード表記(チップ)の正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表・§git 用語のコード表記
- UI の値は [デザイン規約.md](internal-docs/デザイン規約.md) のトークンのみ使用 — QML を書く前に必ず該当 § を引き、値は表から選ぶ(**数値を検討・微調整しない**)。数値・色・フォント名の直書き禁止。トークンの追加・変更には人間の承認が必要

## 技術スタック

- 言語は Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート。通常の依存として追加、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う。他の依存・バージョンは Cargo.toml が正
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結

## ビルド・テスト

前提: Qt(qtbridge の要求以上)+ C++ ツールチェーンがあり、`qmake` が PATH にあること。Windows は Qt の `bin` を PATH に追加 / Ubuntu(ディストリ Qt)は `QMAKE=qmake6` / macOS は Qt の `bin` を PATH に、`DYLD_FRAMEWORK_PATH` に Qt の `lib`。

開発は debug ビルド。**`--release` は性能計測と起動確認だけ**だが、release でないと QML(exe 埋め込み)が反映されない。以下 `cargo` / `cargo xtask` を省略。

**確認は 3 段**(混ぜると日常が重くなるか反映が甘くなる)。**1 日常** = コンテナ無し / **2 反映前** = 軽量 CI = workspace の fmt / clippy / test + `linux test -p platitude-core` + 触った動詞の `linux verify-ui` + `linux bare` / **3 フル** = 完全性 CI = 2 + `linux bare --discover` + 3OS CI + 性能実測(リリース前と、依存や環境を触った時)。**段 2 は `check --verb <触った動詞>…` で一括実行**(ホストとコンテナは書き込み先が別 = 並列で回る)。

**Done の基準**: 段 2 が全て通ること。テストを実行していないコードは動かないものとして扱う。UI 配線の Done は **両 OS の `verify-ui` が同じ動詞で PASS し、両方の PNG を目視するまで**(手順・動詞表・Windows の罠は **verify-ui スキル**を必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — 起動までを複合コマンドで先に済ませて即報告し、fmt / clippy / test は報告後にバックグラウンドで追報する(Done の基準は不変)。手順は verify-ui スキル §起動 fast path
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ。Linux ではその場で実行 — `bare` だけは常にコンテナ)。イメージは core / app / runtime(**宣言した依存だけ**)から自動で選び、ビルド先は docker volume でこの `target/` を汚さない。最低 git バージョンを積んだ唯一の環境。**`bare` は建てた場所の外で動くかだけを見る**(依存が増えた瞬間その名前で止まる。実測は P5-確認事項 §実測済み)
- **並行セッションは git worktree で分ける** — 同一 checkout の共有は `target/` が単一障害点(ビルドロック直列化・incremental 相互無効化・別セッション編集の焼き込み実績)。worktree なら target も demo / screenshot も自然に分離される
- **worktree は固定の座席 `a`〜`f` だけ**: `claude --worktree a` / セッション内は EnterWorktree で**空き座席の path** へ。トピック名・自動命名で**新造しない**(用途名は再利用されず、席の `target/` 温存が働かない — 非座席名は hook が確認を挟む)。空き状況と**推薦席**は開始時の挨拶が言う。**席は早い者勝ち**、取り損ねたら別の空き文字へ。**席のブランチ(`worktree-<席>`)がマージ済みなら `git reset --hard main` で先頭に揃えてから始める**。未マージの席は前の仕事のマージ待ち — 続き以外は別の席へ、全席詰まりなら増設せず報告する。**完了しても main へは戻さない**(§Git 運用)
- **本体 checkout で実装作業をしない**(ドキュメント編集・レビューは可。**ただしスキル・`.claude/rules`・`.claude/rules-refs` の編集は worktree で** — 並行セッションが同じファイルを触りやすく、main 直コミットが衝突する)
- **worktree からのアプリ起動は headless だけ** — 実ウィンドウは画面(と別セッションの窓検証)を横取りし、居座るプロセスは exe を掴んでリンクを塞ぐ。`cargo xtask verify-ui` を使うか、自分で叩くなら `QT_QPA_PLATFORM=offscreen` + `PG_AUTO_QUIT_MS` + **自分の worktree の** exe。`cargo xtask hook pre-shell` がそれ以外を deny する(ユーザーが窓を明示指示した時だけ `PG_ALLOW_GUI=1` を先頭に付ける)。本体 checkout からの起動は対象外(ユーザー自身の起動)。**窓に出たビルドがどのツリーのものかは右下が名乗る**(実装は rules-refs/app-ui.md)
- 開発補助ツールを **Windows 専用形式(.ps1 / .bat)で作らない** — タスクランナーが要る時は `cargo xtask` パターン(ワークスペース内クレート + `.cargo/config.toml` の alias、依存は std のみ)で 3OS 同一に書き、OS 差(Qt の PATH / フォント等)はコード内の分岐に焼き込む。just / make 等の外部タスクランナーも導入しない

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 識別子・コメント・ログ・コミットメッセージは英語(設計メモ等の docs は日本語可)。ログは `tracing`(`println!` / `eprintln!` 禁止)
- スナップショット(insta)の手編集禁止。再生成して差分をレビューする
- パーサのテストは実 git の出力を fixture として保存して回す。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する
- **git の挙動に確信が持てなければ、実装の前に使い捨てリポジトリで実測する**(`tests/it/support` の `TestRepo` = tempdir + 実 git + 決定的 SHA)。観測した挙動をテストへ固定してから実装する — 想定だけで書くと実装とテストが**同じ間違いで揃って緑のまま壊れる**。動確で壊れたら、直す前に再現する統合テストが赤になるのを確認する

## 性能予算

`JetBrains/kotlin` 級(10万コミット超)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
コミット数に比例する同期処理を UI 操作の経路に置かない。遅延読み込みと差分更新を基本とする。
**refs の本数にも比例させない** — 基準リポジトリは refs が 5 万本(うちタグ 4.5 万)で、ループの中の走査は積になる。ref 同士を突き合わせる時は索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`。[refs-join 実測](ci/baseline/refs-join-windows-x64.md))。未着手の非同期化の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)、メッセージは英語
- force push しない
- **rebase はその場でユーザーが指示した時だけ**(main への追従・履歴の squash を含む)。worktree ブランチが main より遅れたままは正常な状態で、直す対象ではない(例外は座席のマージ済みブランチの `reset --hard main` — §ビルド・テスト)。`hook pre-shell` が deny する(`--abort` / `--quit` は除く)— 指示があった時だけ `PG_ALLOW_REBASE=1` を先頭に付ける
- **main へブランチを反映するのは、その場でユーザーが指示した時だけ**。セッションはコミットを `worktree-<名前>` に積んだまま「マージ可」と報告して終わる。自分の判断で ff-merge しない — 反映済みと未反映が混ざると管理できなくなる
  - `cargo xtask hook pre-shell` が main を書く git(`merge` / `:main` への refspec / `branch -f main` / `update-ref`)と、本体 checkout からの `.claude/skills` / `.claude/rules` / `.claude/rules-refs` を含むコミットを deny する。**指示があった時だけ** `PG_ALLOW_MAIN=1` を先頭に付けて再実行する。使い捨てリポジトリと worktree ブランチ上のコミットは対象外
  - **反映は本体 checkout の `git merge` で行う** — `update-ref` / `branch -f` は本体の index と作業ツリーを置き去りにし、落差が staged に見える(**中身は HEAD より後ろ** — コミットすると反映済みの仕事が消える)。診断は `git reflog show main`、復旧は `git restore --source=HEAD --staged --worktree -- .`
- 本体 checkout での直コミットは可(ドキュメント等。**`.claude/skills` / `.claude/rules` / `.claude/rules-refs` は除く** — worktree に積んで反映指示を待つ)。main の ref を動かす操作のうち止まるのは上記の反映系だけ

## 現在のフェーズ: **Phase 2 / 3 の日常操作まで配線済み**

- 性能 4 項目は基準リポジトリで全て予算内: [実測](ci/baseline/perf-windows-x64.md)(余白が薄いのはメモリ)
- **配線済み操作の一覧・個々の意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**(触る項を Grep で引く。app の不変条件は [.claude/rules/app-ui.md](.claude/rules/app-ui.md) が自動ロード)。**本ファイルは未配線だけを持つ** — ここに一覧を置くと機能を足すたび太る
- **未配線**: フル interactive rebase 画面のみ(その周辺の日常操作は配線済み — 一覧は rules-refs)
- 残作業と要判断事項は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読むこと**。配布準備期に検証する項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- CI(3OS + 完全オフライン job)は記述済み・**pushするまで実行しない**。初回検証は**配布準備期(P5)**。CI も軽量(段 2)/ 完全性(段 3)に分ける(中身は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3.5)
- ネットワーク非通信の baseline 実測: [baseline](ci/baseline/windows-x64.md)(主張の立て方は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3)
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持** — 常時ロードされ、肥大化すると遵守率が下がる)/ core 実装の規約・罠 → [.claude/rules/core.md](.claude/rules/core.md) / app・QML の規約・意匠 → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)(**rules 本体は不変条件だけ。各論は `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行で追記** — `.claude/rules/` 配下は再帰スキャンされ、frontmatter 無しの .md は常時ロードされるため、参照ファイルを rules の下に置かない)/ 検証手順・自動化動詞 → verify-ui スキル / 機械で守れる禁止事項 → `.claude/settings.json` の hooks(実体は `cargo xtask hook`)
- コードから読み取れるアーキテクチャ説明は書かない(陳腐化するため)。罠と決定事項のみを記す
- **バージョン番号をハードコードしない**。ツールチェーン・依存の正確なバージョンは Cargo.toml / ロックファイルを、製品要件は [実装計画.md](internal-docs/実装計画.md) を正とする(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)を本ファイルに置かない**。索引だけを置き、実体は分割先へ
