# 分割・共通化 各論(1 項目 1 行。自動ロードされない — 触る項を Grep で引く)

置くのは**どのファイルにも効く**分割・共通化の技法と罠だけ。**1 つのファイルだけの事情(分割しない理由を含む)はそのファイルの冒頭コメント**。

## 分割の各論

- **検証ハーネスは「別のビルド」で分ける** — ファイルを動かすだけでは出荷物からは消えない(モジュールの分け方は rules/app-ui.md)。**条件は「製品側にハーネスの名前が静的に残らないこと」** — Rust なら型・関数が `cfg` の内側に閉じていること(`harness::knobs` の `#[cfg(feature = "automation")] fn read`)、QML なら URL で読むこと(`HarnessSeat`)。名前が残っている限り、消したビルドはリンクもロードもできない
- **QML の `Component.onCompleted` は親が先、子が後** — 完了ハンドラから触る子の物は「訊かれたらその場で作る」形にする(`HarnessSeat.ask()`)

- **mod.rs から出す型の `pub(super)` は `pub(crate)` と書き写す** — session は crate 直下なので mod.rs の `pub(super)` = `pub(crate)`。移動先で `pub(super)` にすると session 内へ狭まる(逆に mod.rs で private だった型は移動先で `pub(super)` にすると元と同じ範囲)
- **insta のスナップショットはソースファイル基準の `snapshots/` を見る** — `foo.rs` → `foo/bar.rs` へ動いたテストは置き場(`src/snapshots/` → `src/foo/snapshots/`)とファイル名(モジュールパス)の両方が変わる。中身は不変なので `INSTA_FORCE_UPDATE=1 cargo test` で `source:` 行だけ再生成する
- **`mod tests` を専用ファイルへ持ち上げる時、複数行文字列リテラルの中身は字下げされていない** — 一律 dedent は fixture を壊す。宣言行だけ下げ、リテラルの行はそのまま移す(`parse/diff/testkit.rs` の `PATCH` が実例)
- **xtask も mod.rs 方式 — `run` / dispatch は子(`cli.rs` / `run.rs` 等)へ出す** — mod.rs に残るのは `mod`・re-export・共有型・`Command` の static(`budget` / `gate` / `shots` / `verify` / `waits`)。file-parent の `corpus` `docs` `land` `linux` `perf` `seats` `structure` `sweep` `usage` は触った時に寄せる
- **外部クレートと同名のモジュールを作ったら親の `use` は `self::` で書く** — `settings/toml.rs` があると mod.rs の `use toml::…` は extern prelude と衝突して E0659。子ファイル側は素の `use toml::…` のままでよい
- **移動だけのコミットは、移動先を決める前に「どの行がどこへ行くか」を機械で突き合わせる** — 元ファイルの全行が行き先ちょうど 1 つに入り、落ちるのは列挙した scaffolding(バナー・共有 `use`・`mod tests` の包み)だけ、と検証してから書き出す
- **`include_bytes!` / `include_str!` はソースファイル基準** — 1 段深いディレクトリへ動かした項は `../` を足さないと別の場所を読みに行く。移動先に同名のディレクトリが在れば黙って別物を焼き込む
- **1 段深くした項の `pub(super)` は届く先が 1 段狭まる** — 元の親から呼ばれていたなら `pub(crate)` へ広げ、間に挟まった mod.rs が `pub(super) use` で元の広さへ戻す(再輸出は元より広くできない = `pub(super)` のままだと E0364/E0365)
- **本体を移した fn からは `#[expect(unsafe_code)]` を剥がす** — `unsafe` をマクロや別 fn へ出した側は expectation が不発になり `-D warnings` で赤くなる
- **generic な木・容器は `impl<T> Default` で書く** — `#[derive(Default)]` は `T: Default` の境界が付き、葉に Default の無い型(借用ポインタのタプル等)を入れられなくなる
- **`#![allow(dead_code)]` の下から実装を出したら、`pub use` の再輸出は呼ばれている名前だけ** — 再輸出は別 lint(`unused_imports`)なので `-D warnings` で赤くなる。呼ばれていない名前は module 越し(`support::wait::QUIET_BUDGET`)で届くから消してよい
- **「可変ローカルを全分岐が触る」は fn 全体の話** — `parse_patch` は 6 本を 1 つの record(`Reading`)に束ねてから、行の種類ごとに method へ割った。**割る前に、既に在る兄弟の形を探す** — 片側だけが inline に残っていることがそのまま切れ目。`use super::*` のテストは道連れになる — 本体が名指さなくなった型はテスト側で import し直す
- **セッションの事実は、それが起きた所で書き留める** — 後で走るプロセスは自分の cwd からそれを判断できない(実例と理由は `seats::entry` のコメント)

- **QML の「描かないホスト」は `anchors.fill: parent` を書く** — メニュー・ポップアップ・ダイアログは宣言された親アイテム越しに窓を測る(`AppMenu.ownerItem.Window.window` / `AppDialog` の `anchors.centerIn: parent` / `Popup.x` は親座標)。page 直下から寸法ゼロの Item の下へ移すと、行幅の上限もダイアログの中央も 0 になる。行カード・チップ一覧のようにシーン座標を受け取って置くものも同じ(`row.mapToItem(host, …)` が page 相当になるのは埋めた時だけ)
- **`page.` を名乗る名前を子へ移したら、外から呼ぶ口だけは page に残す** — 窓の帯は `curPage.<名前>` で能動タブを読む(`TopBar`)ので、alias 再輸出か 1 行の転送を残さないとボタンが黙って死ぬ。自動化(`AutoActDriver`)側は逆に新しい持ち主を property で渡して呼ぶ
- **操作 1 つぶんを寄せる形は「要求・保留・完了を 1 人の持ち主へ、`OperationId` で」** — `crates/platitude-app/src/ops/`。寄せ方の 3 点・土台(`Press`)・QML 側に残る物は `ops/mod.rs` のコメント。残りの候補は P3-確認事項 の write group の項
- **page の状態を書く子は「書く先を `required property Item page` で受けて、自分だけが書く状態だけを書く」形なら出せる** — 跨ぐのは同じ状態を複数の子が書く時で、そこから先が再設計の領分(実例 `PageLayout`)
- **dispatcher が id で start する切片は独立ファイルにできない** — QML の id はファイルを跨がないので、出した先は全メンバを `readonly property alias` で再公開し、親はそれをまた別名で読み戻すことになる。その切れ目は閾値まで 1 ファイルで持つ
- **自動化の切片は「動詞の dispatch」か「計測の因果」かで宿主が決まる** — 前者は `AutoActDriver` / `WindowAutoActDriver`(`PGG_AUTO_ACT` gate)、後者は `PagePerfDriver` / `WindowPerfDriver`(`PGG_AUTO_PERF` gate)。**動かすのはこの境界の内側**。移す前に、その Loader の `active` が動かす側の起動条件を全部含むかを確かめる(フラグは動詞無しでも立つ — 実例 `PageAutoStart`)
- **性能敏感な QML(行デリゲート)は「切り出す部品の root を、置き換える当のアイテムにする」** — `Layout.*` / `anchors` / `visible` は使用側に残るので、アイテム木は分割前と同じ本数のまま。`GraphRowDelegate` → `GraphLaneCell` と `DiffPane` の行がこの形
- **`ListView.view` 直読を in-property 化しても、`ListView.onReused` だけはデリゲート root から動かせない** — attached `ListView` は root にしか生えない。切り出した子の再描画はデリゲートから子のメソッドを呼ぶ(`GraphLaneCell.loadFace` / `repaintLanes`)
- **リスト自体を切り出す時は root をその `ListView` にする** — デリゲートが `ListView.view` 越しにしか読めない状態はリストに載っているので、リストと一緒に動く。root がリストそのものなら、外からリストを指す参照(`ListView.view` 経由を含む)は 1 つも書き換えずに済む。**`Keys.on*Pressed` は attached property なので使用側に残せる**(実例 `GraphList`。キーを使用側に残した理由は `GraphPane.qml` のコメント)
- **手を持つ部品と、その手が上げる線は同じファイルに入らない** — QML の重なりは親の 1 つの `z` でまとまるので、線をリストの下(`z` 既定)に、当たり判定をリストの上(`z: 2`)に置くことは 1 つの子アイテムでは両立しない。手だけを持つ部品と、その状態を property で読んで線を描く部品に分ける(実例 `GraphColumnDividers` / `GraphDividerLines`)
- **切り出した非表示のホストは `Item` にする**(子を持つ物の話)— `QtObject` は子を置く場所(default property)を持たないので、`Component` / `Timer` を連れて出た瞬間に `Cannot assign to non-existent default property` で**その型ごと unavailable になり、Main.qml が丸ごとロードに失敗する**(窓は出ず、verify-ui は watchdog まで無言)。子を持たず property と関数だけの物(`PageLayout` / `CommitMenuState` / `CommandsOwner`)は `QtObject` で足りる。子にメニュー・ポップアップ・ダイアログが来るなら上の「描かないホスト」も要る
- **ペインの「下に貼り付いた塊」は、その塊の Flickable を root にして出せる** — `contentY` を触る関数は塊の側へ行く。代わりに増えるのは読み戻しの口 — ペインが名指していた子が alias 1 段深くなる。**書ける口は関数で出す**(`readonly property alias` へは代入できない)。ペインが子の座標を要る所はそのアイテム自身を alias で渡す(実例 `WipCommitBlock`)
- **`ListView` の `delegate:` を独立ファイルへ出す時、モデルのロールに依存する「使用側の式」は使用側に書ける** — 宣言した id 経由でその行自身の required property を読める。行が持つ状態(選択集合)をペイン側に残したまま行を純表示に保てる

## コマンドの正本(`cargo xtask docs` が読む。規約本体は rules/structure.md §共通化)

- **宣言は実装モジュールの中**(`crates/xtask/src/command.rs` の `Command` を `pub(crate) static` で置き、そのモジュールの `COMMANDS` に並べる)。`crates/xtask/src/commands.rs` は所有者の索引だけ — 1 コマンドの構文・用途・実行場所・前提条件・許可の参照先が 1 箇所
- **許可は参照** — `Permission::Escape` は hook の const(`hook::APPROVAL_FLAGS`)をそのまま持ち、`Permission::Permit` は `hook::permit` の判断を指すだけ。走ってよいかは常に hook がその場で判断する
- **id は操作の名前** — verb の改名・モジュール移動・用途の書き換えを越えて同じ。1 id は 1 操作。台帳は `crates/xtask/command-ids.txt`(`--sync` が書く。id が消えれば diff の削除行 + 赤)
- **文書側の記法は 3 段**: 実行形(escape 付き・verb の後に語がある)は `` `<!--cmd:<id>-->` `` を直前に置き、続く inline code を `--sync` が書く / **`cargo xtask` を省く節**(gate スキルの冒頭のように、省略を宣言した節)は `` `<!--call:<id>-->` `` で call だけを書かせる — **escape 付きのコマンドに省略形は無い**(flag が落ちると許可が見えなくなるので拒む)/ 単なる名指し(`cargo xtask <verb>` だけ)は素のまま — verb が実在するかだけ見る。**id には実行方法が必ず並ぶ**
- **Rust 側は型付きの参照**(`crate::gui::KILL.line()` / `.instruction()`)。deny 文・エラー文も同じ参照で綴る
- **どのファイルを読むかは `docs/commands.rs` の `ROOTS` / `LOOSE` / `USAGE` / `DISPATCH` が持つ** — ソースのコメント行・テスト(`#[cfg(test)]` 以降と `crates/xtask/tests/`)・`ci/baseline`・`ci` / `packaging` のシェルは読まない(理由はそこのコメント)
- **限界**(明示): 宣言に無い option の組み合わせは「例」として verb しか見ない・前置き無しの 1 語は常に対象外(英語と区別できない)・fence の中は書き換えない(複合シェル行に marker は置けない)
- **追加**: モジュールに `static` を足して `COMMANDS` に並べ、<!--cmd:docs.sync-->`cargo xtask docs --sync` で台帳を書き、usage ページにその verb の節を書く(option は節の中で名前を出す — 出ていなければ `docs` が赤くなる)
- **改名・廃止**: 専用の変更として、文書・スキル・ソース・`.claude/settings.json`・hook の全参照を動かしてから `--sync`。過去の記録(ci/baseline・確認事項の履歴)はそのまま

## 分割しない判断(閾値超過の免除の台帳 — 行が消えたら分割済み)

- **この見出しの節は `cargo xtask structure` が機械で読む** — ファイルの恒久免除になるのは行頭が `- **` + バッククォート付きパスの箇条書きだけ(パス後方一致)。行の形は ``- **`<パス>`** — 理由はファイル冒頭のコメント``。fn 単位の項のようにパスを文中で挙げるだけの行は免除にならない。太字を落とすと免除が外れて count が赤くなる。見出しの `## 分割しない判断` を変えると免除が全部外れる(節が無い時はツールがエラーで止まる)。範囲は次の `## ` 見出しまで
- **免除は行数を持たない — 行が在れば免除、消えれば分割済み** — パスに当たるファイルが無い行は fail(割り終わったら行を消す)。第 2 の閾値は却下済み(数字の付け替えが増えるだけで、理由の読み直しにはならない)。**理由は台帳の行ではなく、そのファイルの冒頭コメントに書く**(書き方と読み直しは rules/structure.md §長さの閾値)

- **`ui/RepoPage.qml`** — 理由はファイル冒頭のコメント
- **`#[expect]` の不発判定は「その lint が実際に発火するか」** — 未有効(pedantic)のままでも長い fn なら不発にならない
- **`mod tests` を専用ファイルへ出すだけでファイルが閾値を切ることがある** — 判定は「コードだけの行数」なので、テストが半分を占める大きな .rs は最初にこれを見る。**置き場は 2 つ**: テストが対象の公開 API しか触らないなら兄弟の `<名前>_tests.rs`(core の既存踏襲)、private を触るなら対象の子 = `<名前>/tests.rs`(可視性は製品の都合だけで決める)。dedent は往復で証明する — 4 桁下げて出した後、`sed s/^./    &/` で戻して原本と `cmp` する(複数行リテラルが混ざっていれば必ず差が出る)
