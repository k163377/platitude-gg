# 画面表示中ポーリングのコスト実測(Windows x64)

`Metrics.pollIntervalMs` を決めるための実測。計測日: 2026-08-05。

1 tick で走るのは refs 側 3 本(`for-each-ref` / `symbolic-ref` / `config --get-regexp`)と
status 側 2 本(`status --porcelain=v2 -z --branch -uall` / 進行中操作の検出)。
両側は `tokio::join!` で並走するため、体感コストは**遅い方の側**で決まる。
ウォームキャッシュ、3 回の最小値。

| リポジトリ | commits | refs | 1 tick の実時間 |
|---|---|---|---|
| `JetBrains/kotlin` | 226,815 | 53,535 | **約 760ms**(status 740ms / refs 側 380ms) |
| `jackson-databind` | 13,454 | 340 | 約 61ms |
| `platitude-gg` | 142 | 4 | 約 46ms |

- 10 秒間隔での占有率は kotlin 級でも 1 コアの 8% 弱。実在する最大級のリポジトリの
  値なので、間隔を 10 秒から動かす理由は無いと判断する
- 支配項は `git status -uall` で、refs の本数ではなく**ワーキングツリーの規模**で効く。
  `-uall` は hunk / 行ステージングのために必要(`-unormal` はディレクトリに畳む)
- ポーリングは前回が終わるまで再入せず、書き込み中は走らない(`RepoSession::refresh_poll`)。
  したがって 1 tick が間隔より長いリポジトリでは、間隔が自然に伸びるだけで積み上がらない
- 変化が無い tick はイベントを一切出さない(グラフの walk は refs / status が
  動いた時だけ)。上表は**その静止時のコスト**
