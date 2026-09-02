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

`tst_ink.qml`: 親が非表示の場合、初回描画前の非表示化→再表示、実際のpaint完了後の
再表示を検証する。前2ケースは修正前に `Ink.owed = 1`(期待0)で失敗する。
固定sleepは使わない。
