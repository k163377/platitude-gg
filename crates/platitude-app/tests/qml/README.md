# QMLのテスト

`tst_*.qml` は本体のQMLモジュールをQtの `qmltestrunner` に読ませる(Rustの登録型は
入らない)。ここにあるのはRustのテストが届かないもの。

`stubs/` は Rust の singleton の代役(`GitFacts`)— 行のデリゲートが組み立ての途中で読む、
引数だけから答える事実に限る。規則の正本は Rust 側のテストで、代役の答えに寄りかかった
テストは代役を試していることになる。

```
cargo xtask qmltest
cargo xtask linux qmltest
```
