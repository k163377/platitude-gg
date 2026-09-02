---
paths:
  - "crates/platitude-app/**"
---

# platitude-app 規約(Qt Bridges・QML 配線・意匠の実装対応)

UI の色・寸法・用語の正本は [デザイン規約.md](../../internal-docs/デザイン規約.md)(触る § を見出しの Grep で探してその § だけを読み、値は表から選ぶ。Theme.qml と Main.qml 冒頭の定数ブロックはその写し)。**トークンは 3 層**(規約 §トークンの三層 — 基礎 = 色・字・余白の刻み / ベーシック = 通常モードの窓の骨格 / 個別 = 特別な画面 1 枚ごと)。**個別の層は名指しした画面だけが読む** — 別の画面が同じ寸法を欲しがったら、同じ数でも自分のトークンを足す。core 側は `.claude/rules/core.md`、ヘッドレス検証の手順と `PG_AUTO_ACT` 動詞表は verify-ui スキル。

## Qt Bridges の要点(罠)

- 全 QObject は `Rc<RefCell<_>>`(メインスレッド専有)。**QML からの呼び出し中に再入 borrow すると panic** — 借用は短く保つ
- バックグラウンド → UI は `QObjectHolder::get_qml_method_invoker()` で得た `QmlMethodInvoker` をスレッドへ移動し `invoke_method*`(queued 実行)する。それ以外の経路で UI を触らない。invoker は Send だが Clone 不可 — 渡す先の数だけ `get_qml_method_invoker()` で取る
- `#[derive(QModelItem)]` は非衛生的展開 — 使用側ファイルに `use std::collections::HashMap;` が必須。ロール名はフィールド名そのまま(snake_case)。`display` という名前のフィールドは Qt の display ロールに割り当てられる
- `QListModelBase` にバッチ挿入は無い(push/insert は 1 行ずつ)。大量追記は `try_get_rust_proxy_ptr()` + `base_begin_insert_rows(first, last)` + `Vec::extend` + `base_end_insert_rows` のレンジ挿入で行う(P0 スパイク `spike/src/main.rs` の `extend_notified` 参照)
- `QModelItem` のロール型はプリミティブと `String` のみ(`Vec` 不可)— 配列的なデータは文字列にエンコードし、QML 側で機械デコードする(グラフの形状・ラベルが前例)
- `QModelIndex` の公開パスは `qtbridge::qtbridge_type_lib::QModelIndex`
- `ConvertToCamelCase` はスロット/シグナルのメタ名を camel 化する — そのオブジェクトへの `invoke_method!` も camel 名で呼ぶこと(混在事故を防ぐため、invoker で呼ぶ対象には付けないのが安全)
- QML モジュール名は既定で Cargo パッケージ名(ハイフン不可)だが、`#[qobject(NoQmlElement)]` + 手動 `impl QmlRegister`(URI 定数)で任意にできる
- Qt Widgets 不可・C++ 混在不可。必要になった時点で CXX-Qt 移行を検討
- 参照実装は公式 examples(`hello_world` / `minimal_app` / `host_monitor` = tokio 連携 / `color_palette`)。ドキュメント https://doc-snapshots.qt.io/qtbridge-rust/qtbridge/index.html / リポジトリ https://github.com/qt/qtbridge-rust / フォールバックの CXX-Qt book https://kdab.github.io/cxx-qt/book/
- **QML にビジネスロジックを書かない**(表示とインタラクションのみ。JS でのデータ加工禁止)— ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件でもある。**QML が値で問う純ルール(WIP oid・チップの gone filter・remote 名の切り分け・push standing・identity 分解等)は `GitFacts` singleton の stateless slot** — 引数だけを読むので「スロットはバインディングで固まる」規則の例外条件を満たす(隠れた Rust 状態を読む slot は従来どおり禁止)。状態から導く派生値(`stashStanding` / `treeRevision`)は drain で計算する qproperty
- `include_bytes_qml!("dir/file", "prefix")` は **prefix にファイルの相対パス全体を連結**する(qrc:/prefix/dir/file)。ソースのディレクトリ構造 = qrc 構造として設計する。qmldir も埋め込めるので QML singleton(`platitude.ui` の `Theme` / `Metrics`)はこの方式で成立する
- QML は `ui/` 直下フラットに 1 ファイル 1 コンポーネント(`platitude.ui` モジュール)。**新規 QML は qmldir と main.rs の `include_bytes_qml!` の両方へ登録**(qmldir があるディレクトリでは列挙された型しか見えない)
- **QML モジュールは 2 つ — 製品は `ui/`(`platitude.ui`)、検証ハーネスは `auto/`(`platitude.auto`)**。後者は `PG_AUTO_ACT` の動詞・ドライバ・撮影の影武者で、**Cargo feature `automation` でしか埋め込まれない**(素の `cargo build --release` = 出荷ビルドには入らない。xtask が起動するビルドは全部この feature を付ける = `crate::HARNESS_FEATURE`)。**`platitude.ui` から `platitude.auto` の型を名指ししない**(**機械化済み: `cargo xtask structure`**)— 静的な型参照は feature 無しのビルドで Main.qml ごとロード失敗になる。窓とページは `HarnessSeat` を 1 つ持ち、部品を URL で読む(渡す物は `seats`、取り出しは `ask()`)。逆向き(`platitude.auto` → `platitude.ui`)は自由
- **Rust 側で `PG_*` を読むのは `harness::knobs` だけ**(例外は `main.rs` の `PG_LOG` のみ。**機械化済み: `cargo xtask structure`**)— feature 無しのビルドは環境を一切見ずに「誰も運転していない」を返す。新しい自動化のつまみはここへ足す(`AppBackend` に `std::env::var` を書かない)。**core へ渡すのも値**(運転されているかは `settings::Build.driven` で渡す — store 側が `PG_*` を読むと出荷ビルドが設定を失う)
- **意匠の土台は既存の部品から選ぶ**(一覧は rules-refs/app-ui.md の台座の行): カード = `AppCard` / `AppCardFace`、hover の閉じ待ち = `HoverCardHost`、回るリング = `SpinnerIcon`、ダイアログの足 = `DialogActions`、欄の見出し = `LabeledField`、ツールボタンの一言 = `HoverToolButton.tip`、スクロールする一覧 = `AppListView`、閉じる `✕` = `CloseToolButton`、自動化の sampler = `SampleTimer`、**列に並ぶ名前・文 = `CutName`**(素の `elide` を列に書かない — 入り切らなかった幅が右辺に溜まり、行ごとに違う幅で列の右辺がほつれる。名前は既定の中央切り・文は `cutAt: "end"` = 印を列の右辺に貼る)。素の `Popup` / `ListView` / `RotationAnimator` を新しく書かない — 台座に積んだ罠避け(hover の 2 handler・撮影中の停止・追いかけない一覧)がまるごと落ちる。**3 箇所以上で言う同じ文・同じ語は `Words`**(定数は readonly property・規則は引数だけ読む function)
- コンポーネント配線規約: データは親→子へ property、操作・状態変更は子→親へ signal。タブのモデル群は `RepoPage` が所有しペインへ渡す(ペインはモデルのスロットを呼んでよいが、ページ状態は signal で上げて page が変更する)。ウィンドウ横断(タブを開く・settings・identity)は Main まで signal で上げる。`GraphPane.view` は自動化フック専用の露出
- QML の font 値型に `families`(配列)は無い — フォールバックは `Qt.fontFamilies()` と照合して Theme 側で 1 家族に解決する
- `grabToImage` は `Window.contentItem` には使えない("no QML engine")— QML 宣言したアイテムを対象にする
- **画面全体の入力観測は最前面オーバーレイ + `PointHandler`**(passive grab のみが仕様保証)。TapHandler は DragThreshold でも press を消費して下のコントロールへ届かなくし、contentItem 直付けでは手前の MouseArea が accept した press が届かない(実測)
- **`HoverHandler` は「親アイテムの全面」で hover を受け取り、下に重なったアイテムの hover を殺す** — 親に直付けなら自分の子は生きるが、**兄弟として上へ積んだアイテムに載せると、その下の行・セルの `containsMouse` が二度と立たない**(2026-08-22 qmltestrunner で実測。症状は「hover が出ない・出たカードが閉じない・ハイライトが点いたままスタック」)。ペイン全体の hover を測る handler は**ペイン自身に置く**(部品へ切り出す時に持って行かない)
- **Flickable(ListView 含む)に宣言した子は contentItem に養子入りする** — そのままではコンテンツと一緒にスクロールして流れ去る。ビュー枠に固定するオーバーレイ(スクロール位置で出入りする現在ブランチ行など)は `parent:` でビュー自身を指し、描画も入力もデリゲートより手前に来る(実測)
- **QML バインディングはプロパティにしか反応しない** — `#[qslot]` は呼び出し用。`enabled:` 等が値の変化を追う必要があるものは `qproperty!` にする(スロットのままだと初期値のまま固まる)
- **`FontMetrics.advanceWidth()` も同じ側**(メソッド)— バインディングが依存を取らないので**初回の 1 回で確定**し、しかも**その 1 回のメトリクスはまだ既定フォント**(自分の `font` バインディングは後から着く)。エラーも警告も出ず値だけが黙って狂う(実測)。**測って決める値はバインディングにせず関数で押し出す**(`TabStrip.settleTitleCap` / `widestAction` が既にその形)
- **`palette` へのグループ無し代入は全グループを塗り、しかも後から上書きする** — disabled グループまで同じ色になり `enabled: false` が見た目に出ない。`disabled { … }` を足しても、グループ無し側がバインディングだと**そちらが後に確定して勝つ**(リテラル代入なら勝たない — この差で「単体 QML では再現しない」事故になる。実測)。状態で変わる役割は `active` / `inactive` / `disabled` の 3 つ全てに書く(`Main.qml` の `palette`)。`contentItem` 自作のコントロール(`ActionButton`)は palette を通らないので自分で `enabled ? … : textMuted` を書く
- **メニューは `AppMenu` / `AppMenuItem` / `AppMenuSeparator` で書く**(素の `Menu` を使わない — Fusion の素のメニューは**幅が中身によらず 200px 固定**で長い文言が無言で省略され、**地は `palette.base` = グラフと同色**で枠も見えない)
- **右クリックのメニューで行の可否を書くのは `AppMenuItem.offered`(`enabled:` ではない)**(規約 §メニュー = 選べない行は消す)。**`visible` を判定に使わない** — 閉じているメニューの ListView は行を release して visible を落とすので、**開く前に読むと全行が非表示に見える**(`offer()` が「1 行も無い」と判断して永久に開かない)。`offered` は誰も触らないので閉じていても正しい。幅・チップ列・長押しの字下げも `offered` で数える。サブメニューは `AppMenu.applies`(Menu の `visible` は「カードが画面に出ている」の意味なので使えない)
- **メニューは `popup()` ではなく `offer()` で開く** — 出す行が 0 なら開かない(空のカードを出さない)。**行の可否はメニューを開く関数で 1 度だけ決めて、そのメニュー(`RefRowMenu` / `FileRowMenu` / `CommitRowMenu`)の `can*` プロパティに置く**。生の条件を `offered:` に直接書くと、タイマの fetch が `busyCount` を動かした瞬間にポインタの下で行が出入りする
- **区切りは `AppMenuSeparator` が自分で決める**(使う側に `visible:` を書かない)。`AppMenu` が `Component.onCompleted` で自分自身を各区切りの `inMenu` に入れるので、**AppMenu のインスタンス側で `Component.onCompleted` を書かない**(奪うと区切りが親を見失う)。**引かない時は `implicitHeight` を 0 にする**(消えても高さを持つと ListView に穴が残る — `AppMenuItem` と同じ)
- **`ListView.highlightFollowsCurrentItem` は false にする**(`AppListView` が持つ — スクロールする一覧はこれを使う)— 既定の true では `currentIndex` を動かすだけでビューが追いかけ、背景更新で選択行が 1 つずれただけでも履歴を読んでいる人の視界を選択位置まで飛ばす。行の入れ替えでコンテンツが N 行ずれる分は `GraphPane.shiftRows()` で contentY を戻す(**レイアウト前なので 1 拍遅らせる** — 直後は contentHeight が旧値で clamp に食われる)
- **モデルリセット後の ListView は `currentIndex` を保ったまま `contentY` だけ 0 に戻る** — 直後の `positionViewAtIndex` は後続の relayout(polish)に上書きされ、`Qt.callLater` でも不十分。短い Timer(50ms)で遅らせる(`GraphPane` の anchorTimer)

## UI 自動化の因果性

- **固定時間を完了条件にしない** — `PG_AUTO_ACT` は、入力の受理 → 必要なら busy 開始 → busy 終了 → 対象モデルの更新 → 出力プロパティまたは描画可能状態、というその動詞固有の因果を待つ。短い反復 Timer は状態を観測する sampler としてだけ使い、回数・経過時間で成功にしない。`--watchdog-ms` は壊れた run を診断して止める外側の天井であり、撮影時点を選ばない
- **非因果の終了境界も成功条件にしない** — 性能測定の 12 秒窓は測定入力として保持するが、起動からの固定 quit で成功扱いにしない。`perf_done` と親 watchdog の完了・kill 判定を使い、`PG_AUTO_ACT` の因果完了と混ぜない。worktree からの raw app 起動は親監督なしでは許可せず、`cargo xtask verify-ui` / `cargo xtask linux verify-ui` を使う
- **harness は親の自動化状態を継承しない** — 起動前に全 `PG_AUTO_*` と関連する automation env を除去し、その harness が所有する値だけを設定する。repo / config / shot は run 固有にし、別 harness や並行 session の入力・状態・出力を合成しない
- **1 run の完了 owner は 1 つ** — page が動詞を原子的に claim し、window-level 動詞は page completion を defer する。新しく開いた tab が同じ動詞を再実行してはならない。撮影は owner が `finishAutoAct()` を 1 回通知した後だけ
- **前提条件を完了判定に混ぜない** — 動詞が動く前に要る状態(タブが 2 本ある・行が選べる)は**入力を出す枝の中だけ**で読む。毎 tick 読み直すと、その動詞自身の答え(1 本減った)が前提を割って完了へ進めなくなり、watchdog まで無言で待つ。**入力が届いたことも確かめてから latch する** — ビュー(ListView 等)の item はモデルが行を得た次のレイアウトで生まれるので、`itemAtIndex` は空振りしうる。空振りを押下として latch すると同じ無限待ちになる(入力経路の関数に「押せたか」を答えさせる)
- **書き込みの答えは、その書き込みが無効化した読み直しより先に来る** — core は `WriteFinished` を出してから status / refs / グラフを publish する(`session::write::run_write`)ので、**書き込み境界で撮った絵は撃つ前の画面**。動詞の見せ物が「書き込みの後の画面」なら、書き込みの後段にもう 1 段のバリアを置き、**その書き込みが動かす当のモデルの行を待つ**(グラフは `graphGoneOid`、作業ツリーの一覧は `treeGoneRow`)。**カウンタで待たない** — status / refs の seq は誰も頼んでいない読み直しでも進むので、この書き込みが動かしたことの証拠にならない
- **「まだ答えが無い」と値 0 / false を分ける** — 非同期モデルは `loaded` / request generation / sequence 等の readiness を公開し、自動化は readiness の後で値を読む。初期値 0 を clean・空・完了と判定しない
- **一瞬だけ立つ状態は signal で観測して latch する** — error / busy / loading が polling 1 周より短くても、その実 edge を見た証拠を保持し、非同期 `grabToImage` が終わるまで意図した中間表示を保つ。入力フラグを立てただけで出力状態を偽装しない
- **描画境界は画像 callback が答える** — completion 後に `requestUpdate()` と event-loop turn を通し、app / overlay 両方の `grabToImage` callback が返ってから終了する。静止した offscreen scene は `frameSwapped` を出さないことがあるため、それ単独を完了条件にしない
- **並行 run は状態を共有しない** — preset repository・config・shot directory は run ごとに作り、明示した `--repo` / `--config-dir` / `--shot-dir` は原子的に所有権を取る。同じ明示リソースの同時利用は待って混線させず fail fast。build は一度済ませ、反復・並行実行は `--no-build` にする

## 配線済み操作の意匠と実装対応

**個々の操作・部品の意匠決定・実装対応・罠、および配線済み操作の一覧の正本は [rules-refs/app-ui.md](../rules-refs/app-ui.md)**(自動ロードされない)— 触る部品名・操作名・動詞名・規約 §名で Grep して該当行だけを読み(全読みしない)、配線・決定・罠は該当行へ 1 行で追記する(本ファイルへは全セッション共通の不変条件だけを昇格。CLAUDE.md は未配線だけを持つ)。
