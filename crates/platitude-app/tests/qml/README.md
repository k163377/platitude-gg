# QMLのテスト

`tst_*.qml` はRustの登録型を使わず、本体のQMLモジュールをQtの `qmltestrunner`
に読ませる。ここにあるのはRustのテストが届かないもの — Canvasが撮影に負っている
paintが済んだかどうかは、モデルではなくアイテムツリーの事実。

```
cargo xtask qmltest
cargo xtask linux qmltest
```

**手順の正本はコード** (`crates/xtask/src/qmltest.rs`)。importツリーの組み方
(`platitude.ui` はディレクトリ名で解決するので `src/ui` のままでは読めない)、
出力の受け取り方(Windowsではリダイレクトしたstdoutが空で返る)はそこにある。

`cargo xtask gate` は、本体のQMLモジュールかこのディレクトリに変更が届いた時に
両OSで走らせる。

**待ちの規則は `cargo xtask waits` が機械で見る**(gate の常時ステップ)。`wait(ms)` は
使わない(答えを待つなら `tryCompare` / `tryVerify`)。`tryCompare` / `tryVerify` に独自の
timeout を渡さない — runner の既定が共通の期限で、message だけ渡すなら timeout の席に
`undefined` を置く。`waitForRendering` / `waitForItemPolished` の戻り値は `verify()` で読む
(false は「描かれなかった」)。残す時は行の上に `// waits(<purpose>): <reason>`
(purpose は `.claude/rules/core.md` §非同期・並行テスト)。

`tst_ink.qml`: 親が非表示の場合、初回描画前の非表示化→再表示、実際のpaint完了後の
再表示を検証する。前2ケースは修正前に `Ink.owed = 1`(期待0)で失敗する。
固定sleepは使わない。

`tst_menudismiss.qml`: 入れ子のカードから呼んだ Qt の `Menu.dismiss()` が上のメニューまで
全段畳むこと、親を閉じれば開いているカードも落ちることを固定する。製品のカードはこの
1 発だけで畳み、上へ `closeRequested` を返さない(app-ui.md §メニューを閉じるのは自分)。
