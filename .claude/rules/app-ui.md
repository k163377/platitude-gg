---
paths:
  - "crates/platitude-app/**"
---

# platitude-app 規約(Qt Bridges・QML 配線・意匠の実装対応)

UI の**値**の正本は `Theme.qml` / `Metrics.qml`(値の隣に、なぜその値かが在る)。**どこで何に使うか**と用語は [デザイン規約.md](../../internal-docs/デザイン規約.md)(触る § だけを読み、トークンを表から選ぶ。表の値の列は <!--cmd:docs.sync-->`cargo xtask docs --sync` が書く)。**トークンは 3 層**(基礎 / ベーシック / 個別 — 規約 §トークンの三層)。**個別の層は名指しした画面だけが読む** — 別の画面が同じ寸法を欲しがったら、同じ数でも自分のトークンを足す。

**各論(意匠決定・実装対応・Qt / QML の罠・配線済み操作の一覧)の正本は [rules-refs/app-ui.md](../rules-refs/app-ui.md)**(1 項目 1 行。自動ロードされない)。**書く前に、触る部品名・操作名・動詞名・規約 §名と、使う Qt の型名・API 名(`HoverHandler` / `ListView` / `Flickable` / `palette` / `FontMetrics` / `grabToImage` …)で Grep し、該当行だけを読む**。決定・罠を足したら同ファイルへ 1 行(本ファイルへは全セッション共通の不変条件だけ)。

## Qt Bridges・QML の不変条件

- 全 QObject は `Rc<RefCell<_>>`(メインスレッド専有)。**QML からの呼び出し中に再入 borrow すると panic** — 借用は短く保つ
- バックグラウンド → UI は `QmlObject::get_qml_method_invoker()` の `QmlMethodInvoker` をスレッドへ移して `invoke_method*`(queued)。UI を触るのはこの経路だけ。invoker は Send・Clone 不可 — 渡す先の数だけ取る
- モデルの proxy(`try_get_rust_proxy_ptr()`)は `&*proxy` の共有参照でだけ触る — Qt が通知の最中に同じ proxy へ再入する(各論は rules-refs の `QListModelBase` の行)
- **橋を渡る値は `encode::wire` の形だけ**: レコードの列は `Listed<T>`(QML は欄を名前で読む)、欄の無い列は `Vec<String>` / `Vec<i32>`、1 件は `One<T>`、無いかもしれない 1 件は `Optional<T>`(無しは `undefined` — 番兵を作らない)。**`Optional<T>` は `qproperty!` に置けない**(プロセスごと落ちる。`PartialEq` を持たせないことで型が拒む。代わりは revision の property + slot)。**Qt の値は読む時に組む — 行の隣に持たない**(全行ぶんの Qt ヒープが常駐する)。各論は rules-refs の同項
- **QML は表示とインタラクションだけ**(データ加工は Rust)— CXX-Qt へ差し替え可能に保つ条件。**QML が値で問う純ルールは `GitFacts` singleton の stateless slot**(引数だけを読む = 「スロットはバインディングで固まる」の例外条件)。状態から導く派生値は drain で計算する qproperty
- **QML バインディングはプロパティにしか反応しない** — `#[qslot]` は呼び出し用。値の変化を追わせる物は `qproperty!`
- QML は `ui/` 直下フラットに 1 ファイル 1 コンポーネント(`platitude.ui`)。**新規 QML は qmldir と main.rs の `qrc::embed!` の両方へ登録**(列挙された型しか見えない。`include_bytes_qml!` は使わない — rules-refs の `qrc::embed!` の行)
- **QML モジュールは 2 つ — 製品 `ui/`(`platitude.ui`)と検証ハーネス `auto/`(`platitude.auto`)**。後者は Cargo feature `automation` でしか埋め込まれず(出荷ビルドには入らない)、**製品から `platitude.auto` の型は URL 越しだけ**(`HarnessSeat` に `seats` を渡し `ask()` で取り出す)。**走らせ方は `Harness` singleton が答え、製品は中立な property だけを持つ**。**Rust 側で `PGG_*` を読むのは `harness::knobs` だけ**(例外は `main.rs` の `PGG_LOG`)、core へは値で渡す。どれも**機械化済み: `cargo xtask structure`**
- **意匠の土台は既存の部品から選ぶ**(一覧は rules-refs の台座の行)。素の `Popup` / `ListView` / `RotationAnimator` / `elide` を新しく書かない — 台座に積んだ罠避けがまるごと落ちる。**3 箇所以上で言う同じ文・語は `Words`**
- 配線: データは親→子へ property、操作・状態変更は子→親へ signal。タブのモデル群は `RepoPage` が所有しペインへ渡す(ペインはモデルの slot を呼んでよいが、ページ状態は signal で上げて page が変更する)。ウィンドウ横断(タブを開く・settings・identity)は Main まで signal で上げる
- **メニューは `AppMenu` 系で書く**: 行の可否は開く関数で 1 度だけ決めて `offered` に置き、`offer()` で開き、閉じるのは自分の `dismiss()` 1 発(素の `Menu` を書かない・1 関数で閉じるのは 1 つまで = 機械化済み。各論は rules-refs の `AppMenu` / `offered` の行)
- **`onXChanged` は同じ `x` から導かれるバインディングより先に走り、`FontMetrics.advanceWidth()` 等のメソッドはバインディングが依存を取らない** — 測って押し出す値は、元のプロパティを自分で読み直す関数にする(各論は rules-refs)

## UI 自動化(`PGG_AUTO_ACT` 動詞)の不変条件

- **完了条件は動詞固有の因果だけ** — 入力の受理 → busy 開始 → 終了 → 対象モデルの更新 → 出力プロパティか描画可能状態。短い反復 Timer は sampler、`--watchdog-ms` は診断の天井で撮影時点を選ばない。性能測定の窓は測定入力で、完了は `perf_done` と親 watchdog の別勘定
- **1 run の完了 owner は 1 つ**(page が原子的に claim し、window-level 動詞は defer)。撮影は owner の `finishAutoAct()` 1 回の後だけ
- **run は自分の環境と状態を建てる** — 起動前に全 `PGG_AUTO_*` を除いてから自分の値だけを設定し、repo / config / shot は run 固有(明示した `--repo` / `--config-dir` / `--shot-dir` は原子的に所有、同時利用は fail fast)。worktree からの起動は親が監督する `cargo xtask verify-ui` / `linux verify-ui`。build は一度、反復・並行は `--no-build`
- **待ちの型は rules-refs**(前提条件は入力を出す枝で読む・書き込みの答えは読み直しより先に来る・readiness の前の 0 / false は答えでない・一瞬の edge は signal で latch・描画境界は `grabToImage` callback で、`frameSwapped` を待つのは `window.update()` で頼んだ後だけ = 素の `requestUpdate()` は静止した scene で swap しない)— 動詞を書く前に `PGG_AUTO_ACT` / `finishAutoAct` / 動詞名で Grep する。QML テストの待ちも `cargo xtask waits` が見る(残す理由は `// waits(<purpose>): <reason>`、core.md §非同期・並行テスト)
- **ドライバは取りこぼしても止まらない形に組む** — 状態の述語は sampler の拍ごとに見直す(signal だけで進むと、繋いでいない入力 — 寸法・可視・attached — が最後に変わった run が watchdog まで止まる)。**効果を待つ要求は、効果が見えるまで拍ごとに撃ち直す**(冪等な物だけ。書き込みは答えで待つ)。**段を持つドライバは段が変わるたびに名乗る**(`Awaited.at` / `Awaited.all` — 変わった時だけ 1 行)— 天井の行 `awaited` が、止まった段と揃わなかった条件を名指す
