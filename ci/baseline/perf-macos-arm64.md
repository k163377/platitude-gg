# 性能実測記録(macOS arm64)

状態: 実機baseline未取得。手順は [perf-local.md](perf-local.md)。
現行xtask perfはmacOS用メモリsampler未実装のため明示的に拒否する。実測にはsamplerと
表示条件収集の実装・実機検証が必要。

| 指標 | 実測 | 判定 |
|---|---|---|
| 起動→可視グラフ | 未取得 | 未判定 |
| case別raw/coloured応答 | 未取得 | 未判定 |
| graph/diff frame分布 | 未取得 | 未判定 |
| OS固有定義の常駐量・peak・操作後保持 | 未取得 | 未判定 |

取得時はcommit/exe hash、corpus token、ケース一覧、OS/SoC/Qt、renderer、画面/Hz/DPI、
窓寸法、cache準備、A/B順序、証跡パスを残す。Windowsのメモリ定義は確認してから使う。
