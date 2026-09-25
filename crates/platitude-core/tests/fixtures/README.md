# Parser fixtures

`.bin` ファイルは**実 git の生出力**(バイト列そのまま)。再生成のみ。

再生成(git のバージョン更新等で差分が出た場合):

```bash
cargo test -p platitude-core --test it -- --ignored capture
```

実行後、`git diff` で差分をレビューしてからコミットする。生成シナリオは
`tests/it/worktree_state.rs` の `capture_fixtures` テストに定義されている。
