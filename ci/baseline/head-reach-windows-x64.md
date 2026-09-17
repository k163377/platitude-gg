# ブランチ先端を他が掴んでいるかの実測(Windows x64)

drop 行を長押しにするかクリックにするかの判定
(`reachable::reached_without_branch`)が払う git のコスト。**refs が動いた
refresh と書き込み 1 件ごとに 1 回**走る。計測日: 2026-08-08。

- 対象: `JetBrains/kotlin` full clone(refs 53,672 本 = タグ 45,846 /
  リモートブランチ 7,824 / ローカル 1、コミット 138,915)
- commit-graph chain あり、ウォームキャッシュ、5 回の平均と最良
- 「掴まれている」= HEAD の先端(`origin/master` が届く)/「掴まれていない」=
  同じ先端の上に `commit-tree` で 1 つ載せた参照なしコミット

| walk に入れる ref | 掴まれている | 掴まれていない |
|---|---|---|
| `--branches --remotes --glob=refs/stash*` | **85 ms**(最良 83) | **86 ms**(最良 82) |
| 同 + `--tags` | 501 ms | 501 ms(最良 498) |

**タグを walk から外したのはこの 6 倍差**。内訳を分けて測ると
`--tags` だけで 478 ms、`--remotes` だけなら 83 ms — コストは
**タグの本数に比例**する(性能予算「refs の本数から
独立」)。

先端にちょうど乗っているタグは refs の一覧
(`reachable::a_ref_sits_on_head`)が 0 プロセスで拾うので、walk が取り
こぼすのは**先端より先に居るタグ**だけ。取りこぼすと長押しが出る =
クリックで済む行に印が出る側へ倒れる。

## 他に測った形(不採用)

| 形 | 掴まれている | 掴まれていない |
|---|---|---|
| `for-each-ref --contains=<tip> --count=2` | 148 ms | 706 ms |
| `for-each-ref --contains=<tip>`(件数制限なし) | 980 ms | — |

`for-each-ref --contains` は refs/stash を見るが**古い stash
(`refs/stash` の reflog)は見えず**、掴まれていない側で 8 倍遅い。

## 走らせる回数を減らす形

- **先端にちょうど乗っている ref は refs の一覧で判る**ので、そこで決まる時は
  walk を撃たない(`session::remember_head_hold`)。push 済みで追跡 ref が
  先端に居る = 最も多い「クリックでよい」状態が 0 プロセスで済む
- 起動から使い続ける間、走るのは**refs が動いた refresh と書き込みの後だけ** —
  ポーリングの tick には載せていない
