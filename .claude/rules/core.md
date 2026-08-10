---
paths:
  - "crates/platitude-core/**"
---

# platitude-core 規約(git サブプロセス・セッション実装)

core のファイルを読み書きすると自動ロードされる。常時必要な不変条件は CLAUDE.md、app / QML 側は `.claude/rules/app-ui.md`、検証手順は verify-ui スキル。

**コマンド別・モジュール別の各論(git の実測挙動・実装の決定事項)の正本は [rules-refs/core.md](../rules-refs/core.md)**(1 項目 1 行。自動ロードされない)。**書く前に、触るコマンド名・モジュール名・関数名で Grep して該当行と前後を読む** — 読まずに書くと、実測済みの罠をもう一度踏む。**実測・決定を足したら同ファイルへ 1 行で追記**(全セッション共通の不変条件に昇格するものだけ本ファイルへ)。

## git サブプロセス規約

- コマンドは**引数配列**で組み立てる。シェル文字列の連結禁止
- パースは機械可読形式のみ: `--porcelain=v2` / `-z`(NUL 区切り)/ `--format=` を常用。人間向け・ローカライズされ得る出力をパースしない
- 実行時の環境変数: `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`(プロンプトでハングさせない。認証は credential helper に委譲)、status 等の読み取り系ポーリングは `GIT_OPTIONAL_LOCKS=0`
- 失敗メッセージは stderr 優先・空なら stdout(`git commit` の「nothing to commit」は stdout に出て exit 1 する)
- 書き込みは**セッション単位のキューで直列化**する(ロックでは順序が保証されない — spawn したタスクが mutex を取る順は実行順と一致しない)
- **複数コマンドの合成は 1 手目が失敗したら止める**(`?` で伝播。途中まで進めた状態で次を撃たない)。落とし穴は**失敗が `Ok` に化ける経路** — `switch` の拒否(`CheckoutOutcome::Blocked`)と `stash pop` の非ゼロ終了は成功として返るので、そこだけは明示的に判定し、**戻せるものは戻す**(`session::carry_across` は tree を空にしても拒まれたら stash を pop で戻す)。失敗後に走ってよいのは読み取りだけ(`catch_up_after` の fetch)
- 対話エディタを開かせない(`GIT_EDITOR` / `GIT_SEQUENCE_EDITOR` を非対話に固定して rebase 等を駆動する)
- **ASCII のキーで前方一致する時も `str` を byte index で切らない**(`l[..KEY.len()]`)— 本文の文字がその 1 バイトを跨いだ瞬間に落ちる。`l.as_bytes().get(..KEY.len())` + `eq_ignore_ascii_case` にする(キーが ASCII なら意味は同じで、境界を気にする必要が消える)。**落ちた先が walk のワーカだと、グラフはローディングのリングのまま止まり、エラー表示にもコマンドログにも何も出ない** — 「サイドバーは出るのにグラフだけ空」はまず stderr のパニックを疑う(em dash / CJK の本文が全リポジトリのグラフを止めた実績: `parse::log` の co-author 除去)
- 全実行にタイムアウトとキャンセルを付ける。auto fetch は多重起動を防ぐ
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド
- **新しいオプションは最低 git バージョン([要望.md](../../internal-docs/要望.md))のマニュアルで存在確認してから使う** — 開発機の git は最新なので、`config get`(2.46)のように手元で動いて最低版に無いものが素通りする。照合の記録と手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md)(全発行コマンド照合済み)

## セッション・実装の決定事項

- **core の API は純 Rust 型のみ**(`String` / `Vec` / serde DTO)。`Rc<RefCell>` パターンや qtbridge の型を core に漏らさない。**core → UI の通知は core 定義の trait / チャネルで抽象化**し、app 側でブリッジ機構に接続する — ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件
- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- **コマンドログは executor の observer 1 本で取る**(`process::CommandObserver`。spawn は `execute` の 1 箇所)。セッションは同じ observer に**利用者用と背景用の 2 つのハンドル**を挿し(`GitExecutor::observed`)、書き込みキューだけが利用者用を使う = 分類がキューの分岐 1 箇所で決まる。**auto fetch はキューを通るが背景扱い**(オフラインで毎分パネルが開くのを防ぐ)。表示用文字列(`describe`)とコピー用の完全形(`-c` 群 + 環境変数まで)は別で、記録しない時は `records()` で早期に降りて組み立てない
- **終了コードで答える問い合わせはコマンドログの失敗にしない**(`GitCommand::answers_by_code()`。`merge-base --is-ancestor` の exit 1 は答えなので、受け取っただけでログが飛び出さない)。**`run_unchecked` で非ゼロを分岐に使っている箇所は全部これが要る** — `sequencer::resolve` の `rev-parse --verify --quiet` は「最初のコミットに親は無い」を exit 1 で受けており、付け忘れると**根に届く squash / drop のたびにパネルが開いた**(実測・修正済み)
- **統合テストは 1 バイナリ**(`tests/it/` のモジュール。`cargo test` はバイナリを 1 つずつ走らせるので、`tests/` 直下に .rs を足すと別バイナリ = 直列実行とリンク 1 本分の後退。新しい統合テストは `it/` にモジュールとして足し `main.rs` へ登録)。部分実行は `cargo test -p platitude-core --test it <モジュール名>`
- **「もう起きない」を sleep で確かめない** — キューに乗った書き込みは前の write の refresh まで終わってから始まるので、静かな時間の長さは「止まった」と「遅い」を区別しない(`cargo test --workspace` の負荷で落ちる)。タイマは手で進めて、進めた先が受け取ったかどうかを見る(`RepoSession::auto_fetch_ticker`)
  - **待ちの上限も同じ** — 「待ち始めてから N 秒」は経過時間で止まったと決める形で、遅いだけの実行を落とす。**上限は沈黙に対して数える**(`session_integration::Patience`。イベントが 1 つ来るたび更新し、`OVERALL_BUDGET` だけを全体の歯止めに残す)= ハングの検出条件は「20 秒何も来ない」のまま、答えが返り続けている待ちだけが払わなくなる。数字を上げて凌ぐと**そのファイルの全テストが同じだけ検出を遅らせる**(実測: 12 件の直列書き込みが単体 4.7s / `--workspace` の負荷下では 1 件 ~2.5s = 20 秒で 8 件目。沈黙で数えた側は静かなセッションを 20.2s で捕まえる)
