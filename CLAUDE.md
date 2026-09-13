# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。**Intel Mac は対象外**。
機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に該当セクションを読む(性能要件は本ファイル §性能予算)。
本ファイルは全セッション共通の不変条件だけ。**規約本体は分割配置**: core → [.claude/rules/core.md](.claude/rules/core.md)、app → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)、分割・共通化 → [.claude/rules/structure.md](.claude/rules/structure.md)(該当クレートに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — 触る項を Grep で引く)。動確・起動・`PGG_AUTO_ACT` 動詞表は **verify-ui スキル**、反映前テスト(gate / land / census)は [反映前テストの機械化.md](internal-docs/反映前テストの機械化.md)、依存の追加は [依存とライセンス.md](internal-docs/依存とライセンス.md)。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲し、アプリ側で再実装しない。**パスフレーズ・認証情報をアプリが受け取らない**(agent / helper が自前の pinentry で聞く)
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md) に従う。それ未満向けのフォールバックコードを書かない
- **依存(crate)と意匠素材の追加・変更は、ライセンス確認と人の許可が必須**。git 実装とネットワーク通信の依存は禁止(git 操作はシステム git のサブプロセス、通信は git コマンド経由のみ)。許可集合・例外・手順は [依存とライセンス.md](internal-docs/依存とライセンス.md)
- UI はダークテーマ(青系)のみ。文言は英語のみ・ハードコード禁止(`qsTr()` 必須)
- **内部コマンドと UI 表記は分ける** — 内部は最新 git の適切なコマンド(`switch` / `restore` 等)、UI 文言は「その操作が何をするか」で選ぶ。正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表・§git 用語のコード表記
- UI の値は `Theme.qml` / `Metrics.qml` のトークンのみ(**値の正本はこの 2 ファイル**。なぜその値かはトークンの隣に書く)。使い分けは [デザイン規約.md](internal-docs/デザイン規約.md) の該当 § を引き、**数値を検討・微調整しない**(表の値の列はソースからの引用 — `cargo xtask docs --sync` が書く)。数値・色・フォント名の直書き禁止。トークンの追加・変更には人間の承認が必要

## 技術スタック

- Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う。依存・バージョンは Cargo.toml が正
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結
- 開発補助ツールは **`cargo xtask`**(ワークスペース内クレート + `.cargo/config.toml` の alias、**依存は std のみ**)で 3OS 同一に書き、OS 差はコード内の分岐に焼き込む。**.ps1 / .bat を作らない**・just / make 等の外部タスクランナーも導入しない

## ビルド・テスト

前提: Qt(qtbridge の要求以上)+ C++ ツールチェーン、Qt の `bin`(`qmake`)が PATH に(Ubuntu のディストリ Qt は `QMAKE=qmake6`、macOS は `DYLD_FRAMEWORK_PATH` に Qt の `lib`)。
開発は debug ビルド。**`--release` は性能計測と起動確認だけ**だが、release でないと QML(exe 埋め込み)が反映されない。以下 `cargo` / `cargo xtask` を省略。

**確認は 3 段**: **1 日常** = `gate --host-only`(コンテナ無し)/ **2 反映前** = `gate`(差分の依存木からテストを機械が選んで両 OS で回し、緑を commit にスタンプする — `land` と git hook がそのスタンプを要求)/ **3 フル** = `gate --all` + `linux bare --discover` + 3OS CI + 性能実測(リリース前と依存・環境を触った時)。

**Done の基準は `gate` の PASS**。UI 配線の Done は **gate が選んだ動詞が両 OS で PASS し、両方の PNG を目視するまで**(手順・動詞表・罠は verify-ui スキルを必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — シェル呼び出し 1 個で起動して即報告し、ターンを終える(監視しない。手順と罠は verify-ui スキル §起動 fast path)。Done の基準は不変で、段 2 はユーザーが検証・反映を指示した時に走らせる
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ。最低 git バージョンを積んだ唯一の環境。**`bare` は宣言した依存だけの Ubuntu で動くかだけを見る**)
- **書く作業は worktree 座席 `a`〜`f` で行う**(ドキュメントも含めて全部 — 本体 checkout は読むだけ)。**席は選ばない・与えられる**: `cargo xtask seat`(引数なし)が空き席を lock して letter と path を返すので、それを EnterWorktree に渡す(`seats` は状況を読む道具で、座る判断には使わない)。未マージの席は続きの仕事以外触らない。全席詰まりなら増設せず報告。**席は land が返す** — main へ反映した時点で letter は roster に戻り、そのまま作業を続ければ次の編集で claim が戻る(取られていたら `cargo xtask seat` をもう一度)。land しない終わり方で返すのは `cargo xtask seat release`。寝落ちの SessionEnd では claim は残る。**完了しても main へは戻さない**(§Git 運用)
- **残件はチップに逃がさない** — 検証・掃除・後追いの修正はこのセッションの仕事。**チップは別セッションでしかできない物だけ**(別マシン・実ウィンドウ・このセッションでは得られない判断)で、書き残すだけなら `internal-docs/P<n>-確認事項.md` へ。**触っているファイルを claim するチップは deny**(出口は `hook pre-chip` / `hook stop` の文)
- **worktree からのアプリ起動は headless(`verify-ui`)だけ**。実ウィンドウはユーザーが明示した時だけ `PGG_ALLOW_GUI=1 cargo xtask launch`。自ツリーの終了後も残るアプリプロセスや exe の使用中状態は `cargo xtask kill` で解消する(対象は自ツリーの残存プロセスのみ)。**窓のビルドがどのツリーのものかは右下が名乗る**

## Rust 規約

- production コードで `unwrap()` / `expect()` / `panic!` 禁止(テストは可)。`let _ =` で Result を捨てない
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- `#[expect(...)]` を `#[allow(...)]` より優先
- `unsafe` は原則禁止(やむを得ない場合は `// SAFETY:` コメント必須)
- 識別子・コメント・ログ・コミットメッセージは英語(docs は日本語可)。ログは `tracing`(`println!` / `eprintln!` 禁止)
- **コメントは現在形の制約と罠だけ**(`.rs` / `.qml` 共通)— 日付付きの帰属注記・決定経緯・変更履歴を書かない
- **機械やロードで揺れる実測値をコメントに書かない** — 時間・メモリ・スループット・機材の構成(画面 Hz・アダプタ・コア数)は、機械が変われば無効になるのにコードは動かないもののふりをする。**コメントに書くのは支配項と桁**(「プロセスが代金」「桁で大きい」「予算の外」)で、**数字が要るなら [ci/baseline/code-costs-windows-x64.md](ci/baseline/code-costs-windows-x64.md) へ 1 行足して、コメントはそこを指す**。**環境を名指した幾何・字送りの実測は残してよい**(フォント名・OS・窓幅を書いたもの = 再現できる制約)
- スナップショット(insta)の手編集禁止。再生成して差分をレビューする
- **テストは並行実行が既定** — `--test-threads` を下げず、固定 temp path / port / 設定名や process-global 可変状態を共有しない(待ち方と反復判定は core.md §非同期・並行テスト)
- パーサのテストは実 git の出力を fixture に。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する
- **git の挙動に確信が無ければ、実装前に使い捨てリポジトリで実測する**(`tests/it/support` の `TestRepo`)。観測をテストへ固定してから実装。動確で壊れたら、直す前に再現テストが赤になるのを確認する

## 性能予算

`JetBrains/kotlin` 級(10万コミット超・refs 5 万本、うちタグ 4.5 万)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
コミット数にも refs の本数にも比例する同期処理を UI 操作の経路に置かない — 遅延読み込みと差分更新が基本。ref 同士の突き合わせは索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`。[refs-join 実測](ci/baseline/refs-join-windows-x64.md))。未着手の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)。force push しない
- **rebase はその場でユーザーが指示した時だけ**(main への追従・squash を含む。指示があった時だけ `PGG_ALLOW_REBASE=1` を先頭に付ける)。worktree ブランチが main より遅れたままは正常
- **main を動かすのもその場でユーザーが指示した時だけ**。セッションは `worktree-<席>` に積んだまま「マージ可」と報告して終わる。**反映の指示を受けたら即 `PGG_ALLOW_MAIN=1 cargo xtask land <branch>`**(fast path と同格。land 自身が席で rebase → gate → fast-forward する。それ以外の main への書き込みは hook が止める)。**許可は発話 1 回 = land 1 回** — hook が「反映」を含むユーザー発話で開き、land 1 回で閉じ、次の発話で消える(反映され / 反映済 / 反映前 / 未反映 / 反映漏れ / 反映するな は開かない)。無ければ deny されるので「マージ可」で止まる
- **本体 checkout への直コミットはしない** — ドキュメントも設定も規約も、その場でユーザーが本体への直接の変更を許可したケース以外は全部席で進める

## 現在のフェーズ: **Phase 3 の操作まで配線済み(未配線の操作なし)**

- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**。残作業と要判断は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読む**。配布準備期の検証項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- 性能 4 項目のうち**メモリが予算超過**(判定行の max で数 MB。正体は絵文字 1 文字が呼ぶフォントのフォールバック探索)。数値・計測条件・読み方は [実測記録](ci/baseline/perf-windows-x64.md) が正(計測対象は `cargo xtask corpus` が生成する合成リポジトリ。実行手順は同記録 §計測条件)。改善の残件は P3-確認事項 §app
- CI(3OS + 完全オフライン job)は記述済み・**push するまで実行しない**。初回検証と軽量 / 完全性の分割は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3.5、ネットワーク非通信の証明は [実装計画.md](internal-docs/実装計画.md) §11
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所は冒頭の索引のとおり: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持**)/ core・app・分割 → `.claude/rules/`(**本体は不変条件だけ。各論は `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行で追記** — `.claude/rules/` 配下は再帰スキャンで常時ロードされるため参照ファイルを置かない)/ 検証手順・動詞 → verify-ui スキル / 機械で守れる禁止事項 → `.claude/settings.json` の hooks(実体は xtask の `hook <event>`。ビルド先は `--profile hooks` = 席のビルドロックを待たない。理由は [反映前テストの機械化.md](internal-docs/反映前テストの機械化.md) §hook)
- コードから読み取れるアーキテクチャ説明は書かない。罠と決定事項のみ。**規約は動詞を言い、手順は hook の deny 文と分割先が言う** — 同じ手順を本ファイルに写さない
- **バージョン番号をハードコードしない** — 依存は Cargo.toml / ロックファイル、製品要件は [実装計画.md](internal-docs/実装計画.md) が正(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)は本ファイルに置かない**。索引だけを置き、実体は分割先へ
