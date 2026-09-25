# QMLのテスト

`tst_*.qml` は本体のQMLモジュールをQtの `qmltestrunner` に読ませる(Rustの登録型は
入らない)。ここにあるのはRustのテストが届かないもの。

```
cargo xtask qmltest
cargo xtask linux qmltest
```
