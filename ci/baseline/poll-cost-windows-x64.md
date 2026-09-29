# 画面表示中ポーリングのコスト実測(Windows x64)

`Metrics.pollIntervalMs` を決めるための実測。

1 tick は refs 側(`for-each-ref` 1 本)と status 側(`status --porcelain=v2 -z --branch -uall` ほか)を
`tokio::join!` で並走するため、体感コストは**遅い方の側**で決まる。
ウォームキャッシュ、3 回の最小値。下表の refs 側は `symbolic-ref` /
`config --get-regexp` も 1 tick ごとに撃っていた時の値 = 現在は上限。
status 側の実測は 2 本の時のもの(3 本目は短いローカル config 1 本。印が upstream と別のリモートへ送るブランチの
間だけ 4 本目に push 先との数の `for-each-ref` 1 本 — perf corpus で数えない時 19ms・6 万コミットを数えて 40ms、
同じ corpus の `status` は 380ms)— 支配項は変わらない。

| リポジトリ | commits | refs | 1 tick の実時間 |
|---|---|---|---|
| `JetBrains/kotlin` | 226,815 | 53,535 | **約 760ms**(status 740ms / refs 側 380ms) |
| `jackson-databind` | 13,454 | 340 | 約 61ms |
| `platitude-gg` | 142 | 4 | 約 46ms |

- 10 秒間隔での占有率は kotlin 級でも 1 コアの 8% 弱。実在する最大級のリポジトリの
  値なので、間隔は 10 秒と判断する
- 支配項は `git status -uall` で、**ワーキングツリーの規模**で効く。
  `-uall` は hunk / 行ステージングのために必要(`-unormal` はディレクトリに畳む)
- ポーリングは前回が終わるまで再入しない — 1 tick が間隔より長いリポジトリでは、
  間隔が自然に伸びるだけで積み上がらない
- 上表は変化が無い tick(グラフの walk は refs / status が動いた時だけ)= **静止時のコスト**
