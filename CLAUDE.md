# platitude-gg 開発規約

**platitude-gg** — 軽量・マルチプラットフォーム(Windows / macOS arm64 / Ubuntu。**Intel Mac は対象外**)の git GUI。機能要件・スコープ外の正本は [実装計画.md](internal-docs/実装計画.md) — 機能実装の前に該当セクションを読む。
本ファイルは全セッション共通の不変条件だけ。**規約本体は分割配置**: コード共通(Rust / QML)→ [.claude/rules/code.md](.claude/rules/code.md)、core → [.claude/rules/core.md](.claude/rules/core.md)、app → [.claude/rules/app-ui.md](.claude/rules/app-ui.md)、分割・共通化 → [.claude/rules/structure.md](.claude/rules/structure.md)(該当クレートに触れると自動ロード。**各論は `.claude/rules-refs/` の同名ファイル** — 触る項を Grep で引く)。動確・起動・`PGG_AUTO_ACT` 動詞表は **verify-ui スキル**、反映前テスト(gate / land / census)は [反映前テストの機械化.md](internal-docs/反映前テストの機械化.md)、依存の追加は [依存とライセンス.md](internal-docs/依存とライセンス.md)。

## 絶対制約(変更には人間の明示承認が必要)

- git に存在する機能は git に委譲する。**パスフレーズ・認証情報は agent / helper が自前の pinentry で聞く**
- 対応 git の最低バージョンは [git最低バージョン整合.md](internal-docs/git最低バージョン整合.md)。コードが相手にするのはその版以上だけ
- **依存(crate)と意匠素材の追加・変更は、ライセンス確認と人の許可が必須**。git 操作はシステム git のサブプロセス、通信は git コマンド経由のみ
- UI はダークテーマ(青系)のみ。文言は英語のみ・`qsTr()` 必須
- **内部コマンドと UI 表記は分ける** — 内部は最新 git の適切なコマンド(`switch` / `restore` 等)、UI 文言は「その操作が何をするか」で選ぶ。正本は [デザイン規約.md](internal-docs/デザイン規約.md) の用語表
- UI の値は `Theme.qml` / `Metrics.qml` のトークンのみ(値の正本はこの 2 ファイル、使い分けは [デザイン規約.md](internal-docs/デザイン規約.md))。トークンの追加・変更には人間の承認が必要

## 日本語の技術説明

- `arm` / `armed` の訳: タイマーは「セット／解除／再設定」、条件待ちは「待機開始／解除」、入力判定は「有効／無効」、エッジ検知は「再検知できる状態に戻す」。識別子・ログの `armed` は保持する

## 技術スタック

- Rust stable、UI は Qt Quick (QML)、ブリッジは **Qt Bridges**(`qtbridge` クレート、build.rs 不要)。Qt の必要バージョンは qtbridge の要求に従う
- **CXX-Qt へ差し替えられる構成を守る**(QML と core を無変更で移せる形。条件は core.md / app-ui.md)。core は Qt 依存ゼロ・`cargo test` で完結
- 開発補助ツールは **`cargo xtask`**(ワークスペース内クレート、**依存は std のみ**)で 3OS 同一に書き、OS 差はコード内の分岐に焼き込む。**スクリプト仕事も xtask のサブコマンド**・タスクランナーはこれ 1 本

## ビルド・テスト

前提: Qt は `.qt-version` の版(xtask は Windows で `C:\Qt\<版>` を選び、PATH の別の版では組まない)+ C++ ツールチェーン(他 OS の環境は `ci/` が正)。開発は debug ビルド。**`--release` は性能計測と起動確認だけ**(release でないと QML = exe 埋め込みが反映されない)。以下 `cargo` / `cargo xtask` を省略。

**確認は 3 段**: **1 日常** = <!--call:gate.daily-->`gate --host-only`(コンテナ無し)/ **2 反映前** = `gate`(差分の依存木と `verb-tiers.txt` の段で選んだテストを回し、緑を commit にスタンプ = `land` と git hook が要求)/ **3 フル** = `gate --all`(`periodic` のテストと全動詞を両 OS で回す。それ以外で回すのはユーザーがテストを指示した時だけ)+ <!--call:linux.bare-->`linux bare --discover` + 3OS CI + 性能実測(リリース前と依存・環境を触った時。**版(crate・Qt・最低 git)を動かす差分は `gate` 自身が段 3 の計画へ上がる**、残りは手で)。

**変更作業の完了には、依頼範囲の作業完了(§Git 運用)と、現在の commit に対する `gate` の PASS の両方が必要**。UI 配線では **触った動詞が両 OS で PASS し、両方の PNG を目視するまで**(verify-ui スキルを必ず呼ぶ)。

- **起動だけの要求(「rebase して起動」等)は fast path** — シェル呼び出し 1 個で起動して即報告し、ターンを終える(verify-ui スキル §起動 fast path)。段 2 はユーザーが検証・反映を指示した時
- **Linux での確認は `linux <コマンド>`**([ci/linux/Dockerfile](ci/linux/Dockerfile) のコンテナ = 最低 git バージョンを積んだ唯一の環境。Windows では WSL 3 の `wslc` が回す — Docker 不要。**`bare` は宣言した依存だけの Ubuntu で動くかだけを見る**)
- **書く作業は worktree 座席 `a`〜`f` で行う**(ドキュメントも含めて全部 — 本体 checkout は読むだけ)。席は `cargo xtask seat`(引数なし)が lock して返すので、それを EnterWorktree に渡す。未マージの席は続きの仕事専用。**claim は会話のもの — プロセスの死活は見ない**。claim を動かすのは 3 つだけ: **land が返す** / <!--cmd:seat.release-->`cargo xtask seat release` / **ユーザーがその場で指示した時だけ**の乗っ取り <!--cmd:seat.takeover-->`PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover <letter>`(対象は他人の claim だけ)。全席詰まりなら増設せず、持ち主を添えて報告して止まる。**完了後も席に留まる**
- **依頼範囲の作業はこのセッションで完了する**。チップは別セッションでしかできない作業だけ(触っているファイルを claim するチップは hook が deny)。継続して参照する制約・要判断は `internal-docs/P<n>-確認事項.md` へ記録する
- **worktree からのアプリ起動は headless(`verify-ui`)だけ**。実ウィンドウはユーザーが明示した時だけ <!--cmd:app.launch-->`PGG_ALLOW_GUI=1 cargo xtask launch`。自ツリーの残存プロセス・exe の使用中状態は `cargo xtask kill`。**窓のビルドがどのツリーのものかは右下が名乗る**

## 性能予算

`JetBrains/kotlin` 級(10万コミット超・refs 5 万本、うちタグ 4.5 万)で: 起動→グラフ初回表示 3 秒以内 / 操作応答 100ms / スクロール 60fps / メモリ 300MB 以下。UI 操作の経路の同期処理はコミット数・refs の本数から独立 — 遅延読み込みと差分更新が基本。ref 同士の突き合わせは索引を 1 本作ってから回す(`session::RefJoins` / `refs::RemoteBranches`)。未着手の候補は [非同期化の候補.md](internal-docs/非同期化の候補.md)。

## Git 運用

- コミットは Conventional Commits(`feat:` / `fix:` / `refactor:` / `docs:` / `test:` / `chore:`)で、メッセージは英語。push は追加のみ
- **依存(crate・Qt・toolchain)の更新は、追従の改善まで 1 回の land に入れる** — その land の前後を `land` が `deps/<日付>/before`・`after` のタグで挟んで push する(範囲の印はこの 2 本だけ。[反映前テストの機械化.md](internal-docs/反映前テストの機械化.md) §版を動かした land のタグ)
- **rebase はその場でユーザーが指示した時だけ**(main への追従・squash を含む。その時だけ `PGG_ALLOW_REBASE=1` を先頭に付ける)。worktree ブランチが main より遅れたままは正常
- **完了報告・反映の前に、依頼範囲の修正・レビュー対応・docs 更新を完了して commit する**(バックグラウンドのレビュー・検証も結果を受け取って対応まで)。外部確認・判断が得られなければ阻害要因を報告し、範囲を減らすならユーザーと合意する
- **main を動かすのはその場でユーザーが指示した時だけ**: <!--cmd:land.branch-->`cargo xtask land <branch>`(席で rebase → gate → fast-forward)。指示がなければ席で `gate` を通し、branch/SHA と検証結果を添えて「マージ可」と報告する。**許可は発話 1 回につき main が動く land 1 回**で、main が動いて初めて消える(判定は [land の許可](internal-docs/反映前テストの機械化.md#land-の許可permit))
- **確認の要る UI 挙動変更が入るなら反映を中止し、board で承諾を得る**(反映の指示より後に絵を載せた席の `land` は、絵を下げてもユーザーの次の発言まで止まる)。**land 後に絵を載せ直さない**(絵は claim を戻さない)
- **land は自分のセッションの branch(作った物・アプリが起動用に作った物)とそれを出す席以外の worktree をゴミとして残さない** — 持ち主は hook が記録し(reference-transaction / SessionStart)、land は中身を main が持つ物を消して残りを名指す。使用中で消えない起動用 worktree は手放し、後の land(誰のでも)が消す。他セッション・アプリ・ユーザーの物には触らない。名指された控え等は報告前に消し、main に無い作業を残すなら `keep/<名前>` にして確認事項へ置き場を書く

## 現在のフェーズ: **Phase 3 の操作まで配線済み(未配線の操作なし)**

- **配線済み操作の一覧・意匠決定・実装対応は [.claude/rules-refs/app-ui.md](.claude/rules-refs/app-ui.md) が正**。残作業と要判断は [P3-確認事項.md](internal-docs/P3-確認事項.md) — **UI 配線の前に必ず読む**。配布準備期の検証項目は [P5-確認事項.md](internal-docs/P5-確認事項.md) へ積む
- 性能 4 項目は net で予算内。**メモリだけ gross が予算の縁を越える**(正体は絵文字 1 文字が呼ぶフォントのフォールバック探索。数値と読み方は [実測記録](ci/baseline/perf-windows-x64.md))。改善の残件は P3-確認事項 §性能
- CI(3OS + 完全オフライン job、mac の撮影 `shots.yml`)は **main への push で走る**。runner の上で分かっている事と残りは P5-確認事項 §3.5 / §3.6。mac / Ubuntu は実機なし — 品質保証は 3OS CI のみ(mac の絵は verify-ui スキル mac.md)、実機検証は Phase 5 ゲート

## 規約の置き場所と本ファイルの運用

- ルール追加は「非自明・繰り返し発生・行動可能」を満たす場合のみ。**常時ロードされる文書には上限がある**(毎回の呼び出しが読み直す。**機械化済み: `cargo xtask docs` と post-write hook**): 本ファイル **12KB**、`.claude/rules/*.md` **各 8KB**。置き場所: 全セッション共通の不変条件 → 本ファイル / 触るファイルによらず効く規則 → `.claude/rules/`(参照ファイルを置かない = 常時ロード)/ **名前で引ける各論(型・部品・コマンド・関数・操作)→ `.claude/rules-refs/` の同名ファイルへ 1 項目 1 行** / 検証手順・動詞 → verify-ui スキル / 機械で守れる規則 → hooks(実体は xtask の <!--call:hook.event-->`hook <event>`、[反映前テストの機械化.md](internal-docs/反映前テストの機械化.md) §hook)
- 書くのは罠と決定事項だけ(アーキテクチャはコードが語る)。規約は行動と完了条件、hook は観測できる条件の強制。規約違反への対策は、既存の矛盾・重複・誤った誘導を先に削る
- **バージョン番号は正本から引く** — 依存は Cargo.toml / ロックファイル、製品要件は [実装計画.md](internal-docs/実装計画.md)
- **増え続けるもの(機能一覧・確認事項・実測値)は本ファイルには索引だけ**
