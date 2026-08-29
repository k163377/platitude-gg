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
- 全実行にタイムアウトとキャンセルを付ける。auto fetch は多重起動を防ぐ(**例外は統合テストの executor** — 負荷が壁時計を破るので stock timeout を外さず **suite の backstop(`OVERALL_BUDGET`)まで上げる**: セッション系は Patience が先に効き、executor を直接 await するテストでも wedged git がテスト名付きの赤になる。rules-refs/core.md の同項)
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド
- **新しいオプションは最低 git バージョンのマニュアルで存在確認してから使う** — 開発機の git は最新なので、`config get`(2.46)のように手元で動いて最低版に無いものが素通りする。照合の記録と手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md)

## セッション・実装の決定事項

- **core の API は純 Rust 型のみ**(`String` / `Vec` / serde DTO)。`Rc<RefCell>` パターンや qtbridge の型を core に漏らさない。**core → UI の通知は core 定義の trait / チャネルで抽象化**し、app 側でブリッジ機構に接続する — ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件
  - **承認済みの例外は `crate::Name`(`compact_str::CompactString`)1 つだけ**。**条件は「app が型を名指ししないこと」**(`as_str` / `&str` との `==` / `to_string` で読める範囲に限る)。**app に `compact_str` を依存として足さねばならなくなったら例外の前提が崩れている**(理由と実測は rules-refs/core.md の同項)
- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- **コマンドログは executor の observer 1 本で取る**(`process::CommandObserver`)。セッションは利用者用と背景用の 2 ハンドル(`GitExecutor::observed`)を挿し、書き込みキューだけが利用者用 = 分類はキューの分岐 1 箇所で決まる。**auto fetch はキューを通るが背景扱い**(オフラインで毎分パネルが開くのを防ぐ)。記録しない時は `records()` で早期に降り、コピー用の完全形(`-c` 群 + 環境変数)を組み立てない
- **終了コードで答える問い合わせはコマンドログの失敗にしない**(`GitCommand::answers_by_code(<答えのコード>)`。例: `merge-base --is-ancestor` の exit 1 は答え)。**答えのコードはコマンドごとに名指す** — 同じ数字が別のコマンドでは本物の失敗を意味する。**`run_unchecked` で非ゼロを分岐に使っている箇所は全部これが要る** — 付け忘れるとその答えのたびにパネルが開く(対象コマンドと答えのコードは rules-refs/core.md の `answers_by_code` 項)
- **統合テストは 1 バイナリ** — 新しい統合テストは `tests/it/` にモジュールとして足し `main.rs` へ登録する(`tests/` 直下に .rs を置かない — 理由は違反時に pre-write hook が届ける)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`

## 非同期・並行テストの実装方針

- **正しさは時間ではなく因果で待つ** — 完了イベント、join handle、ack、barrier、世代番号、最終状態のいずれかを本体が返し、テストは「要求した処理」が終わった後だけ assert する。イベントが reader / lock / queue の途中で送られるなら、それ自体を完了扱いせず owner の返却まで待つ。操作が成功してもイベントを出さない経路には明示的な完了境界を足す(`RefreshTask` / `RefreshOutcome` / `RemoteTagRefreshTask`)。タイマは手で進めて、その tick の ack を待つ(`RepoSession::auto_fetch_ticker`)
- **「もう起きない」を sleep / quiet window で証明しない** — quiet は「止まった」と「遅い」を区別できず、余剰性能が落ちた時だけ偽陽性になる。無変更・exactly-once・二重起動無しは、対象操作の完了後に件数または状態を読む。完了境界を作れない時はテストを先に弱めず、実装の観測可能性を直す
- **baseline は開始条件を列挙して待つ** — `Opened` は path を受理しただけで、その後の refs / status / log は未完了。snapshot event も reader 内から送られ、その後に graph refresh を要求し得る。測定対象に先行処理を混ぜないよう `opening_snapshots` → `wait_for_snapshot_reads` → tracked graph refresh → **追い出した pass の停止**(`wait_for_graph_passes` — 要求は前の pass を cancel するだけで、cancel は止まった証拠ではない)→ 着地した pass の順で閉じる(`CaptureSink::opened_graph_gen`)
- **並行実行を既定として設計する** — `--test-threads` を下げない・serial 化で隠さない。各テストは専用の一時 repository / 設定 / socket を持ち、固定 port、共有ファイル名、process-global の可変状態を避ける。PID/時刻を名前に足すだけでなく原子的な作成成功を所有権にする。in-process の mutex は別テストバイナリ・別セッションを隔離しない。重複排除を主張する実装は single-flight にし、同時 miss と read 中の invalidation を barrier / channel で再現して呼出回数も固定する(`Derived`)
- **外部設定は executor 単位で隔離する** — integration test の Git は一時 `GIT_CONFIG_GLOBAL` / `XDG_CONFIG_HOME` と `GIT_CONFIG_NOSYSTEM=1` を全 subprocess に渡し、利用者の identity・ignore・hook・system config を読まない。process-global env の書換えは並行 test と競合するため使わず、意図的に host config を読む test だけ raw executor を明示する
- **待ちの上限は失敗検出の backstop** — 「開始から N 秒以内」を正しさや性能の assert にしない。通知待ちと同時に backstop を arm し(通知が止まっても永久待機しない)、進捗イベントごとに沈黙予算を更新し、livelock 用の全体上限だけ別に残す(`Patience`)。性能予算は専用 benchmark / baseline で判定し、機能テストの狭い timeout と混ぜない
- **runner の終了コードを失わない** — pipe、ログ整形、後続の `echo` 等で test process の非ゼロ終了を成功へ上書きしない。並列起動時は全 child の終了を回収し、1 件でも非ゼロなら全体を非ゼロにする
- **日常開発を flaky campaign にしない** — 通常の変更は関連 test を既定並列度で 1 回と通常の Done ゲートで確認し、10 回反復を課さない。偶発的な赤を再実行の緑で打ち消さず、再現条件と失敗を残して、当該変更で直すか専用 cleanup task に切り出す
- **本格的な flaky cleanup task だけ反復 campaign を行う** — 対象を再現に必要な最小 suite に絞り、観測された OS・runner の並走・負荷形態を含める。修正した実装ごとに 0 から数え、最低 10 回連続 OK。途中の NG は原因を直してから新しい実装として数え直す。全 workspace / 段 2 は最後に 1 回でよく、同じ全量 suite を 10 回回さない
