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
- 実行の監督は**レーンで分ける**(ユーザー決定 2026-08-30): **読み取りとネットワーク(fetch / push / ls-remote / clone)はタイムアウト+キャンセル**、**ローカル書き込みは完走まで待つ** — stock timeout を外し、close でも kill しない(遅さは仕事量に比例するだけでハングではなく、半端に殺された書き込みだけが依頼を失わせる。ユーザーの hook が固まる場合も殺さず、busy 表示とコマンドログが見せている状態で待つ)。分類点は `session::write::remote_paced` の 1 箇所。auto fetch は多重起動を防ぐ(**統合テストの executor** は stock timeout を外さず **suite の backstop(`OVERALL_BUDGET`)まで上げる**: セッション系は Patience が先に効き、executor を直接 await するテストでも wedged git がテスト名付きの赤になる — 書き込みレーンはテストでもキューが stock を外すので、待ちは常に Patience / `bounded` が名前で赤にする。rules-refs/core.md の同項)
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド
- **新しいオプションは最低 git バージョンのマニュアルで存在確認してから使う** — 開発機の git は最新なので、`config get`(2.46)のように手元で動いて最低版に無いものが素通りする。照合の記録と手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md)

## セッション・実装の決定事項

- **core の API は純 Rust 型のみ**(`String` / `Vec` / serde DTO)。`Rc<RefCell>` パターンや qtbridge の型を core に漏らさない。**core → UI の通知は core 定義の trait / チャネルで抽象化**し、app 側でブリッジ機構に接続する — ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件
  - **承認済みの例外は `crate::Name`(`compact_str::CompactString`)1 つだけ**。**条件は「app が型を名指ししないこと」**(`as_str` / `&str` との `==` / `to_string` で読める範囲に限る)。**app に `compact_str` を依存として足さねばならなくなったら例外の前提が崩れている**(理由と実測は rules-refs/core.md の同項)
- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- **コマンドログは executor の observer 1 本で取る**(`process::CommandObserver`)。セッションは `process::Kept` の 3 ハンドル(`GitExecutor::observed`)を挿し、**分類はキューの分岐 1 箇所**(`session::write::run_write`)で決まる: 利用者用(`Asked`)/ 背景の読み(`Unasked` — 記録しない時は `records()` で早期に降り、コピー用の完全形を組み立てない)/ **頼まれていない fetch**(`UnaskedUnlessItFails` — 着地したら行を残さず、git が断ったら行になる。**開いたパネルは、それを開けたコマンドを持っていなければならない**)。**行が「利用者の doing か」は行に付いて回る**(`CommandStarted.asked`)— 頼んでいない行は赤い印もパネルも動かさないので、オフラインでも毎分は開かない
- **終了コードで答える問い合わせはコマンドログの失敗にしない**(`GitCommand::answers_by_code(<答えのコード>)`。例: `merge-base --is-ancestor` の exit 1 は答え)。**答えのコードはコマンドごとに名指す** — 同じ数字が別のコマンドでは本物の失敗を意味する。**`run_unchecked` で非ゼロを分岐に使っている箇所は全部これが要る** — 付け忘れるとその答えのたびにパネルが開く(対象コマンドと答えのコードは rules-refs/core.md の `answers_by_code` 項)
- **統合テストは 1 バイナリ** — 新しい統合テストは `tests/it/` にモジュールとして足し `main.rs` へ登録する(`tests/` 直下に .rs を置かない — 理由は違反時に pre-write hook が届ける)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`

## 非同期・並行テストの実装方針

- **正しさは時間ではなく因果で待つ** — 完了イベント、join handle、ack、barrier、世代番号、最終状態のいずれかを本体が返し、テストは「要求した処理」が終わった後だけ assert する。イベントが reader / lock / queue の途中で送られるなら、それ自体を完了扱いせず owner の返却まで待つ。操作が成功してもイベントを出さない経路には明示的な完了境界を足す(`RefreshTask` / `RefreshOutcome` / `RemoteTagRefreshTask`)。タイマは手で進めて、その tick の ack を待つ(`RepoSession::auto_fetch_ticker`)
- **「もう起きない」を sleep / quiet window で証明しない** — quiet は「止まった」と「遅い」を区別できず、余剰性能が落ちた時だけ偽陽性になる。無変更・exactly-once・二重起動無しは、対象操作の完了後に件数または状態を読む。完了境界を作れない時はテストを先に弱めず、実装の観測可能性を直す
- **baseline は開始条件を列挙して待つ** — `Opened` は path を受理しただけで、その後の refs / status / log は未完了。snapshot event も reader 内から送られ、その後に graph refresh を要求し得る。測定対象に先行処理を混ぜないよう `opening_snapshots` → `wait_for_snapshot_reads` → tracked graph refresh → **追い出した pass の停止**(`wait_for_graph_passes` — 要求は前の pass を cancel するだけで、cancel は止まった証拠ではない)→ 着地した pass の順で閉じる(`CaptureSink::opened_graph_gen`)
- **競合の相手は手で目的の地点まで進める** — `yield_now` / sleep / 大量反復で「もう門に居るはず」を作らない。テストが所有する future は `crate::wait::poll_once`(統合スイートは双子の `support::wait::poll_once`)で 1 回だけ poll して最初の待ち地点(門・読みの中)で止め、`Pending` を確かめてから相手を解放する(`state_tests` / `read_flight` の tests、`session_integration` の diff の読み)。spawn した相手には本体が到着を言う境界を持たせる(`ReadFlight::wait_for_askers`)
- **Busy は「来るな」ではなく「その読みが終わった」で答える** — single-flight の slot に断られた ask の ack は、slot を持つ読みが手放してから送る(`RemoteTagRefreshOutcome::Busy`)。テストは sleep で撃ち直さず、Busy の ack を待ってから次を撃つ(競合相手の数だけ撃ち直し、その先は名前付きで赤 — `remote_tags_integration::asked_past_busy`)
- **時間の仕様は機能テストから外す** — タイマの「間隔が来たら 1 回・停止が overdue の tick に勝つ・最初の tick は間隔 1 つ先」は手回しの clock を挿した単体テストで固定する(`session::auto_fetch::drive_auto_fetch` / `AutoFetchClock`)。統合テストは全 tick を `auto_fetch_ticker` で手で進め、実時間の間隔を待たない。**製品の clock も実時間では見ない** — `#[tokio::test(start_paused = true)]` の仮想時計で間隔の両端を固定する(rules-refs/core.md の同項)
- **`cargo xtask waits` は全テストコードと「試験装置」の本体を読む**(gate の常時ステップ。統合スイート + support・各クレートの `#[cfg(test)]` 区画(`mod` / `fn` / `impl`、`all(test, …)` / `any(test, …)` も。`mod x;` は `#[path]` まで追う) / `*_tests.rs`・`tst_*.qml`。**試験装置 = xtask の `src` と app のハーネス両半分**(動詞の QML `src/auto` と、run を運転・観測する Rust `src/harness`)で、どれも本体まで読む — 名指す形は rules-refs。**製品本文は読まない**)— 直接の sleep / `yield_now`、`Instant::now`(`()` 無しの関数参照も) / `Duration` 直書きの `timeout`、答えを捨てた待ち(`let _ =`、QML の裸の `waitForRendering`)、`bounded` 無しの silent wait を名指す。**残す時は文の上か同じ行に `// waits(<purpose>): <reason>`**(purpose = `paced` 再試行の間隔 / `ceiling` 診断だけの天井 / `measured` 判定に使わない時計 / `timed` 製品の実時間を下限で見る)。**何も覆わないマーカーは赤**(直したら剥がす)。単体テストの backstop は `crate::wait::bounded`
- **並行実行を既定として設計する** — `--test-threads` を下げない・serial 化で隠さない。各テストは専用の一時 repository / 設定 / socket を持ち、固定 port、共有ファイル名、process-global の可変状態を避ける。PID/時刻を名前に足すだけでなく原子的な作成成功を所有権にする。in-process の mutex は別テストバイナリ・別セッションを隔離しない。重複排除を主張する実装は single-flight にし、同時 miss と read 中の invalidation を barrier / channel で再現して呼出回数も固定する(`Derived`)
- **外部設定は executor 単位で隔離する** — integration test の Git は一時 `GIT_CONFIG_GLOBAL` / `XDG_CONFIG_HOME` と `GIT_CONFIG_NOSYSTEM=1` を全 subprocess に渡し、利用者の identity・ignore・hook・system config を読まない。process-global env の書換えは並行 test と競合するため使わず、意図的に host config を読む test だけ raw executor を明示する
- **待ちの上限は失敗検出の backstop** — 「開始から N 秒以内」を正しさや性能の assert にしない。通知待ちと同時に backstop を arm し(通知が止まっても永久待機しない)、進捗イベントごとに沈黙予算を更新し、livelock 用の全体上限だけ別に残す(`Patience`)。性能予算は専用 benchmark / baseline で判定し、機能テストの狭い timeout と混ぜない
- **runner の終了コードを失わない** — pipe、ログ整形、後続の `echo` 等で test process の非ゼロ終了を成功へ上書きしない。並列起動時は全 child の終了を回収し、1 件でも非ゼロなら全体を非ゼロにする
- **日常開発を flaky campaign にしない** — 通常の変更は関連 test を既定並列度で 1 回と通常の Done ゲートで確認し、10 回反復を課さない。偶発的な赤を再実行の緑で打ち消さず、再現条件と失敗を残して、当該変更で直すか専用 cleanup task に切り出す
- **本格的な flaky cleanup task だけ反復 campaign を行う** — 対象を再現に必要な最小 suite に絞り、観測された OS・runner の並走・負荷形態を含める。修正した実装ごとに 0 から数え、最低 10 回連続 OK。途中の NG は原因を直してから新しい実装として数え直す。全 workspace / 段 2 は最後に 1 回でよく、同じ全量 suite を 10 回回さない
