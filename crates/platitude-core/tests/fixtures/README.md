# Parser fixtures

`.bin` ファイルは**実 git の生出力**(バイト列そのまま)。手編集禁止。

再生成(git のバージョン更新等で差分が出た場合):

```bash
cargo test -p platitude-core --test it -- --ignored capture
```

実行後、`git diff` で差分をレビューしてからコミットする。生成シナリオは
各テストファイルの `capture_fixtures` テストに定義されている(決定的な
identity / timestamp を使うため、同一 git バージョンなら出力は安定)。
