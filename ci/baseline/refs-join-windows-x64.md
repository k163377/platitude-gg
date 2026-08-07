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

| | 1 join |
|---|---|
| 修正前 | **2512.4 ms** |
| 修正後 | **39.0 ms** |

内訳(修正前 → 後):

| 処理 | 前 | 後 |
|---|---|---|
| `build_label_map` | 1544 ms | 25 ms |
| `build_snapshot` | 916 ms | 21 ms |
| リモートタグ索引の構築 | 19 ms × 2(join ごと) | 0(fetch 時のみ) |
| スナップショットの deep clone(セクション 3 本へ配る) | 10 ms | 0(`Arc` 共有) |

原因は**ループの中の走査**が 2 か所(いずれもタグのリモート状態を出す配線で
入った):

- `build_label_map`: リモートのタグ 1 件ごとに `refs` 全体を `find` して手元の
  同名タグを探していた = 45,782 × 53,614
- `build_snapshot`: リモートのタグ 1 件ごとに構築中の `snapshot.tags` を `any`
  で走査していた = 45,782 × 45,782

どちらも `RefJoins`(`session.rs`)の索引 1 本に置き換えた。同じ形の走査は
`refs::spoken_for_remote` にもあり(ローカル 1 本ごとにリモートブランチ全体を
走査)、`refs::RemoteBranches` の索引にまとめてある — ローカル 201 本で
6.8 ms → 1.5 ms。

**この数字は git のコストには入っていない**。1 tick の git 側は
[poll-cost-windows-x64.md](poll-cost-windows-x64.md) の通り kotlin 級で約 760ms
(status 側が支配)。修正前はその上に 2.5 秒が乗り、10 秒間隔のポーリングが
1 コアの 3 割を焼き続けていた。
