# ローカル性能検証

固定コーパスの通常計測は [perf-windows-x64.md](perf-windows-x64.md) の条件を使う。
値はOS・renderer・画面・cache・caseを揃えたものだけ比較する。

## ケース

連続するケースは周回境界も含めて異なるOIDにする。同じ行の再クリックは改名などの
別ジェスチャーになる。

`--cases <file.tsv>` はヘッダー無しの4列: `name<TAB>full-oid<TAB>path<TAB>completion`。
空行と `#` 行は無視する。nameは英数字・`-`・`_`、OIDは完全長、pathはそのコミットの変更
ファイル一覧に載るパス。completionは `raw` または `coloured`。一覧の順に選択・diffを開き、
`--cycles N` でN周する。反復には異なるOIDが2つ以上必要。

```
cargo xtask perf --repo <corpus> --cases <cases.tsv> --cycles 50 --runs 5
cargo xtask perf --repo <corpus> --cases <scroll-cases.tsv> --diff-scroll --runs 5
cargo xtask perf --repo <corpus> --select-oid <full-oid> --file <path> --completion coloured --runs 5
```

実ウィンドウを使うため、席から実測する時は明示的な起動許可と `PGG_ALLOW_GUI=1` が必要。
合否はケースごとに判断する。プロセス内反復と別プロセスのrunは別の軸。

通常の変更、多数ファイル変更、小さい変更を含む大きなソース、長いdiffを固定ケースにする。
画像・非対応言語は `raw` として別ケースにする。`coloured` は最終 `DiffModel.coloured` が
立つ対象専用。時間切れは失敗のまま記録する。rawが測るのは画像デコード前まで。

`perf_diff_frame` は対象diffの初回表示、`perf_colour_frame` は同じfingerprintの最終色付けが
反映されたフレーム。
`--diff-scroll` は各操作で可視diffを2秒スクロールし、正の移動量と終了後の可視行を要求する。
短くoverflowのないdiffは失敗なのでスクロール用一覧は分ける。グラフの12秒計測とは別系列。
反復と併用する時は、全操作の計測時間を含む `--watchdog-ms` を指定する。

## cacheとA/B

- `--cache warm` が既定。同じ経路の捨てrunで温める。
- `--cache first --runs 1` は、このinvocationの最初のプロセス。OS cacheの状態は
  別に確かめる。
- `--cache cold --cold-prepare <executable>` は利用者指定のOS固有プログラムを各run直前に
  実行する。repositoryと測定binaryの2引数を渡し、成功終了を準備完了とする。
  準備失敗・期限超過は測定失敗。
  プログラムがどのcacheをどう冷却したかを記録に併記する。成功終了が示すのは準備が
  走ったことだけ。

```
cargo xtask perf --repo <corpus> --at <A> --compare <B> --runs 3 --cases <cases.tsv>
```

A/Bの `--runs` はABBAブロック数。3なら各6回。
画面・Hz・renderer・features・corpus・GPUの記録が変われば比較を中止する。古いcommitが新しい
計測口を持たない場合、要求したケースや色付けの証跡欠落を失敗とする。

## 短いメモリ峰

100msのWorkingSet最大とは別にWindowsの `PeakWorkingSet64`、Linuxの `VmHWM` を
`memory.csv` の `os_peak_working_set_bytes` に記録する。OSが保持するプロセス寿命中の
最大常駐量で、サンプル間の短い峰を検出する。
private commitの峰・割当回数・発生時刻は分からないので、
差が問題になる場合は別の診断runでOSの割当トレースを採る。帰属取得の計測負荷は別runへ
置き、OS最大は素の値で記録する。

## workload別の比較

`cargo xtask corpus --against <reference>` は以下を別profileとして出力する。
プロファイルは軸ごとに独立して読む。

| profile | 入力 | 対応する性能 |
|---|---|---|
| startup/status | 追跡数・directory・index・無視ファイル・git設定 | 起動・status完了 |
| refs | refs数・注釈タグ比・remote階層 | refs読込・ナビゲーション |
| graph/scroll | lane幅・chip数・文字量・フォールバック | frame分布・停止・常駐量 |
| diff | 変更数・ソースサイズ各分位・言語・変更量 | raw/coloured完了・diff scroll |

## OS別の証跡

既定の出力先は `target/perf/<os>-<arch>/<run>/`。
Windowsの正本は既存記録。Linux と macOS arm64 は実機待ちで、取る物と残す物は
[P5-確認事項.md](../../internal-docs/P5-確認事項.md) §23。
