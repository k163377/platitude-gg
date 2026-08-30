# P1着手: 表示条件とフレーム時系列

2026-08-30。P0と詳細要求の修正をレビューし、`97f072e6` までmainへ反映した後、席bを再取得して着手。
これは [改善計画](../../internal-docs/メモリと操作応答の改善計画.md) の残計測項目であり、性能改善の結果ではない。

## 保存する証拠

- `cargo xtask perf` は各プロセスの前後に `display-before.txt` / `display-after.txt` を保存する。
  Windowsではアクティブな各画面名、物理ピクセル寸法、設定の整数Hzを読み取る。設定変更はしない。
  API未対応のOSや採取失敗は `unavailable` と明記し、0Hzや60Hzで埋めない。
- `app.log` の `perf_display` は実際のQMLウィンドウの画面名、model/manufacturer、DPR、論理DPI、
  画面と窓の論理座標・寸法、visibilityを、開始・変化・完了で記録する。通常起動では有効にならない。
- `--trace-frames` は12秒窓のフレーム到着をアプリ共通の単調時計で保持し、窓が閉じてから
  `perf_frame index=... clock_ms=... interval_ms=...` を時系列順に出力する。
  先頭間隔はスクロール開始から最初の通知まで。`perf_scroll_begin` / `perf_viewport` も同じ時計を使う。
  要求した系列の行数が `scroll_bench` の `frame_count` と合わないrunは失敗になる。
  ログ先頭の親プロセス到着時刻は書き出し時刻なので、各フレーム時刻の代用にしない。
- 詳細系列は診断専用。書き出しは最終描画やメモリ採取に影響し得るので、予算合否用の通常runと分ける。
  per-frame callbackではファイル書込やログ整形をしない。

## 表示条件の限界

[EnumDisplaySettingsW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsw)
の現在設定を使う。`DEVMODEW.dmDisplayFrequency` は整数で、0/1は既定値を表し具体的Hzではないためnull扱い。
小数Hz・VRR・実際のパネル走査は計測していない。
[QML Screen](https://doc.qt.io/qt-6.10/qml-qtquick-screen.html) が公開する情報にはHzがないため、
その画面名をOSの採取結果と対応付ける。実測FPSからHzを逆算していない。

前後snapshotの一致だけでは、途中で設定を変更して元に戻したケースを検出できない。
QML側は画面・DPI・窓位置等の変化を記録するが、Hzだけの一時変更は残課題。
画面境界をまたぐ窓や不明なHzを含むrunも自動的に比較可能とはしない。
同じ画面・設定Hz・DPI・窓位置／寸法に固定した対照を取り、変化や欠損があれば比較から除外する。

`frameSwapped` はGUI側へ届いた通知の間隔。GUIスレッドの停止とrender/presentの停止をこれだけで区別しない。
CPU/待機トレースやUI適用区間との相関を採るための時系列であり、物理走査の完了証拠ではない。

## 実機の事前調査

読み取り専用の同じWindows採取処理で、次の現在設定を得た。
証拠は席bの `target/perf/p1-display-preflight.txt`。

| 画面 | primary | 寸法(px) | 設定Hz |
|---|---|---|---|
| `\\.\DISPLAY1` | false | 1920×1080 | 100 |
| `\\.\DISPLAY2` | true | 1920×1080 | 180 |
| `\\.\DISPLAY3` | false | 1920×1080 | 100 |

過去のrunがどの画面に載っていたかは遡及できない。約98fpsの既存値をこの表へ後付けで対応させない。

`C:\Windows\System32\wpr.exe` があり、読み取り専用の `wpr -status` は未記録状態を返した。
PATHおよびWindows Kitsのインストール先ではWPA、xperf、UMDH、gflagsを見つけられず、
使用中QtディレクトリでもQt Core/Gui/QuickのPDBは見つかっていない。
WPR helpではPIDの存続中だけのheap snapshot設定が可能と確認できたが、収集は未開始。
OS設定やIFEO、他プロセスのトレースは変更していない。
現時点ではnativeの生存確保スタックへの帰属、UI仕事と長いフレームの原因特定は未完了。

## 診断の実行例

実ウィンドウの計測が許可された席で、他のビルド／テストを止めて実行する。
出力先は毎回新しいディレクトリを指定する。

```text
cargo xtask perf --repo C:/Users/wrongwrong/IdeaProjects/kotlin --no-select --runs 1 --trace-frames --breakdown --settle-ms 1000 --output target/perf/p1-frame-diagnostic
```

1回のcold runと1回のkept runで記録経路を確認する例であり、200操作の応答分布や改善のA/B試験を代替しない。

## 検証

`cargo xtask check` に `perf none`、`perf scroll-none`、`perf scroll`（いずれも `--preset perf`）を指定し、
段2をPASS（3分18秒）。Windows/Linuxの3ケースずつのPNGを確認した。boardを更新済み。
ログは `target/perf/p1-instrumentation-check/check-logs` に退避し、過去runのログと分離した。
アプリ共通時計からフレーム間隔への変換と、要求した系列の欠落を拒否するテストも通る。
offscreenの画面名が空であることをログで確認し、ここでの値を実機性能値には使っていない。
