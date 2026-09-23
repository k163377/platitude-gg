---
paths:
  - "crates/platitude-core/**"
---

# platitude-core 規約(git サブプロセス・セッション実装)

**各論(git の実測挙動・実装の決定事項・テストの型)の正本は [rules-refs/core.md](../rules-refs/core.md)**(1 項目 1 行。自動ロードされない)。**書く前に、触るコマンド名・モジュール名・関数名で Grep し、該当行だけを読む(前後は `-C 2`)**。実測・決定を足したら同ファイルへ 1 行(本ファイルへは全セッション共通の不変条件だけ)。

## git サブプロセス規約

- コマンドは**引数配列**で組み立て、パースは機械可読形式のみ(`--porcelain=v2` / `-z` / `--format=`)。環境は `LC_ALL=C`・`GIT_TERMINAL_PROMPT=0`・`GIT_EDITOR=true`(rebase -i は `GIT_SEQUENCE_EDITOR` = 非対話 helper)、読み取り系ポーリングは `GIT_OPTIONAL_LOCKS=0`。Windows は `CREATE_NO_WINDOW`
- 失敗メッセージは stderr 優先・空なら stdout(`git commit` の「nothing to commit」は stdout で exit 1)
- git の実行も完了待ちも常に UI(Qt)スレッドの外
- **複合操作は 1 手目が失敗したら止める**(`?`)。**失敗が `Ok` に化ける経路**(`switch` の `CheckoutOutcome::Blocked`・`stash pop` の非ゼロ終了)だけは明示的に判定し、戻せるものは戻す(`session::carry_across`)。失敗後に走ってよいのは読み取りだけ
- **`&str` をバイト長で切らない** — キーの前方一致は `as_bytes().get(..KEY.len())` + `eq_ignore_ascii_case`(`l[..KEY.len()]` は多バイト文字を跨いだ瞬間に落ち、walk のワーカで落ちるとグラフはリングのまま無言で止まる = 「サイドバーは出るのにグラフだけ空」はまず stderr のパニック)
- **実行の監督はレーンで分ける**(ユーザー決定 2026-08-30): **読み取りとネットワーク(fetch / push / ls-remote / clone)はタイムアウト + キャンセル、ローカル書き込みは完走まで待つ**(close も)。分類点は `operation::OperationKind::lane` の exhaustive match 1 箇所。auto fetch は多重起動しない
- **git プロセスは全部 1 組の実行枠 `process::Slots` を通る**(**枠 = 子プロセス 1 本**、spawn 直前に取り reap で返す)。優先度(`Priority` = 誰が待つか、executor のハンドル)・ペース(`Pace` = 何が速さを決めるか、コマンド)・ログ(`Kept`)・キャンセル・書き込み順序は別の概念。背景の読みと外部に paced されるコマンドは全体の 1/4 だけ = クリックの枠は常に空く(各論は rules-refs の `Slots` 項)
- **新しいオプションは最低 git バージョンのマニュアルで存在確認してから使う**(手元の git は最新なので素通りする。手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md))

## セッション・実装の決定事項

- **core の API は純 Rust 型のみ**(`String` / `Vec` / serde DTO。qtbridge の型は app 側に留まる)。承認済みの例外は `crate::Name`(`compact_str`)1 つだけで、条件は app が `as_str` / `&str` 比較 / `to_string` でだけ読むこと。**core → UI の通知は core 定義の trait / チャネル**で、app がブリッジへ繋ぐ — ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つ条件
- 書き込みは `RepoSession` のキュー経由で直列化し、成否どちらでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ
- **コマンドログは executor の observer 1 本**(`process::CommandObserver` + `process::Kept` の 3 ハンドル)。**分類はキューの分岐 1 箇所**(`session::write::run_write`): 利用者用 `Asked` / 背景の読み `Unasked` / 頼まれていない fetch `UnaskedUnlessItFails`(行になるのは git が断った時だけ)。**`Asked` は書き込みの中にしか無い**(`CommandStarted.operation` が常に `Some`)— app の「パネルを開けるのは操作の答えだけ」がこれに乗るので、外へ出すなら app の配線も一緒に決め直す
- **終了コードで答える問い合わせは `GitCommand::answers_by_code(<コード>)` をコマンドごとに名指す**(`run_unchecked` で非ゼロを分岐に使う箇所は全部。付け忘れるとその答えのたびにパネルが開く)
- **統合テストは 1 バイナリ** — `tests/it/` にモジュールとして足し `main.rs` へ登録する(pre-write hook が守る)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`

## 非同期・並行テストの実装方針

- **正しさは因果で待つ** — 完了イベント・join handle・ack・世代番号・最終状態のいずれかを本体が返し、テストはその後だけ assert する。イベントが lock / queue の途中で送られるなら owner の返却まで待つ。出さない経路には完了境界を足す(`RefreshTask` / `RefreshOutcome`)。タイマは手で進めて tick の ack を待つ
- **待つ述語は、待っている当の物だけが満たせる形にし、主張を決める最初の境界で止める** — 人向けレポートの部分一致は、見出しの同じ語で**その物が生まれる前に真になる**。到着は当人が言う(ack / 完了イベント / その行だけが持つ形)
- **「もう起きない」は完了後の件数・状態で証明する** — quiet は「止まった」と「遅い」を区別できない。完了境界を作れないなら実装の観測可能性を直す
- **競合の相手は手で目的の地点まで進める** — `yield_now` / sleep / 大量反復は推測。所有する future は `crate::wait::poll_once`(統合スイートは `support::wait::poll_once`)で最初の待ち地点に止め、`Pending` を確かめてから相手を解放する。spawn した相手には到着を言う境界を持たせる
- **時間の仕様は機能テストから外す** — タイマの間隔・順序は手回しの clock を挿した単体テストで固定し、統合テストは全 tick を手で進める。製品の clock も仮想時計で見る(`#[tokio::test(start_paused = true)]`)
- **`cargo xtask waits` が全テストコードと試験装置の本体を読む**(gate の常時ステップ。範囲は rules-refs)— 直接の sleep / `yield_now` / `Instant::now` / `Duration` 直書きの `timeout` / 答えを捨てた待ち / `bounded` 無しの silent wait を名指す。**残す時は文の上か同じ行に `// waits(<purpose>): <reason>`**(`paced` 再試行の間隔 / `ceiling` 診断だけの天井 / `measured` 判定に使わない時計 / `timed` 製品の実時間を下限で見る)。**何も覆わないマーカーは赤**。単体テストの backstop は `crate::wait::bounded`
- **専有(一時 repository / 設定 / port — CLAUDE.md Rust 規約)の所有権は原子的な作成成功で取る**。in-process の mutex は別バイナリ・別セッションを隔離しない。重複排除を主張する実装は single-flight にし、同時 miss と read 中の invalidation を再現して呼出回数も固定する(`Derived`)
- **外部設定は executor 単位で隔離する** — 一時 `GIT_CONFIG_GLOBAL` / `XDG_CONFIG_HOME` と `GIT_CONFIG_NOSYSTEM=1` を全 subprocess に渡す。process-global env は書き換えない(host config を読む test だけ raw executor を明示)
- **待ちの上限は失敗検出の backstop**(`Patience` = 進捗ごとに沈黙予算を更新 + livelock 用の全体上限)。**沈黙予算は無言の 1 手の上限でもある** — 負荷で 1 手は桁で伸びる。単独で秒の桁の手(機械の棚卸し等)は待ちに入れない(要れば mry の属性で差し替え、実物を読むテストは `periodic` で全体予算で待つ)。性能予算は benchmark / baseline で判定する
- **runner の終了コードは外まで通す**(pipe・整形・後続の先まで。並列起動は全 child を回収し、1 件でも非ゼロなら全体を非ゼロ)
