# アバター配線の実測(Windows x64)

グラフの log に著者アドレスを足し(`%an` → `%aN` + `%aE`)、割り当て画像を
全ノードへ描くようにした分のコスト。

- 対象: `JetBrains/kotlin`(`rev-list --count HEAD` = 138,915、`.mailmap` **無し**)
- 環境: Windows 11 / x64 / release / ウォームキャッシュ / offscreen
- 測り方: `cargo xtask verify-ui wip --repo <kotlin>` の `first_chunk_ms`、
  および `git log` 単体の壁時計(`--max-count=2000`)

## log フォーマットの変更ぶん

| format | 3 回の実測 | stdout |
|---|---|---|
| 旧 `%H %P %an %at %s` | 71 / 60 / 56 ms | 346,327 chars |
| 新 `%H %P %aN %aE %at %s` | 58 / 53 / 56 ms | 407,057 chars |

**時間差はノイズに埋もれる**(mailmap 未設置なので `%aN` / `%aE` は生の値へ
素通しになり、引き当ての仕事が無い)。増えるのは出力 **+18%** = 1 行あたりの
アドレスぶんで、これは `StrPool` に intern されるので行数ではなく**著者数**に
比例して残る。

**`.mailmap` を持つリポジトリは引き当てのぶんだけ git 側が遅くなる** — その値は
未実測。名寄せを自前で持たない代償として受け入れる(規約 §アバターを与える)。

## 画像を描くぶん

| 状態 | first chunk |
|---|---|
| 誰にも画像なし | 318 / 392 / 730 / 804 ms |
| 最頻著者に 1 枚(2,000 行中 9 行が該当) | 432 / 560 / 788 ms |

**同じ帯に収まる**。予算(起動→グラフ初回表示 3s)に対して十分な余裕がある。
ばらつきが大きいのは測定中に別のビルドが走っているため。

**デコードは URL ごとに 1 回**(Qt の pixmap キャッシュ)なので、行数ぶんの
デコードにはならない。画像は取り込み時に 256px へ正規化される(`picture::normalize`。
実測は [avatar-shrink](avatar-shrink-windows-x64.md))。offscreen の fps は表示性能を
表さない(= verify-ui スキル)ため、**実ウィンドウでの fps 確認は P5 の実機ゲートへ**。
