# refs の join のコスト実測(Windows x64)

`publish_refs` が 1 回で払う **git 実行以外**のコスト。ポーリング 1 tick と
書き込み 1 件ごとに必ず走る。計測日: 2026-08-08。

- 対象: `JetBrains/kotlin` full clone(refs 53,614 本 = タグ 45,782 /
  リモートブランチ 7,831 / ローカル 1)
- リモートのタグは **fetch 済みの定常状態**を置く(`ls-remote --tags` が
  手元のタグを全部返した状態 = `origin` が 45,782 件を名乗る)
- 再現: `PG_PERF_REPO=<repo> cargo test -p platitude-core --release
  refs_join_at_scale -- --ignored --nocapture`(`session::tests`)
- release ビルド、ウォームキャッシュ

| 処理 | 1 join |
|---|---|
| **合計** | **39.0 ms** |
| `build_label_map` | 25 ms |
| `build_snapshot` | 21 ms |
| リモートタグ索引の構築 | 0(fetch 時のみ。45,782 名で 19ms) |
| スナップショットのセクション 3 本への配布 | 0(`Arc` 共有) |

`RefJoins`(`session.rs`)の索引 1 本を使わない形は 1 join が **2512 ms**
— **ループの中の走査**が 2 か所(リモートタグ 1 件ごとに `refs` 全体を
`find` = 45,782 × 53,614 / 構築中の `snapshot.tags` を `any` で走査 =
45,782 × 45,782)。同じ形の走査は `refs::spoken_for_remote` にもあり
(ローカル 1 本ごとにリモートブランチ全体を走査)、`refs::RemoteBranches` の
索引で 6.8 ms → 1.5 ms(ローカル 201 本)。

**この数字は git のコストには入っていない**。1 tick の git 側は
[poll-cost-windows-x64.md](poll-cost-windows-x64.md) の通り kotlin 級で約 760ms
(status 側が支配)。
