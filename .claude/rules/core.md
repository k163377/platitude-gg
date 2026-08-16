---
paths:
  - "crates/platitude-core/**"
---

# platitude-core 規約(git サブプロセス・セッション実装)

常時必要な不変条件は CLAUDE.md、app / QML 側は `.claude/rules/app-ui.md`、検証手順は verify-ui スキル。

**コマンド別・モジュール別の各論(git の実測挙動・実装の決定事項)の正本は [rules-refs/core.md](../rules-refs/core.md)**(1 項目 1 行。自動ロードされない)。**書く前に、触るコマンド名・モジュール名・関数名で Grep し、該当項だけを読む(前後は `-C 2`)。ファイルを Read で全読みしない**。**実測・決定を足したら同ファイルへ 1 行で追記**(全セッション共通の不変条件に昇格するものだけ本ファイルへ)。

## git サブプロセス規約

- コマンドは**引数配列**で組み立てる(シェル文字列の連結禁止)
- パースは機械可読形式のみ: `--porcelain=v2` / `-z`(NUL 区切り)/ `--format=` を常用。人間向け・ローカライズされ得る出力をパースしない
- 実行時の環境変数: `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`、status 等の読み取り系ポーリングは `GIT_OPTIONAL_LOCKS=0`
- 失敗メッセージは stderr 優先・空なら stdout(`git commit` の「nothing to commit」は stdout に出て exit 1 する)
- 書き込みは**セッション単位のキューで直列化**する(ロックでは順序が保証されない — spawn したタスクが mutex を取る順は実行順と一致しない)
- **複数コマンドの合成は 1 手目が失敗したら止める**(`?` で伝播。途中まで進めた状態で次を撃たない)。落とし穴は**失敗が `Ok` に化ける経路** — `switch` の拒否(`CheckoutOutcome::Blocked`)と `stash pop` の非ゼロ終了は成功として返るので、そこだけは明示的に判定し、**戻せるものは戻す**(`session::carry_across`)。失敗後に走ってよいのは読み取りだけ(`catch_up_after` の fetch)
- 対話エディタを開かせない(`GIT_EDITOR` / `GIT_SEQUENCE_EDITOR` を非対話に固定して rebase 等を駆動する)
- **ASCII のキーで前方一致する時も `str` を byte index で切らない**(`l[..KEY.len()]` は本文の文字がその 1 バイトを跨いだ瞬間に落ちる)。`l.as_bytes().get(..KEY.len())` + `eq_ignore_ascii_case` にする。**落ちた先が walk のワーカだと、グラフはローディングのリングのまま止まり、エラー表示にもコマンドログにも何も出ない** — 「サイドバーは出るのにグラフだけ空」はまず stderr のパニックを疑う
- 全実行にタイムアウトとキャンセルを付ける。auto fetch は多重起動を防ぐ
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド
- **新しいオプションは最低 git バージョンのマニュアルで存在確認してから使う** — 開発機の git は最新なので、`config get`(2.46)のように手元で動いて最低版に無いものが素通りする。照合の記録と手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md)

## セッション・実装の決定事項

- **core の API は純 Rust 型のみ**(`String` / `Vec` / serde DTO)。`Rc<RefCell>` パターンや qtbridge の型を core に漏らさない。**core → UI の通知は core 定義の trait / チャネルで抽象化**し、app 側でブリッジ機構に接続する — ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件
  - **承認済みの例外は `crate::Name`(`compact_str::CompactString`)1 つだけ**。**条件は「app が型を名指ししないこと」**(`as_str` / `&str` との `==` / `to_string` で読める範囲に限る)。**app に `compact_str` を依存として足さねばならなくなったら例外の前提が崩れている**(理由と実測は rules-refs/core.md の同項)
- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- **コマンドログは executor の observer 1 本で取る**(`process::CommandObserver`)。セッションは利用者用と背景用の 2 ハンドル(`GitExecutor::observed`)を挿し、書き込みキューだけが利用者用 = 分類はキューの分岐 1 箇所で決まる。**auto fetch はキューを通るが背景扱い**(オフラインで毎分パネルが開くのを防ぐ)。記録しない時は `records()` で早期に降り、コピー用の完全形(`-c` 群 + 環境変数)を組み立てない
- **終了コードで答える問い合わせはコマンドログの失敗にしない**(`GitCommand::answers_by_code()`。例: `merge-base --is-ancestor` の exit 1 は答え)。**`run_unchecked` で非ゼロを分岐に使っている箇所は全部これが要る** — 付け忘れるとその exit 1 のたびにパネルが開く(対象コマンドの一覧は rules-refs/core.md の `answers_by_code` 項)
- **統合テストは 1 バイナリ** — 新しい統合テストは `tests/it/` にモジュールとして足し `main.rs` へ登録する(`tests/` 直下に .rs を置かない — 理由は違反時に pre-write hook が届ける)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`

## 非同期・並行テストの実装方針

- **正しさは時間ではなく因果で待つ** — 完了イベント、join handle、ack、barrier、世代番号、最終状態のいずれかを本体が返し、テストは「要求した処理」が終わった後だけ assert する。操作が成功してもイベントを出さない経路には明示的な完了境界を足す(`RefreshTask` / `RefreshOutcome`)。タイマは手で進めて、その tick の ack を待つ(`RepoSession::auto_fetch_ticker`)
- **「もう起きない」を sleep / quiet window で証明しない** — quiet は「止まった」と「遅い」を区別できず、余剰性能が落ちた時だけ偽陽性になる。無変更・exactly-once・二重起動無しは、対象操作の完了後に件数または状態を読む。完了境界を作れない時はテストを先に弱めず、実装の観測可能性を直す
- **baseline は開始条件を列挙して待つ** — `Opened` は path を受理しただけで、その後の refs / status / log は未完了。測定対象に先行処理を混ぜないよう `opening_snapshots`、着地した pass、tracked refresh 等を待つ。`support::settled` は baseline 用の補助であり、個別操作の完了証明には使わない
- **並行実行を既定として設計する** — `--test-threads` を下げない・serial 化で隠さない。各テストは専用の一時 repository / 設定 / socket を持ち、固定 port、共有ファイル名、process-global の可変状態を避ける。in-process の mutex は別テストバイナリ・別セッションを隔離しない。重複排除を主張する実装は single-flight にし、同時 miss と read 中の invalidation を barrier / channel で再現して呼出回数も固定する(`Derived`)
- **待ちの上限は失敗検出の backstop** — 「開始から N 秒以内」を正しさや性能の assert にしない。進捗イベントごとに沈黙予算を更新し、livelock 用の全体上限だけ別に残す(`Patience`)。性能予算は専用 benchmark / baseline で判定し、機能テストの狭い timeout と混ぜない
- **runner の終了コードを失わない** — pipe、ログ整形、後続の `echo` 等で test process の非ゼロ終了を成功へ上書きしない。並列起動時は全 child の終了を回収し、1 件でも非ゼロなら全体を非ゼロにする
- **同期プリミティブ・待ち helper を追加または変更した時は並列で反復検証する** — 関連 suite を独立 process でも同時実行し、最低 10 回確認する。11 回目以降まで続いた場合は、最後の NG の後に 5 回連続 OK になるまで方針を OK にしない
