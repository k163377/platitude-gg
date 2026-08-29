# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。**Intel Mac は対象外**。
機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に該当セクションを読む(性能要件は本ファイル §性能予算)。
本ファイルは全セッション共通の不変条件だけ。**規約本体は分割配置**: core → [.claude/rules/core.md](.claude/rules/core.md)、app → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)、分割・共通化 → [.claude/rules/structure.md](.claude/rules/structure.md)(該当クレートに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — 触る項を Grep で引く)、動確手順と `PG_AUTO_ACT` 動詞表は **verify-ui スキル**。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md) に従う。それ未満向けのフォールバックコードを書かない
- ライセンス: **同梱・再配布するものが全て著作権上クリアであること** — **フォント・アイコン・テーマ等の意匠素材も対象**(素材は許可集合がコード依存と違うので個別に人の承認)。アプリ本体は MIT。依存追加は MIT / Apache-2.0 / BSD / Zlib / MPL-2.0 系のみ、**GPL 系禁止**(Qt 本体と qtbridge は LGPL-3.0-only — 承認済みの例外)
- ライブラリ新規導入のうち以下は明示的禁止。他の追加・バージョン変更もライセンス確認と人の許可が必須
  - git 実装(git 操作は**システム git のサブプロセス実行のみ**)
  - ネットワーク通信(**通信は git コマンド経由のみ**)
- UI はダークテーマ(青系)のみ。文言は英語のみ・ハードコード禁止(`qsTr()` 必須)
- **内部コマンドと UI 表記は分ける** — 内部は最新 git の適切なコマンド(`switch` / `restore` 等)、UI 文言は「その操作が何をするか」で選ぶ。正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表・§git 用語のコード表記
- UI の値は [デザイン規約.md](internal-docs/デザイン規約.md) のトークンのみ — 該当 § を引き、値は表から選ぶ(**数値を検討・微調整しない**)。数値・色・フォント名の直書き禁止。トークンの追加・変更には人間の承認が必要

## 技術スタック

- Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う。依存・バージョンは Cargo.toml が正
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結

## ビルド・テスト

前提: Qt(qtbridge の要求以上)+ C++ ツールチェーン、`qmake` が PATH に。Windows は Qt の `bin` を PATH に追加 / Ubuntu(ディストリ Qt)は `QMAKE=qmake6` / macOS は Qt の `bin` を PATH に、`DYLD_FRAMEWORK_PATH` に Qt の `lib`。

開発は debug ビルド。**`--release` は性能計測と起動確認だけ**だが、release でないと QML(exe 埋め込み)が反映されない。以下 `cargo` / `cargo xtask` を省略。

**確認は 3 段**。**1 日常** = コンテナ無し / **2 反映前** = 軽量 CI = workspace の fmt / clippy / test + `linux clippy`(**ホストの clippy は `cfg(not(windows))` の中を一切コンパイルしないので、そこだけで死ぬ名前は Linux / mac 側でしか赤くならない**)+ `linux test -p platitude-core` + 触った動詞の `linux verify-ui` + `linux bare` / **3 フル** = 完全性 CI = 2 + `linux bare --discover` + 3OS CI + 性能実測(リリース前と、依存や環境を触った時)。**段 2 は `check --verb <触った動詞>…` で一括実行**。

**Done の基準**: 段 2 が全て通ること。UI 配線の Done は **両 OS の `verify-ui` が同じ動詞で PASS し、両方の PNG を目視するまで**(手順・動詞表・Windows の罠は **verify-ui スキル**を必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — シェル呼び出し 1 個で起動し、即報告してターンを終える。**起動したら監視しない**(背景タスク・生存確認・撃ち直しを後ろに吊らない — ターンが終わらない間ユーザーの次の指示は届かない)。**`launch` をパイプ・コマンド置換に通さない**(窓の寿命だけターンが返らない。hook が deny)。Done の基準は不変で、段 2 はユーザーが検証・反映を指示した時に走らせる。手順は verify-ui スキル §起動 fast path
- **テストは thread / process の並行実行が既定** — `--test-threads` を下げて通さず、固定 temp path / port / 設定名や process-global 可変状態を共有しない。非同期テストの因果的な待ち方と反復判定は core 規約 §非同期・並行テストの実装方針
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ。Linux ではその場で実行 — `bare` だけは常にコンテナ)。イメージは core / app / runtime(**宣言した依存だけ**)から自動選択、ビルド先は docker volume。最低 git バージョンを積んだ唯一の環境。**`bare` は建てた場所の外で動くかだけを見る**(実測は P5-確認事項 §実測済み)
- **実装作業は worktree 座席 `a`〜`f` で行う**(`claude --worktree <席>` / EnterWorktree で空き席の path へ。本体 checkout はドキュメント・レビューのみ — `target/` と release exe の取り合いを避ける)。空き状況は挨拶が言う。**先に空きを調べない — まず座る**: 入席時に hook が `git worktree lock` で claim し、取られていれば deny するので、**その deny が答え**(別の空き文字へ入り直す。席は早い者勝ち・非座席名の新造は hook が確認を挟む)。調べてから座ると、調べた瞬間と座る瞬間の間で取られる — `cargo xtask seats` は状況を読む道具であって、座るかどうかの判断には使わない。**claim の返却も自動: land が成功した席をその場で unlock し(反映完了 = 離席)、以後の追加作業は席内への最初の編集で hook が再 claim する**。手動 `git worktree unlock` は消えたセッションの席の回収だけ。マージ済みの席は `git reset --hard main` で先頭に揃えてから始める。未マージの席は続きの仕事以外触らない — 全席詰まりなら増設せず報告。**完了しても main へは戻さない**(§Git 運用)
- **worktree からのアプリ起動は headless(`cargo xtask verify-ui`)だけ**。実ウィンドウはユーザーが明示した時だけ **`PG_ALLOW_GUI=1 cargo xtask launch`**(自ツリーの居残り回収→ビルド→起動→生存確認まで一括)。exe が掴まれている・二重起動ゲートが出た時は **`cargo xtask kill`** — 原因は常に自ツリーの居残りで、これはそれだけを落とす(**画像名 kill は他席とユーザーの窓を巻き込むので hook が deny**)。本体 checkout からの起動は対象外(ユーザー自身の起動)。**窓のビルドがどのツリーのものかは右下が名乗る**(実装は rules-refs/app-ui.md)
- 開発補助ツールを **.ps1 / .bat で作らない** — タスクランナーは `cargo xtask` パターン(ワークスペース内クレート + `.cargo/config.toml` の alias、依存は std のみ)で 3OS 同一に書き、OS 差はコード内の分岐に焼き込む。just / make 等の外部タスクランナーも導入しない

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 識別子・コメント・ログ・コミットメッセージは英語(docs は日本語可)。ログは `tracing`(`println!` / `eprintln!` 禁止)
- スナップショット(insta)の手編集禁止。再生成して差分をレビューする
- パーサのテストは実 git の出力を fixture に。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する
- **git の挙動に確信が無ければ、実装前に使い捨てリポジトリで実測する**(`tests/it/support` の `TestRepo`)。観測をテストへ固定してから実装。動確で壊れたら、直す前に再現テストが赤になるのを確認する

## 性能予算

`JetBrains/kotlin` 級(10万コミット超)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
コミット数に比例する同期処理を UI 操作の経路に置かない。遅延読み込みと差分更新が基本。
**refs の本数にも比例させない**(基準リポジトリは refs 5 万本・うちタグ 4.5 万)— ref 同士の突き合わせは索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`。[refs-join 実測](ci/baseline/refs-join-windows-x64.md))。未着手の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)
- force push しない
- **rebase はその場でユーザーが指示した時だけ**(main への追従・squash を含む)。worktree ブランチが main より遅れたままは正常(例外は座席のマージ済みブランチの `reset --hard main`)。`hook pre-shell` が deny(`--abort` / `--quit` は除く)— 指示があった時だけ `PG_ALLOW_REBASE=1` を先頭に付ける
- **main を動かすのもその場でユーザーが指示した時だけ**。セッションは `worktree-<席>` に積んだまま「マージ可」と報告して終わる。**反映の指示を受けたら即 `PG_ALLOW_MAIN=1 cargo xtask land <branch>`**(fast path と同格 — 自分の Done ゲートや段 2 の完了待ちを前提条件にしない)。land はどのセッションからでも動き、本体 checkout の HEAD がどこに居ても安全な手を選ぶ(worktree セッションの git は自ツリーに隔離され、手動 `git merge` は本体に届かない。`branch -f` / `update-ref` の手動反映は本体の index を置き去りにする既知の罠)。hook が main を書く git を deny して land へ誘導する
- 本体 checkout での直コミットは可(ドキュメント等。**`.claude/skills` / `.claude/rules` / `.claude/rules-refs` は除く** — worktree に積んで反映指示を待つ)

## 現在のフェーズ: **Phase 2 / 3 の日常操作まで配線済み**

- 性能 4 項目のうち**メモリだけが予算超過**(基準リポジトリで 350–364MB / 予算 300MB): [実測](ci/baseline/perf-windows-x64.md)。**超過分はグラフを一度流すと高いまま**(流さなければ 298–299MB)で、漏れではなく行を組んだ C++ ヒープが返らない形 — 残件と残る手は [P3-確認事項.md](internal-docs/P3-確認事項.md) §app。起動・応答・fps は予算内
- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**。本ファイルは未配線だけを持つ — **未配線はフル interactive rebase 画面のみ**
- 残作業と要判断事項は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読む**。配布準備期の検証項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- CI(3OS + 完全オフライン job)は記述済み・**push するまで実行しない**。初回検証は配布準備期(P5)。CI も軽量(段 2)/ 完全性(段 3)に分ける([P5-確認事項.md](internal-docs/P5-確認事項.md) §3.5)
- ネットワーク非通信の baseline: [windows-x64](ci/baseline/windows-x64.md)(主張の立て方は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3)
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持**)/ core → [.claude/rules/core.md](.claude/rules/core.md) / app・QML → [.claude/rules/app-ui.md](.claude/rules/app-ui.md) / 分割・共通化(全クレート) → [.claude/rules/structure.md](.claude/rules/structure.md)(**rules 本体は不変条件だけ。各論は `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行で追記** — `.claude/rules/` 配下は再帰スキャンで常時ロードされるため参照ファイルを置かない)/ 検証手順・動詞 → verify-ui スキル / 機械で守れる禁止事項 → `.claude/settings.json` の hooks(実体は `cargo xtask hook`)
- コードから読み取れるアーキテクチャ説明は書かない。罠と決定事項のみ
- **バージョン番号をハードコードしない** — 依存は Cargo.toml / ロックファイル、製品要件は [実装計画.md](internal-docs/実装計画.md) が正(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)は本ファイルに置かない**。索引だけを置き、実体は分割先へ
