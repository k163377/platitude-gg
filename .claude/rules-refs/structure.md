# 分割・共通化 各論(1 項目 1 行。自動ロードされない — 触る項を Grep で引く)

## 分割の各論

- **`session/mod.rs` の `use` ブロックは兄弟 15 ファイルの共有 prelude** — 各ファイルの `use super::*` が `Oid` / `GitError` / `Segment` 等をここ経由で引くので、mod.rs 自身が使わなくなった import も消せない(glob 経由の利用を rustc は「使用済み」と数えるため警告も出ない = 消すと一斉に壊れて初めて分かる)
- **`session/repo.rs` は作れない** — `crate::repo`(RepoInfo / open)と名前が衝突する。`RepoSession` の置き場は `session/repo_session.rs`
- **mod.rs から出す型の `pub(super)` は `pub(crate)` と書き写す** — session は crate 直下なので mod.rs の `pub(super)` = `pub(crate)`。移動先で `pub(super)` にすると session 内へ狭まる(逆に mod.rs で private だった型は移動先で `pub(super)` にすると元と同じ範囲)
- **insta のスナップショットはソースファイル基準の `snapshots/` を見る** — `foo.rs` → `foo/bar.rs` へ動いたテストは置き場(`src/snapshots/` → `src/foo/snapshots/`)とファイル名(モジュールパス)の両方が変わる。中身は不変なので `INSTA_FORCE_UPDATE=1 cargo test` で `source:` 行だけ再生成する(手編集は禁止 — CLAUDE.md Rust 規約)
- **`mod tests` を専用ファイルへ持ち上げる時、複数行文字列リテラルの中身は字下げされていない** — 一律 dedent は fixture を壊す。宣言行だけ下げ、リテラルの行はそのまま移す(`parse/diff/testkit.rs` の `PATCH` が実例)
- **外部クレートと同名のモジュールを作ったら親の `use` は `self::` で書く** — `settings/toml.rs` があると mod.rs の `use toml::…` は extern prelude と衝突して E0659。子ファイル側は現モジュールに `toml` が無いので素の `use toml::…` のままでよい
- **移動だけのコミットは、移動先を決める前に「どの行がどこへ行くか」を機械で突き合わせる** — 元ファイルの全行が行き先ちょうど 1 つに入り、落ちるのは列挙した scaffolding(バナー・共有 `use`・`mod tests` の包み)だけ、と検証してから書き出す。取りこぼし・二重取りはこれでしか出ない
- **`include_bytes!` / `include_str!` はソースファイル基準** — 1 段深いディレクトリへ動かした項は `../` を足さないと**別の場所を読みに行く**(`winframe.rs` の `../assets/icon-*.png` → `winframe/win32/icon.rs` では `../../../assets/`)。パスが実在しなければコンパイルエラーで気付くが、移動先に同名のディレクトリが在れば黙って別物を焼き込む
- **1 段深くした項の `pub(super)` は届く先が 1 段狭まる** — 元の親から呼ばれていたなら `pub(crate)` へ広げ、間に挟まった mod.rs が `pub(super) use` で元の広さへ戻す(再輸出は元より広くできない = `pub(super)` のままだと E0364/E0365)。`#[cfg(windows)] mod win32` 配下のような private な入れ子では、`pub(crate)` にしても外から辿れる道は増えない
- **本体を移した fn からは `#[expect(unsafe_code)]` を剥がす** — `unsafe` をマクロや別 fn へ出した側は expectation が不発になり `-D warnings` で赤くなる(貼り忘れ側はエラーになるので、危ないのは剥がし忘れの方)
- **generic な木・容器に `#[derive(Default)]` を貼らない** — `T: Default` の境界が付き、葉に Default の無い型(借用ポインタのタプル等)を入れられなくなる。`impl<T> Default` を手で書く
- **`#![allow(dead_code)]` の下から実装を出したら、誰も呼んでいない名前まで `pub use` で再輸出しない** — allow が覆っていたのは `dead_code` で、再輸出は別 lint(`unused_imports`)なので `-D warnings` で赤くなる。呼ばれていない名前は module 越し(`support::wait::QUIET_BUDGET`)で届くから消してよく、残す口は「実際に外から呼ばれている名前」だけ(tests/it/support が実例)
- **`hook/mod.rs` だけは実装(`run` のディスパッチと `pre_shell` の連鎖)を持つ意図した例外** — 拒否は pre_git → pre_kill → pre_launch の順で最初の拒否が答え。この順序が安全性そのもので、1 ファイルに見えていることが mod.rs 純度に勝つ

- **QML の「描かないホスト」は `anchors.fill: parent` を書く** — メニュー・ポップアップ・ダイアログは宣言された親アイテム越しに窓を測る(`AppMenu.ownerItem.Window.window` / `AppDialog` の `anchors.centerIn: parent` / `Popup.x` は親座標)。page 直下から寸法ゼロの Item の下へ移すと、行幅の上限もダイアログの中央も 0 になる。行カード・チップ一覧のように**シーン座標を受け取って置く**ものも同じ(`row.mapToItem(host, …)` が page 相当になるのは埋めた時だけ)
- **`page.` を名乗る名前を子へ移したら、外から呼ぶ口だけは page に残す** — 窓の帯は `curPage.<名前>` で能動タブを読む(`TopBar` の push 8 本)ので、alias 再輸出か 1 行の転送を残さないとボタンが黙って死ぬ。自動化(`AutoActDriver`)側は逆に**新しい持ち主を property で渡して呼ぶ**(page を経由しない)
- **`ui/RepoPage.qml` の残り(部品を出し切った後)は 1 タブ分の状態機械** — 選択の追従・diff の開閉・書き込み結果の後始末・モデル群の所有が `selectedOid` / `repoTab` で噛み合っている。次に割るならこの単位だが、**先に「page の状態を子から書かない」形へ寄せる必要がある**(免除ではない — 行数は baseline のラチェットが持つ)
- **page の状態を書く子は「書く先を `required property Item page` で受けて、その 2 本だけを書く」形なら出せる** — `PageLayout` は `sidebarCollapsed` / `commandsOpen` を書き戻すが、書く場所が保存レイアウトの復元 1 箇所に閉じているので page 側の不変条件を跨がない。跨ぐのは**同じ状態を複数の子が書く**時で、そこから先が再設計の領分
- **自動化の切片は「動詞の dispatch」か「計測の因果」かで宿主が決まる** — 前者は `AutoActDriver` / `WindowAutoActDriver`(`PG_AUTO_ACT` gate)、後者は `PagePerfDriver` / `WindowPerfDriver`(`PG_AUTO_PERF` gate)。**この境界を跨いで動かさない** — `PG_AUTO_SELECT` の実行を verb 側へ出すと readiness(`PagePerfDriver.selected`)と実行が別ファイルに割れる。移す前に、その Loader の `active` が動かす側の起動条件を全部含むかを確かめる(フラグは動詞無しでも立つ)
- **性能敏感な QML(行デリゲート)は「切り出す部品の root を、置き換える当のアイテムにする」** — `Layout.*` / `anchors` / `visible` は使用側に残るので、アイテム木は分割前と同じ本数のまま(包み Item が増えない)。`GraphRowDelegate` → `GraphLaneCell` / `GraphRowChips` / `WipTallyRow` と `DiffPane` の行がこの形。基準リポジトリ(kotlin 227k commits)で分割前後とも 99.7–99.8fps = 差無し(2026-08-16 実測、vsync 100Hz 上限)
- **`ListView.view` 直読を in-property 化しても、`ListView.onReused` だけはデリゲート root から動かせない** — attached `ListView` は root にしか生えない。切り出した子の再描画はデリゲートから子のメソッドを呼ぶ(`GraphLaneCell.loadFace` / `repaintLanes`)
- **手を持つ部品と、その手が上げる線は同じファイルに入らない** — QML の重なりは親の 1 つの `z` でまとまるので、線をリストの下(`z` 既定)に、当たり判定をリストの上(`z: 2`)に置くことは 1 つの子アイテムでは両立しない。`GraphColumnDividers` は手だけを持ち、線は `GraphPane` に残して状態を property で読む(包みには元の `ColumnDivider` と同じ `z: 2` を書く — 書かないと `lanePan`(z:1)・`laneBar`(z:2)との前後が入れ替わる)
- **切り出した非表示のホストは `QtObject` ではなく `Item` にする** — `QtObject` は子を置く場所を持たないので、`Component` / `Timer` を連れて出た瞬間に `Cannot assign to non-existent property "data"` で**その型ごと unavailable になり、Main.qml が丸ごとロードに失敗する**(窓は出ず、verify-ui は watchdog まで無言 = 140 秒後に FAIL)。既存の非表示ホスト(`WindowPerfDriver` / `AutoShotDriver`)が全部 `Item` なのはこれが理由
- **`ListView` の `delegate:` を独立ファイルへ出す時、モデルのロールに依存する「使用側の式」は使用側に書ける** — `picked: … lineChosen(diffRow.hunk, diffRow.line)` のように、宣言した id 経由でその行自身の required property を読める。行が持つ状態(選択集合)をペイン側に残したまま行を純表示に保てる(`DiffRowDelegate`)

## 分割しない判断(超過理由の台帳 — 行が消えたら分割済み)

- **この見出しの節は `cargo xtask structure` が機械で読む** — **ファイルの恒久免除になるのは行頭が `- **` + バッククォート付きパスの箇条書きだけ**(パス後方一致)。fn 単位の項のようにパスを文中で挙げるだけの行は免除にならない。太字を落とすと免除が外れて count が赤くなる(失敗の向きはこちら側で正しい)。見出し文字列を変えると免除が全部外れる(節が無い時はツールがエラーで止まる)。範囲は次の `## ` 見出しまで

- **`ui/AutoActDriver.qml`(2427 行)は割らない** — `runAutoAct()` の分岐が同じファイルの Timer を id で名指ししており、動詞の beat と dispatch は同じコンポーネントスコープに居ないと繋がらない(割れば Timer を property で渡し直すことになり、ページから渡す 28 本がもう一段増える)。中身は 1 動詞 1 分岐 = 増えるのは分岐の本数だけで、責務は増えない。末尾の 3 本(`PG_SCROLL_TO` / `PG_AUTO_SCROLL` / `PG_AUTO_SELECT`)だけは dispatch を通らず自分のフラグで動く
- **`ui/WindowAutoActDriver.qml`(1100 行)も同じ理由**(窓側の beat と `begin()` の dispatch)
- **`ui/RepoPage.qml`(1678 行)は再設計まで割らない** — 出せる純移動は出し切った(レイアウト = `PageLayout`、コミットメニューの状態 = `CommitMenuState` まで出た)。残るのは 1 タブ分の状態機械そのもので、§分割の各論の「page の状態を子から書かない」再設計とセットでしか動かせない。**再設計が終わったら台帳から基準線ラチェットへ戻す**(この行を消して pin し直す)
- **`ui/WipPane.qml`(872 行)は残りを割らない** — 部品化済み(MessageEditor / OpExitCard / TreeViewToggle)の外に残るのは選択・ステージ・EOL 指しの機構で、全員が `wipList.itemAtIndex` 走査とデリゲート再利用前提の鍵(`<bucket>:<path>`)を共有し、`RepoPage`(`chosenRows` = menuFileCount)と自動化(`chooseOnly` / `rowAt` / `rowFor` / `pointEol`)がその API を直接叩く — これ以上は list と鍵の渡し直し配線だけが増える
- **`models/repo_tab/qobject.rs`(768 行)は割らない** — `#[qobject]` ブロックは QMetaInfo の一貫性で 1 型 1 ブロック 1 ファイル(規約 §分割)。中身は `qproperty!` 53 本(128 行)+ スロット 76 本の署名 + その doc 168 行で、**本体は全て素の impl へ委譲済み**(`drain` / `ops_stage`(集めたパスの 5 スロットは `drain_paths` 1 本)/ `ops_conflict` / `ops_remote` / `ops_config` / `state` の commit・reset)— 残るのは Qt に見せる面と 1〜3 行の転送だけなので、これ以上はスロットを減らすしか縮め方が無い
- `verify/verbs.rs` の `must_say` は 104 行(上限 100)— 1 動詞 1 行の索引で、伸びるのは動詞が増えた時だけ。**割ると「どっちの半分に居るか」という質問が 1 つ増える**(順序も意味を持たない = 分け目に理由が立たない)。`#[expect(clippy::too_many_lines)]` を貼る
- `parse/diff/parse.rs` の `parse_patch` は 196 行(上限 100)— 6 本の可変ローカルを全分岐が触るので、分解は移動と別の変更として理由を立てる。`#[expect(clippy::too_many_lines)]` を貼る
- **`#[expect]` の不発判定は lint の有効・無効ではなく「その lint が実際に発火するか」** — 未有効(pedantic)のままでも長い fn なら不発にならない。不発になるのは fn が閾値を下回った時だけで、`-D warnings` 下ではそれが error になる(実測: 247 行の fn に lint 未有効で貼って無音、短い fn に貼って unfulfilled)
