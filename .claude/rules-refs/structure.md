# 分割・共通化 各論(1 項目 1 行。自動ロードされない — 触る項を Grep で引く)

## 分割の各論

- **`session/mod.rs` の `use` ブロックは兄弟 15 ファイルの共有 prelude** — 各ファイルの `use super::*` が `Oid` / `GitError` / `Segment` 等をここ経由で引くので、mod.rs 自身が使わなくなった import も消せない(glob 経由の利用を rustc は「使用済み」と数えるため警告も出ない = 消すと一斉に壊れて初めて分かる)
- **`session/repo.rs` は作れない** — `crate::repo`(RepoInfo / open)と名前が衝突する。`RepoSession` の置き場は `session/repo_session.rs`
- **mod.rs から出す型の `pub(super)` は `pub(crate)` と書き写す** — session は crate 直下なので mod.rs の `pub(super)` = `pub(crate)`。移動先で `pub(super)` にすると session 内へ狭まる(逆に mod.rs で private だった型は移動先で `pub(super)` にすると元と同じ範囲)

## 分割しない判断(超過理由の台帳 — 行が消えたら分割済み)

- **`ui/AutoActDriver.qml`(1769 行)は割らない** — `runAutoAct()` の分岐が同じファイルの Timer を id で名指ししており、動詞の beat と dispatch は同じコンポーネントスコープに居ないと繋がらない(割れば Timer を property で渡し直すことになり、ページから渡す 27 本がもう一段増える)。中身は 1 動詞 1 分岐 = 増えるのは分岐の本数だけで、責務は増えない
- **`ui/WindowAutoActDriver.qml`(630 行)も同じ理由**(窓側の beat と `begin()` の dispatch)
