# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。
「インストール済み git の CLI を実行するだけの薄い GUI」に徹する。
機能要件・性能要件・スコープ外の正本は [要望.md](internal-docs/要望.md) — 機能実装の前に必ず該当セクションを読むこと。
UI の色・タイポグラフィ・寸法の正本は [デザイン規約.md](internal-docs/デザイン規約.md) — QML を書く前に必ず読み、値は表から選ぶこと(**数値を検討・微調整しない**)。
本ファイルは全セッション共通の不変条件だけを持つ。**規約の本体は分割配置**: core の git サブプロセス規約・セッション実装は [.claude/rules/core.md](.claude/rules/core.md)、app の Qt Bridges / QML / 意匠実装は [.claude/rules/app-ui.md](.claude/rules/app-ui.md)(いずれも該当クレートのファイルに触れると自動ロード)、ヘッドレス動確の手順と `PG_AUTO_ACT` 動詞表は **verify-ui スキル**。

## 絶対制約(変更には人間の明示承認が必要)

- git 操作は**システム git のサブプロセス実行のみ**。libgit2 / gitoxide(gix)等の git 実装ライブラリを導入しない
- ネットワーク通信は**git コマンド経由のみ**。HTTP クライアント・telemetry・forge API(GitHub API 等)のクレートを導入しない
- 認証(ssh / credential helper)・hooks・gitconfig・**署名(gpg / ssh)** は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- AI 機能を実装しない
- ライセンス: アプリ本体は MIT。依存追加は MIT / Apache-2.0 / BSD / Zlib / MPL-2.0 系のみ。**GPL 系依存は禁止**(Qt 本体と qtbridge は LGPL-3.0-only で利用 — 承認済みの例外)
- 対応 git の最低バージョンは [要望.md](internal-docs/要望.md) の定めに従う。それ未満向けのフォールバックコードを書かない
- UI はダークテーマ(青系)のみ。文言は英語のみ・ハードコード禁止(`qsTr()` 必須、将来の i18n に備える)
- **内部コマンドと UI 表記は意図して分ける**。内部は最新 git の適切なコマンドを選ぶ(`switch` / `restore` 等)が、UI 文言は git のコマンド名に引きずられず「その操作が何をするか」を最も適切に表す語を選ぶ。用語とコード表記(チップ)の正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表・§git 用語のコード表記
- UI の値は [デザイン規約.md](internal-docs/デザイン規約.md) のトークンのみ使用。QML への数値・色・フォント名の直書き禁止。トークンの追加・変更には人間の承認が必要

## 技術スタック

- 言語は Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート。通常の依存として追加、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う。他の依存・バージョンは Cargo.toml が正
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結

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

**Done の基準**: fmt / clippy / test が全て通ること。テストを実行していないコードは動かないものとして扱う。UI 配線の Done は `cargo xtask verify-ui` が PASS し PNG を目視するまで(手順・動詞表・Windows の罠は **verify-ui スキル**を必ず呼ぶ)。

- release ビルドしないと QML(exe 埋め込み)は反映されない — 起動確認前に必ず `cargo build --release`
- **統合テストは 1 バイナリ**(`tests/it/` のモジュール。`cargo test` はバイナリを 1 つずつ走らせるので、`tests/` 直下に .rs を足すと別バイナリ = 直列実行とリンク 1 本分の後退。新しい統合テストは `it/` にモジュールとして足し `main.rs` へ登録)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`
- **並行セッション(複数エージェント)は git worktree で分ける** — 同一 checkout の共有は `target/` が単一障害点(cargo のビルドロックで直列化・incremental を相互に無効化・verify-ui が起動する release exe に別セッションの編集が焼き込まれた実績)。worktree なら target も demo / screenshot(temp 下の nanos 付きユニークパス)も自然に分離される
- **worktree は固定名を使い回す**: `claude --worktree <固定名>`(`.claude/worktrees/<固定名>` に恒久作成・次回同名で再開。worktree ごとの `target/` = incremental キャッシュがセッションを跨いで温存される)。使い捨ての自動命名 worktree を乱造しない。ブランチは `worktree-<名前>` に切られる。**完了しても main へは戻さない**(§Git 運用)。**本体 checkout で実装作業をしない**(ドキュメント編集・レビューは可)
- **worktree からのアプリ起動は headless だけ** — 実ウィンドウは画面を横取りし(別セッションの実ウィンドウ検証ごと)、終わり方を持たないプロセスは `target/release` の exe を掴んだままリンクを塞ぐ。`cargo xtask verify-ui` を使うか、自分で叩くなら `QT_QPA_PLATFORM=offscreen` + `PG_AUTO_QUIT_MS` + **自分の worktree の** exe。`cargo xtask hook pre-shell` が worktree セッションのそれ以外を deny する(ユーザーが窓を明示指示した時だけ `PG_ALLOW_GUI=1` を先頭に付ける)。本体 checkout からの起動は対象外 — ユーザー自身の起動で、重なることは許容
- 開発補助ツール(検証・デモ環境生成等)を **Windows 専用形式(.ps1 / .bat)で作らない** — タスクランナーが要る時は `cargo xtask` パターン(ワークスペース内クレート + `.cargo/config.toml` の alias、依存は std のみ)で 3OS 同一に書き、OS 差(Qt の PATH / フォント等)はコード内の分岐に焼き込む。just / make 等の外部タスクランナーも導入しない

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 依存追加は最小限(軽量が目標)。追加時はライセンス確認必須
- 識別子・コメント・ログ・コミットメッセージは英語(設計メモ等の docs は日本語可)。ログは `tracing`(`println!` / `eprintln!` 禁止)
- スナップショット(insta)の手編集禁止。再生成して差分をレビューする
- パーサのテストは実 git の出力を fixture として保存して回す。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する
- **git の挙動に確信が持てなければ、実装の前に使い捨てリポジトリで実測する**(`tests/it/support` の `TestRepo` = tempdir + 実 git + 決定的 SHA)。観測した挙動をテストへ固定してから実装する — 想定だけで書くと実装とテストが**同じ間違いで揃って緑のまま壊れる**。動確で壊れたら、直す前に再現する統合テストが赤になるのを確認する
- **「もう起きない」を sleep で確かめない** — キューに乗った書き込みは前の write の refresh まで終わってから始まるので、静かな時間の長さは「止まった」と「遅い」を区別しない(`cargo test --workspace` の負荷で落ちる)。タイマは手で進めて、進めた先が受け取ったかどうかを見る(`RepoSession::auto_fetch_ticker`)

## 性能予算([要望.md](internal-docs/要望.md) 性能要件より)

`JetBrains/kotlin` 級(10万コミット超)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
コミット数に比例する同期処理を UI 操作の経路に置かない。遅延読み込みと差分更新を基本とする。
**refs の本数にも比例させない** — 基準リポジトリは refs が 5 万本(うちタグ 4.5 万)で、ループの中の走査は積になる。ref 同士を突き合わせる時は索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`。実測 [ci/baseline/refs-join-windows-x64.md](ci/baseline/refs-join-windows-x64.md))。未着手の非同期化の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)、メッセージは英語
- force push しない
- **main へブランチを反映するのは、その場でユーザーが指示した時だけ**(ユーザー指示)。セッションはコミットを `worktree-<名前>` に積んだまま「マージ可」と報告して終わる。自分の判断で ff-merge しない — 反映済みと未反映が混ざると管理できなくなる
  - `cargo xtask hook pre-git` が main を書く git(`merge` / `:main` への refspec / `branch -f main` / `update-ref`)を deny する。**指示があった時だけ** `PG_ALLOW_MAIN=1` を先頭に付けて再実行する。使い捨てリポジトリと worktree ブランチ上のコミットは対象外
- 本体 checkout での直コミットは可(ドキュメント等)。main の ref を動かす操作のうち止まるのは上記の反映系だけ

## 現在のフェーズ: **Phase 2 / 3 の日常操作まで配線済み**

- Phase 1(読み取り専用ビューア)は**完了**。Done 条件の性能 4 項目は完了時点の最終確認でもクリア(first chunk 79ms / 詳細 75ms / 178fps / peak 269MB): [ci/baseline/phase1-perf-windows-x64.md](ci/baseline/phase1-perf-windows-x64.md)
- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules/app-ui.md](.claude/rules/app-ui.md) が正**(app のファイルに触れると自動ロード)。**本ファイルは未配線だけを持つ** — ここに一覧を置くと機能を足すたび太る
- **未配線**: フル interactive rebase 画面(merge / rebase / revert / 単体 drop の起動、止まった操作の出口 = continue / skip / quit / abort、conflict の種別・片側採用・外部ツールへの受け渡しは配線済み)
- 残作業と要判断事項は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読むこと**。配布準備期に検証する項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- CI(3OS + 完全オフライン job)は記述済みだが **GitHub リモート未設定のため一度も実行されていない**。push は相当先まで行わない方針(2026-08-02 ユーザー指示)のため、初回検証は**配布準備期(P5 目安)まで大幅後ろ倒し**
- ネットワーク非通信の baseline 実測: [ci/baseline/windows-x64.md](ci/baseline/windows-x64.md)(主張の立て方は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3)
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持** — 行数ではなくサイズ。常時ロードされ、肥大化すると遵守率が下がる)/ core 実装の規約・罠 → [.claude/rules/core.md](.claude/rules/core.md) / app・QML の規約・意匠 → [.claude/rules/app-ui.md](.claude/rules/app-ui.md) / 検証手順・自動化動詞 → verify-ui スキル / 機械で守れる禁止事項 → `.claude/settings.json` の hooks(実体は `cargo xtask hook`)
- コードから読み取れるアーキテクチャ説明は書かない(陳腐化するため)。罠と決定事項のみを記す
- **バージョン番号をハードコードしない**。ツールチェーン・依存の正確なバージョンは Cargo.toml / ロックファイルを、製品要件は [要望.md](internal-docs/要望.md) を正とする(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)を本ファイルに置かない**。索引だけを置き、実体は分割先へ
