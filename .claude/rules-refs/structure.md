# 分割・共通化 各論(1 項目 1 行。自動ロードされない — 触る項を Grep で引く)

## 分割の各論

- **`session/mod.rs` の `use` ブロックは兄弟 15 ファイルの共有 prelude** — 各ファイルの `use super::*` が `Oid` / `GitError` / `Segment` 等をここ経由で引くので、mod.rs 自身が使わなくなった import も消せない(glob 経由の利用を rustc は「使用済み」と数えるため警告も出ない = 消すと一斉に壊れて初めて分かる)
- **`session/repo.rs` は作れない** — `crate::repo`(RepoInfo / open)と名前が衝突する。`RepoSession` の置き場は `session/repo_session.rs`
- **mod.rs から出す型の `pub(super)` は `pub(crate)` と書き写す** — session は crate 直下なので mod.rs の `pub(super)` = `pub(crate)`。移動先で `pub(super)` にすると session 内へ狭まる(逆に mod.rs で private だった型は移動先で `pub(super)` にすると元と同じ範囲)
- **insta のスナップショットはソースファイル基準の `snapshots/` を見る** — `foo.rs` → `foo/bar.rs` へ動いたテストは置き場(`src/snapshots/` → `src/foo/snapshots/`)とファイル名(モジュールパス)の両方が変わる。中身は不変なので `INSTA_FORCE_UPDATE=1 cargo test` で `source:` 行だけ再生成する(手編集は禁止 — CLAUDE.md Rust 規約)
- **`mod tests` を専用ファイルへ持ち上げる時、複数行文字列リテラルの中身は字下げされていない** — 一律 dedent は fixture を壊す。宣言行だけ下げ、リテラルの行はそのまま移す(`parse/diff/testkit.rs` の `PATCH` が実例)
- **外部クレートと同名のモジュールを作ったら親の `use` は `self::` で書く** — `settings/toml.rs` があると mod.rs の `use toml::…` は extern prelude と衝突して E0659。子ファイル側は現モジュールに `toml` が無いので素の `use toml::…` のままでよい
- **移動だけのコミットは、移動先を決める前に「どの行がどこへ行くか」を機械で突き合わせる** — 元ファイルの全行が行き先ちょうど 1 つに入り、落ちるのは列挙した scaffolding(バナー・共有 `use`・`mod tests` の包み)だけ、と検証してから書き出す。取りこぼし・二重取りはこれでしか出ない

## 分割しない判断(超過理由の台帳 — 行が消えたら分割済み)

- **この見出しの節は `cargo xtask structure` が機械で読む** — **ファイルの恒久免除になるのは行頭が `- **` + バッククォート付きパスの箇条書きだけ**(パス後方一致)。fn 単位の項のようにパスを文中で挙げるだけの行は免除にならない。太字を落とすと免除が外れて count が赤くなる(失敗の向きはこちら側で正しい)。見出し文字列を変えると免除が全部外れる(節が無い時はツールがエラーで止まる)。範囲は次の `## ` 見出しまで

- **`ui/AutoActDriver.qml`(1769 行)は割らない** — `runAutoAct()` の分岐が同じファイルの Timer を id で名指ししており、動詞の beat と dispatch は同じコンポーネントスコープに居ないと繋がらない(割れば Timer を property で渡し直すことになり、ページから渡す 27 本がもう一段増える)。中身は 1 動詞 1 分岐 = 増えるのは分岐の本数だけで、責務は増えない
- **`ui/WindowAutoActDriver.qml`(630 行)も同じ理由**(窓側の beat と `begin()` の dispatch)
- **`ui/WipPane.qml`(739 行)は残りを割らない** — 部品化済み(MessageEditor / StashOptionsCard / OpExitCard / TreeViewToggle)の外に残るのは選択・ステージ・EOL 指しの機構で、全員が `wipList.itemAtIndex` 走査とデリゲート再利用前提の鍵(`<bucket>:<path>`)を共有し、`RepoPage`(`chosenRows` = menuFileCount)と自動化(`chooseOnly` / `rowAt` / `rowFor` / `pointEol`)がその API を直接叩く — これ以上は list と鍵の渡し直し配線だけが増える
- `parse/diff/parse.rs` の `parse_patch` は 196 行(上限 100)— 6 本の可変ローカルを全分岐が触るので、分解は移動と別の変更として理由を立てる。`#[expect(clippy::too_many_lines)]` を貼る
- **`#[expect]` の不発判定は lint の有効・無効ではなく「その lint が実際に発火するか」** — 未有効(pedantic)のままでも長い fn なら不発にならない。不発になるのは fn が閾値を下回った時だけで、`-D warnings` 下ではそれが error になる(実測: 247 行の fn に lint 未有効で貼って無音、短い fn に貼って unfulfilled)
