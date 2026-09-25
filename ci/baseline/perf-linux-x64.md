# 性能実測記録(Linux x64)

状態: 実機baseline未取得。
手順は [perf-local.md](perf-local.md)。取得時はcommit/exe hash、corpus token、ケース一覧、
git/Qt/driver、renderer、GPU、画面/Hz/DPI、窓寸法、cache準備、順序、証跡パスを記録する。

| 指標 | 実測 | 判定 |
|---|---|---|
| 起動→可視グラフ | 未取得 | 未判定 |
| case別raw/coloured応答 | 未取得 | 未判定 |
| graph/diff frame分布 | 未取得 | 未判定 |
| RSS最大 / VmHWM / 操作後RSS | 未取得 | 未判定 |

private列のVmDataはWindowsのPrivate Bytesとは違う。差分は同じOSの中で取る。
Linuxコンテナのverify-uiは描画契約の検証。この表を埋めるのは実機の実測。
