---
paths:
  - "crates/platitude-app/**"
---

# platitude-app 規約(Qt Bridges・QML 配線・意匠の実装対応)

app / QML のファイルを読み書きすると自動ロードされる。UI の色・寸法・用語の正本は [デザイン規約.md](../../internal-docs/デザイン規約.md)(QML を書く前に、触る § を見出しの Grep で探してその § だけを読み、値は表から選ぶ — 3,000 行超を頭から全読みしない)。core 側は `.claude/rules/core.md`、ヘッドレス検証の手順と `PG_AUTO_ACT` 動詞表は verify-ui スキル。

## Qt Bridges の要点(罠)

- main は `QApp::new().register::<Backend>().load_qml(include_bytes!("qml/Main.qml")).run();` の形。QML はバイナリに埋め込む
- `#[qobject]` は `impl` ブロックに付ける。struct は `Default` 実装必須。プロパティは `qproperty!("name", Member = field, Notify = signal)`、メソッドは `#[qslot]` / `#[qsignal]`。`ConvertToCamelCase` で QML 側を camelCase に
- QML モジュール名は Cargo パッケージ名になる
- 全 QObject は `Rc<RefCell<_>>`(メインスレッド専有)。**QML からの呼び出し中に再入 borrow すると panic** — 借用は短く保つ
- バックグラウンド → UI は `QObjectHolder::get_qml_method_invoker()` で得た `QmlMethodInvoker` をスレッドへ移動し `invoke_method*`(queued 実行)する。それ以外の経路で UI を触らない
- `#[derive(QModelItem)]` は非衛生的展開 — 使用側ファイルに `use std::collections::HashMap;` が必須。ロール名はフィールド名そのまま(snake_case)。`display` という名前のフィールドは Qt の display ロールに割り当てられる
- `QListModelBase` にバッチ挿入は無い(push/insert は 1 行ずつ)。大量追記は `try_get_rust_proxy_ptr()` + `base_begin_insert_rows(first, last)` + `Vec::extend` + `base_end_insert_rows` のレンジ挿入で行う(P0 スパイク `spike/src/main.rs` の `extend_notified` 参照)
- `ConvertToCamelCase` はスロット/シグナルのメタ名を camel 化する — そのオブジェクトへの `invoke_method!` も camel 名で呼ぶこと(混在事故を防ぐため、invoker で呼ぶ対象には付けないのが安全)
- QML モジュール名は既定で Cargo パッケージ名(ハイフン不可)だが、`#[qobject(NoQmlElement)]` + 手動 `impl QmlRegister`(URI 定数)で任意にできる
- Qt Widgets 不可・C++ 混在不可。必要になった時点で CXX-Qt 移行を検討
- 参照実装は公式 examples(`hello_world` / `minimal_app` / `host_monitor` = tokio 連携 / `color_palette`)。ドキュメント https://doc-snapshots.qt.io/qtbridge-rust/qtbridge/index.html / リポジトリ https://github.com/qt/qtbridge-rust / フォールバックの CXX-Qt book https://kdab.github.io/cxx-qt/book/
- **QML にビジネスロジックを書かない**(表示とインタラクションのみ。JS でのデータ加工禁止)— ブリッジ差し替え(Qt Bridges → CXX-Qt)を可能に保つための条件でもある
- `include_bytes_qml!("dir/file", "prefix")` は **prefix にファイルの相対パス全体を連結**する(qrc:/prefix/dir/file)。ソースのディレクトリ構造 = qrc 構造として設計する。qmldir も埋め込めるので QML singleton(`platitude.ui` の `Theme` / `Metrics`)はこの方式で成立する
- QML は `ui/` 直下フラットに 1 ファイル 1 コンポーネント(`platitude.ui` モジュール)。**新規 QML は qmldir と main.rs の `include_bytes_qml!` の両方へ登録**(qmldir があるディレクトリでは列挙された型しか見えない)
- コンポーネント配線規約: データは親→子へ property、操作・状態変更は子→親へ signal。タブのモデル群は `RepoPage` が所有しペインへ渡す(ペインはモデルのスロットを呼んでよいが、ページ状態は signal で上げて page が変更する)。ウィンドウ横断(タブを開く・settings・identity)は Main まで signal で上げる。`GraphPane.view` は自動化フック専用の露出
- QML の font 値型に `families`(配列)は無い — フォールバックは `Qt.fontFamilies()` と照合して Theme 側で 1 家族に解決する
- `grabToImage` は `Window.contentItem` には使えない("no QML engine")— QML 宣言したアイテムを対象にする
- **画面全体の入力観測は最前面オーバーレイ + `PointHandler`**(passive grab のみが仕様保証)。TapHandler は DragThreshold でも press を消費して下のコントロールへ届かなくし、contentItem 直付けでは手前の MouseArea が accept した press が届かない(2026-08-04 実測)
- **Flickable(ListView 含む)に宣言した子は contentItem に養子入りする** — そのままではコンテンツと一緒にスクロールして流れ去る。ビュー枠に固定するオーバーレイ(スクロール位置で出入りする現在ブランチ行など)は `parent:` でビュー自身を指し、描画も入力もデリゲートより手前に来る(2026-08-04 実測)
- **QML バインディングはプロパティにしか反応しない** — `#[qslot]` は呼び出し用。`enabled:` 等が値の変化を追う必要があるものは `qproperty!` にする(スロットのままだと初期値のまま固まる)
- **`FontMetrics.advanceWidth()` も同じ側**(メソッド)— バインディングが依存を取らないので**初回の 1 回で確定**し、しかも**その 1 回のメトリクスはまだ既定フォント**(自分の `font` バインディングは後から着く)。実測 2026-08-11: `font.wordSpacing: Theme.spaceXs - mono.advanceWidth(" ")` は**既定 UI フォントの空白 4px を読んで 4 − 4 = 0** に落ち着き、詰めが黙って消えた(エラーも警告も出ない。`Component.onCompleted` で同じ式を読むと 7 を返すので、**ログだけ見ると正しく見える**)。**測って決める値はバインディングにせず関数で押し出す**(`TopBar.settleTitleCap` / `widestAction` が既にその形)
- **`palette` へのグループ無し代入は全グループを塗り、しかも後から上書きする** — disabled グループまで同じ色になり `enabled: false` が見た目に出ない。`disabled { … }` を足しても、グループ無し側がバインディングだと**そちらが後に確定して勝つ**(リテラル代入なら勝たない — この差で「単体 QML では再現しない」事故になる。2026-08-05 実測)。状態で変わる役割は `active` / `inactive` / `disabled` の 3 つ全てに書く(`Main.qml` の `palette`)。`contentItem` 自作のコントロール(`ActionButton`)は palette を通らないので自分で `enabled ? … : textMuted` を書く
- **メニューは `AppMenu` / `AppMenuItem` / `AppMenuSeparator` で書く**(素の `Menu` を使わない)。Fusion の素のメニューは**幅が中身によらず 200px 固定**(背景 Rectangle の implicitWidth。contentItem の ListView は implicitWidth を持たない)で長い文言が無言で省略され、**地は `palette.base` = グラフと同色**で枠も見えない。`AppMenu` は最も広い行に幅を合わせ(上限はウィンドウ幅)、溢れた行だけ省略して hover で全文を出す
- **右クリックのメニューで行の可否を書くのは `AppMenuItem.offered`(`enabled:` ではない)**(規約 §メニュー = 選べない行は消す)。**`visible` を判定に使わない** — 閉じているメニューの ListView は行を release して visible を落とすので、**開く前に読むと全行が非表示に見える**(`offer()` が「1 行も無い」と判断して永久に開かない = 実測で踏んだ)。`offered` は誰も触らないので閉じていても正しい。幅・チップ列・長押しの字下げも `offered` で数える。サブメニューは `AppMenu.applies`(Menu の `visible` は「カードが画面に出ている」の意味なので使えない)
- **メニューは `popup()` ではなく `offer()` で開く** — 出す行が 0 なら開かない(空のカードを出さない)。**行の可否はメニューを開く関数で 1 度だけ決めて `page` のプロパティに置く**(`menuCanSwitch` / `menuFileCanWrite` 等)。生の条件を `offered:` に直接書くと、タイマの fetch が `busyCount` を動かした瞬間にポインタの下で行が出入りする
- **区切りは `AppMenuSeparator` が自分で決める**(使う側に `visible:` を書かない)。`AppMenu` が `Component.onCompleted` で自分自身を各区切りの `inMenu` に入れる(MenuSeparator には `MenuItem.menu` に当たるものが無い)ので、**AppMenu のインスタンス側で `Component.onCompleted` を書かない**(奪うと区切りが親を見失う)。判定は「すぐ上の group に行が残っている」+「下にまだ行がある」= 端の線も、group が空になった時の二重線も出ない。**引かない時は `implicitHeight` を 0 にする**(ListView は高さで並べるので、消えても高さを持つと穴が残る — `AppMenuItem` と同じ)
- **`ListView.highlightFollowsCurrentItem` は false にする** — 既定の true では `currentIndex` を動かすだけでビューが追いかけ、背景更新で選択行が 1 つずれただけでも履歴を読んでいる人の視界を選択位置まで飛ばす。行の入れ替えでコンテンツが N 行ずれる分は `GraphPane.shiftRows()` で contentY を戻す(**レイアウト前なので 1 拍遅らせる** — 直後は contentHeight が旧値で clamp に食われる)
- ブリッジは **Qt Bridges 採用で確定**(Phase 0 スパイク合格)。CXX-Qt へ差し替え可能な構成を維持。スパイクコードは `spike/` に残置
- UI は [internal-docs/デザイン規約.md](../../internal-docs/デザイン規約.md) が正本(Theme.qml と Main.qml 冒頭定数ブロックはその写し)。グラフ・インタラクション定数とレイアウト初期値は **2026-08-02 の UI 基準確定で規約へ昇格済み**

## 配線済み操作の意匠と実装対応

- **配線済み操作の一覧は [rules-refs/app-ui.md](../rules-refs/app-ui.md) 冒頭が正本**(Grep: `配線済み`)。機能を配線したらそちらの行へ追記する — CLAUDE.md は未配線だけを持つ

**個々の操作・部品の意匠決定・実装対応・罠の正本は [rules-refs/app-ui.md](../rules-refs/app-ui.md)**(1 項目 1 行。自動ロードされない)。**書く前に、触る部品名(QML ファイル名)・操作名・動詞名・規約 §名で Grep し、該当項だけを読む(前後は `-C 2` で足りる)。ファイルを Read で全読みしない** — 1 項目 ≈ 1KB × 180 項でコンテキストを数万トークン食う(実測: 4 日で全読み 28 回が最大級の膨張源)。読まずに書くと、決まった意匠を作り直すか、踏んだ罠をもう一度踏む。**決定・罠を足したら同ファイルへ 1 行で追記**(全セッション共通の不変条件に昇格するものだけ本ファイルへ)。
