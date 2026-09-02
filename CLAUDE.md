# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。**Intel Mac は対象外**。
機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に該当セクションを読む(性能要件は本ファイル §性能予算)。
本ファイルは全セッション共通の不変条件だけ。**規約本体は分割配置**: core → [.claude/rules/core.md](.claude/rules/core.md)、app → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)、分割・共通化 → [.claude/rules/structure.md](.claude/rules/structure.md)(該当クレートに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — 触る項を Grep で引く)、動確手順と `PG_AUTO_ACT` 動詞表は **verify-ui スキル**。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md) に従う。それ未満向けのフォールバックコードを書かない
- ライセンス: **同梱・再配布するものが全て著作権上クリアであること** — **フォント・アイコン・テーマ等の意匠素材も対象**(素材は許可集合がコード依存と違うので個別に人の承認)。アプリ本体は MIT。依存追加の許可集合は **`deny.toml` の `[licenses] allow` が正本**(Unicode-3.0 を含む。OR の枝に許可外が居ても許可枝を選べれば違反ではない)、**GPL 系禁止**(Qt 本体と qtbridge は LGPL-3.0-only — 承認済みの例外)
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

**確認は 3 段**。**1 日常** = `cargo xtask gate --host-only`(コンテナ無し)/ **2 反映前** = `cargo xtask gate`(差分の依存木から機械が選ぶ。ホストの clippy は `cfg(not(windows))` の中を見ないので linux 側も回る)/ **3 フル** = 完全性 CI = `gate --all` + `linux bare --discover` + 3OS CI + 性能実測(リリース前と依存・環境を触った時)。**gate の緑は commit にスタンプされ、`land` と git hook が要求する**([反映前テストの機械化.md](internal-docs/反映前テストの機械化.md))。

**Done の基準**: `cargo xtask gate` の PASS。UI 配線の Done は **gate が選んだ動詞が両 OS で PASS し、両方の PNG を目視するまで**(手順・動詞表・罠は **verify-ui スキル**を必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — シェル呼び出し 1 個で起動し、即報告してターンを終える。**起動したら監視しない**(背景タスク・生存確認・撃ち直しを後ろに吊らない — ターンが終わらない間ユーザーの次の指示は届かない)。**`launch` をパイプ・コマンド置換に通さない**(窓の寿命だけターンが返らない。hook が deny)。Done の基準は不変で、段 2 はユーザーが検証・反映を指示した時に走らせる。手順は verify-ui スキル §起動 fast path
- **テストは thread / process の並行実行が既定** — `--test-threads` を下げて通さず、固定 temp path / port / 設定名や process-global 可変状態を共有しない。非同期テストの因果的な待ち方と反復判定は core 規約 §非同期・並行テストの実装方針
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ。Linux ではその場で実行 — `bare` だけは常にコンテナ)。イメージは core / app / runtime(**宣言した依存だけ**)から自動選択、ビルド先は docker volume。最低 git バージョンを積んだ唯一の環境。**`bare` は建てた場所の外で動くかだけを見る**(実測は P5-確認事項 §実測済み)
- **書く作業は worktree 座席 `a`〜`f` で行う**(ドキュメントも含めて全部 — 本体 checkout は読むだけ。`target/` と release exe の取り合いを避ける)。**席は選ばない・与えられる**: 本体 checkout への編集は hook が止め、答えは `cargo xtask seat` 一つ(**引数なし** — 空き席を lock してから letter と path を返す。マージ済みなら main の先頭に揃えるところまでやる)。**返った path を EnterWorktree に渡すだけ**で、それ以外の席への入席は hook が deny する。**空きを調べてから選ばない** — スナップショットは 2 セッションが同じに読めてしまい、決めているのは常に lock の方(実際に席 e で同席事故)。`cargo xtask seats` は状況を読む道具で、座る判断には使わない。claim は session id と Claude の pid を持ち、**プロセスが消えた claim は自動で回収**(手動 `git worktree unlock` は要らない)。land 成功で claim は返り、以後の追加作業は席内への最初の編集で再 claim。未マージの席は続きの仕事以外触らない — 全席詰まりなら増設せず報告。**完了しても main へは戻さない**(§Git 運用)
- **worktree からのアプリ起動は headless(`cargo xtask verify-ui`)だけ**。実ウィンドウはユーザーが明示した時だけ **`PG_ALLOW_GUI=1 cargo xtask launch`**(自ツリーの居残り回収→ビルド→起動→生存確認まで一括)。exe が掴まれている・二重起動ゲートが出た時は **`cargo xtask kill`** — 原因は常に自ツリーの居残りで、これはそれだけを落とす(**画像名 kill は他席とユーザーの窓を巻き込むので hook が deny**)。本体 checkout からの起動は対象外(ユーザー自身の起動)。**窓のビルドがどのツリーのものかは右下が名乗る**(実装は rules-refs/app-ui.md)
- 開発補助ツールを **.ps1 / .bat で作らない** — タスクランナーは `cargo xtask` パターン(ワークスペース内クレート + `.cargo/config.toml` の alias、依存は std のみ)で 3OS 同一に書き、OS 差はコード内の分岐に焼き込む。just / make 等の外部タスクランナーも導入しない

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 識別子・コメント・ログ・コミットメッセージは英語(docs は日本語可)。ログは `tracing`(`println!` / `eprintln!` 禁止)
- **コメントは現在形の制約と罠だけ**(`.rs` / `.qml` 共通)— 日付付きの帰属注記(「2026-xx-xx ユーザー指示」等)・決定経緯・変更履歴を書かない
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
- **rebase はその場でユーザーが指示した時だけ**(main への追従・squash を含む)。worktree ブランチが main より遅れたままは正常(例外は座席のマージ済みブランチの `reset --hard main`)。`hook pre-shell` が deny(`--abort` / `--quit` は除く)— 指示があった時だけ `PG_ALLOW_REBASE=1` を先頭に付ける。`land` 内部の rebase は反映指示が根拠
- **main を動かすのもその場でユーザーが指示した時だけ**。セッションは `worktree-<席>` に積んだまま「マージ可」と報告して終わる。**反映の指示を受けたら即 `PG_ALLOW_MAIN=1 cargo xtask land <branch>`**(fast path と同格。land 自身が席で rebase → gate → fast-forward)。land はどのセッションからでも動き、本体 checkout の HEAD がどこに居ても安全な手を選ぶ(worktree セッションの git は自ツリーに隔離され、手動 `git merge` は本体に届かない。`branch -f` / `update-ref` の手動反映は本体の index を置き去りにする既知の罠)。hook が main を書く git を deny して land へ誘導し、git hook がスタンプ無しの commit への main 更新を拒む(`PG_GATE_SKIP=1` はユーザー専用)
- **本体 checkout への直コミットはしない** — ドキュメントも設定も規約も、**その場でユーザーが main への直接の変更を許可したケース以外は全部席を取って進める**(worktree に積んで反映指示を待つ)。本体 checkout に残るのは読むことだけ

## 現在のフェーズ: **Phase 3 の操作まで配線済み(未配線の操作なし)**

- 性能 4 項目のうち**メモリだけが予算超過**(296–335MB / 予算 300MB。max で超過・min は予算内): [実測](ci/baseline/perf-windows-x64.md)。漏れではなく行を組んだ C++ ヒープが返らない形 — 残件は [P3-確認事項.md](internal-docs/P3-確認事項.md) §app。**計測対象は `cargo xtask corpus` が生成する合成リポジトリ**(生きた clone は fetch で前提が変わるのでやめた)。**この記録は 3 run の仮計測で、内訳・settle・出荷ビルド比は未取得**。**計測は画面・GPU アダプタ・corpus を機械が固定し、駄目な run は落として撮り直す**(条件と読み方、kotlin との形の差は実測記録)
- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**。本ファイルは未配線だけを持つ — **未配線の操作は無い**(フル interactive rebase のプラン編集モードまで配線済み。磨き残しは [P3-確認事項.md](internal-docs/P3-確認事項.md))
- 残作業と要判断事項は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読む**。配布準備期の検証項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- CI(3OS + 完全オフライン job)は記述済み・**push するまで実行しない**。初回検証は配布準備期(P5)。CI も軽量(段 2)/ 完全性(段 3)に分ける([P5-確認事項.md](internal-docs/P5-確認事項.md) §3.5)
- ネットワーク非通信の baseline: [windows-x64](ci/baseline/windows-x64.md)(主張の立て方は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3)
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持**)/ core → [.claude/rules/core.md](.claude/rules/core.md) / app・QML → [.claude/rules/app-ui.md](.claude/rules/app-ui.md) / 分割・共通化(全クレート) → [.claude/rules/structure.md](.claude/rules/structure.md)(**rules 本体は不変条件だけ。各論は `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行で追記** — `.claude/rules/` 配下は再帰スキャンで常時ロードされるため参照ファイルを置かない)/ 検証手順・動詞 → verify-ui スキル / 機械で守れる禁止事項 → `.claude/settings.json` の hooks(実体は `cargo xtask hook`)
- コードから読み取れるアーキテクチャ説明は書かない。罠と決定事項のみ
- **バージョン番号をハードコードしない** — 依存は Cargo.toml / ロックファイル、製品要件は [実装計画.md](internal-docs/実装計画.md) が正(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)は本ファイルに置かない**。索引だけを置き、実体は分割先へ
