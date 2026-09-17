# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu)の git GUI。**Intel Mac は対象外**。
機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に該当セクションを読む(性能要件は本ファイル §性能予算)。
本ファイルは全セッション共通の不変条件だけ。**規約本体は分割配置**: core → [.claude/rules/core.md](.claude/rules/core.md)、app → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)、分割・共通化 → [.claude/rules/structure.md](.claude/rules/structure.md)(該当クレートに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — 触る項を Grep で引く)。動確・起動・`PGG_AUTO_ACT` 動詞表は **verify-ui スキル**、反映前テスト(gate / land / census)は [反映前テストの機械化.md](internal-docs/反映前テストの機械化.md)、依存の追加は [依存とライセンス.md](internal-docs/依存とライセンス.md)。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲する。**パスフレーズ・認証情報は agent / helper が自前の pinentry で聞く**
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md) に従う。コードが相手にするのはその版以上だけ
- **依存(crate)と意匠素材の追加・変更は、ライセンス確認と人の許可が必須**。git 操作はシステム git のサブプロセス、通信は git コマンド経由のみ。許可集合・例外・手順は [依存とライセンス.md](internal-docs/依存とライセンス.md)
- UI はダークテーマ(青系)のみ。文言は英語のみ・`qsTr()` 必須
- **内部コマンドと UI 表記は分ける** — 内部は最新 git の適切なコマンド(`switch` / `restore` 等)、UI 文言は「その操作が何をするか」で選ぶ。正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表・§git 用語のコード表記
- UI の値は `Theme.qml` / `Metrics.qml` のトークンのみ(**値の正本はこの 2 ファイル**。なぜその値かはトークンの隣に書く)。使い分けは [デザイン規約.md](internal-docs/デザイン規約.md) の該当 § を引き、**値はそのまま使う**(表の値の列はソースからの引用 — <!--cmd:docs.sync-->`cargo xtask docs --sync` が書く)。数値・色・フォント名はトークンで。トークンの追加・変更には人間の承認が必要

## 日本語の技術説明

- 会話・報告・docs の `arm` / `armed` 等は後述のように訳す: タイマーは「セット／解除／再設定」、条件待ちは「待機開始／解除」、入力判定は「有効／無効」、エッジ検知は「再検知できる状態に戻す」。待機中と実行中を区別し、識別子・ログの `armed` は保持する。

## 技術スタック

- Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う。依存・バージョンは Cargo.toml が正
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結
- 開発補助ツールは **`cargo xtask`**(ワークスペース内クレート + `.cargo/config.toml` の alias、**依存は std のみ**)で 3OS 同一に書き、OS 差はコード内の分岐に焼き込む。**スクリプト仕事も xtask のサブコマンド**・タスクランナーはこれ 1 本

## ビルド・テスト

前提: Qt(qtbridge の要求以上)+ C++ ツールチェーン、Qt の `bin`(`qmake`)が PATH に(Ubuntu のディストリ Qt は `QMAKE=qmake6`、macOS は `DYLD_FRAMEWORK_PATH` に Qt の `lib`)。
開発は debug ビルド。**`--release` は性能計測と起動確認だけ**だが、release でないと QML(exe 埋め込み)が反映されない。以下 `cargo` / `cargo xtask` を省略。

**確認は 3 段**: **1 日常** = <!--call:gate.daily-->`gate --host-only`(コンテナ無し)/ **2 反映前** = `gate`(差分の依存木からテストを機械が選んで両 OS で回し、緑を commit にスタンプする — `land` と git hook がそのスタンプを要求)/ **3 フル** = `gate --all` + <!--call:linux.bare-->`linux bare --discover` + 3OS CI + 性能実測(リリース前と依存・環境を触った時)。

**変更作業の完了には、依頼範囲の作業完了(§Git 運用)と、現在の commit に対する `gate` の PASS の両方が必要**。UI 配線では **gate が選んだ動詞が両 OS で PASS し、両方の PNG を目視するまで**(手順・動詞表・罠は verify-ui スキルを必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — シェル呼び出し 1 個で起動して即報告し、ターンを終える(手順と罠は verify-ui スキル §起動 fast path)。Done の基準は不変で、段 2 はユーザーが検証・反映を指示した時に走らせる
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ。最低 git バージョンを積んだ唯一の環境。**`bare` は宣言した依存だけの Ubuntu で動くかだけを見る**)
- **書く作業は worktree 座席 `a`〜`f` で行う**(ドキュメントも含めて全部 — 本体 checkout は読むだけ)。**席は与えられる**: `cargo xtask seat`(引数なし)が空き席を lock して letter と path を返すので、それを EnterWorktree に渡す(`seats` は状況を読む道具で、座る判断には使わない)。未マージの席は続きの仕事専用。全席詰まりなら増設せず報告。**席は land が返す** — main へ反映した時点で letter は roster に戻り、そのまま作業を続ければ次の編集で claim が戻る(取られていたら `cargo xtask seat` をもう一度)。land 以外の終わり方で返すのは <!--cmd:seat.release-->`cargo xtask seat release`。寝落ちの SessionEnd では claim は残る。**完了後も席に留まる**(main は §Git 運用)
- **依頼範囲の作業はこのセッションで完了する**。完了に数えるのは作業だけ。チップは別セッションでしかできない作業だけ。継続して参照する制約・要判断は `internal-docs/P<n>-確認事項.md` へ記録する。触っているファイルを claim するチップは `hook pre-chip` が deny する
- **worktree からのアプリ起動は headless(`verify-ui`)だけ**。実ウィンドウはユーザーが明示した時だけ <!--cmd:app.launch-->`PGG_ALLOW_GUI=1 cargo xtask launch`。自ツリーの終了後も残るアプリプロセスや exe の使用中状態は `cargo xtask kill` で解消する(対象は自ツリーの残存プロセスのみ)。**窓のビルドがどのツリーのものかは右下が名乗る**

## Rust 規約

- `unwrap()` / `expect()` / `panic!` はテストだけ。production の `Result` は全部 `?` か分岐で受ける
- エラーは core が `thiserror` で型付き、app は `anyhow` 可
- lint の抑止は `#[expect(...)]` が既定
- `unsafe` はやむを得ない箇所だけ、`// SAFETY:` コメント付きで
- 識別子・コメント・ログ・コミットメッセージは英語(docs は日本語可)。ログは `tracing` のみ
- **コメントは現在形の制約と罠だけ**(`.rs` / `.qml` 共通)— 帰属・経緯・履歴は git log の側
- **コメントの実測値は機械とロードに揺れない物だけ** — 時間・メモリ・スループット・機材の構成(画面 Hz・アダプタ・コア数)は、機械が変われば無効になるのにコードは動かないもののふりをする。**コメントに書くのは支配項と桁**(「プロセスが代金」「桁で大きい」「予算の外」)で、**数字が要るなら [ci/baseline/code-costs-windows-x64.md](ci/baseline/code-costs-windows-x64.md) へ 1 行足して、コメントはそこを指す**。**環境を名指した幾何・字送りの実測は残してよい**(フォント名・OS・窓幅を書いたもの = 再現できる制約)
- スナップショット(insta)は再生成して差分をレビューする
- **テストは並行実行が既定** — 既定の `--test-threads` で通る: temp path / port / 設定名は専用、可変状態はテスト内に閉じる(待ち方と反復判定は core.md §非同期・並行テスト)
- パーサのテストは実 git の出力を fixture に。git 実行系は一時ディレクトリに実リポジトリを作る統合テストで検証する
- **git の挙動に確信が無ければ、実装前に使い捨てリポジトリで実測する**(`tests/it/support` の `TestRepo`)。観測をテストへ固定してから実装。動確で壊れたら、直す前に再現テストが赤になるのを確認する

## 性能予算

`JetBrains/kotlin` 級(10万コミット超・refs 5 万本、うちタグ 4.5 万)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。
UI 操作の経路の同期処理はコミット数・refs の本数から独立 — 遅延読み込みと差分更新が基本。ref 同士の突き合わせは索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`。[refs-join 実測](ci/baseline/refs-join-windows-x64.md))。未着手の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)。push は追加のみ
- **rebase はその場でユーザーが指示した時だけ**(main への追従・squash を含む。指示があった時だけ `PGG_ALLOW_REBASE=1` を先頭に付ける)。worktree ブランチが main より遅れたままは正常
- **変更作業の完了報告・反映の前に、依頼範囲の修正・レビュー対応・必要な docs 更新を完了して commit する**。レビュー・検証はバックグラウンド分も結果を受け取り、指摘への対応と再確認まで終える。必要な外部確認・判断が得られなければ、その阻害要因を報告する。範囲を減らす場合はユーザーと合意する
- **main を動かすのはその場でユーザーが指示した時だけ**。完了確認後に <!--cmd:land.branch-->`cargo xtask land <branch>`(席で rebase → gate → fast-forward)。この場合の最終 gate は land に任せる。反映の指示がなければ席で `cargo xtask gate` を通し、branch/SHA と検証結果を添えて「マージ可」と報告する。**許可は発話 1 回につき main が動く land 1 回**。許可は main が動いて初めて消える。判定と拒否時の案内は [land の許可](internal-docs/反映前テストの機械化.md#land-の許可permit)

## 現在のフェーズ: **Phase 3 の操作まで配線済み(未配線の操作なし)**

- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**。残作業と要判断は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読む**。配布準備期の検証項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- 性能 4 項目のうち**メモリが予算超過**(判定行の max で数 MB。正体は絵文字 1 文字が呼ぶフォントのフォールバック探索)。数値・計測条件・読み方は [実測記録](ci/baseline/perf-windows-x64.md) が正(計測対象は `cargo xtask corpus` が生成する合成リポジトリ。実行手順は同記録 §計測条件)。改善の残件は P3-確認事項 §app
- CI(3OS + 完全オフライン job)は記述済み・**未実行(push で走る)**。初回検証と軽量 / 完全性の分割は [P5-確認事項.md](internal-docs/P5-確認事項.md) §3.5、ネットワーク非通信の証明は [実装計画.md](internal-docs/実装計画.md) §11
- mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。置き場所は冒頭の索引のとおり: 全セッション共通の不変条件 → 本ファイル(**15KB 以下を維持**)/ core・app・分割 → `.claude/rules/`(**本体は不変条件だけ。各論は `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行で追記** — `.claude/rules/` 配下は再帰スキャンで常時ロードされるため参照ファイルを置かない)/ 検証手順・動詞 → verify-ui スキル / 機械で守れる規則 → `.claude/settings.json` の hooks(実体は xtask の <!--call:hook.event-->`hook <event>`。ビルド先は `--profile hooks` = 席のビルドロックを待たない。理由は [反映前テストの機械化.md](internal-docs/反映前テストの機械化.md) §hook)
- 書くのは罠と決定事項だけ(アーキテクチャはコードが語る)。**規約は行動と完了条件、hook は観測できる条件の強制、分割先は手順を担う**。規約違反への対策は、既存の矛盾・重複・誤った誘導を先に削る。足す確認は行動を要する形だけ
- **バージョン番号は正本から引く** — 依存は Cargo.toml / ロックファイル、製品要件は [実装計画.md](internal-docs/実装計画.md) が正(方針は「最新から開始」)
- **増え続けるもの(機能一覧・確認事項・実測値)は本ファイルには索引だけ**。実体は分割先へ
