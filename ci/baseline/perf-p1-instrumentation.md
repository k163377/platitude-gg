# P1着手: 表示条件とフレーム時系列

2026-08-30。P0と詳細要求の修正をレビューし、`97f072e6` までmainへ反映した後、席bを再取得して着手。
これは [改善計画](../../internal-docs/メモリと操作応答の改善計画.md) の残計測項目であり、性能改善の結果ではない。

## 保存する証拠

- `cargo xtask perf` は各プロセスの前後に `display-before.txt` / `display-after.txt` を保存する。
  Windowsではアクティブな各画面名、物理ピクセル寸法、設定の整数Hzを読み取る。設定変更はしない。
  API未対応のOSや採取失敗は `unavailable` と明記し、0Hzや60Hzで埋めない。
- `app.log` の `perf_display` は実際のQMLウィンドウの画面名、model/manufacturer、DPR、論理DPI、
  画面と窓の論理座標・寸法、visibilityを、開始・変化・完了で記録する。通常起動では有効にならない。
- Windowsの `memory.csv` は、自分が起動したPIDの `MainWindowHandle` に対応する画面IDを
  `display_name` 列へ採取する。Qtのfriendly nameとは別であり、このIDをOSのHzと照合する。
  HWNDがまだ無い時は `unknown` と記録し、primaryのHzで代用しない。
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
[QML Screen](https://doc.qt.io/qt-6.10/qml-qtquick-screen.html) が公開する情報にはHzがない。
実機ではQtの名前が `PL2492H (2)`、Windowsが `\\.\DISPLAY1` 等の形式で異なるため、
名前同士で結合しない。PIDのウィンドウから得るOSのIDを経由する。実測FPSからHzを逆算していない。

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

## 最初の実機診断と、次に切り分ける区間

`cc5d8eb6` のrelease + memprobeで上の例を実行した。ソースはクリーン、Kotlin HEADは
`db1bc5055f24c7227a7d2cc37d058432007d288f`、refsは54,268本。
このタスクのビルド／テストとの同時実行はしていない。
`target/perf/p1-frame-diagnostic` に各runのログ、memory.csv、抽出したframes.csv、前後画面設定、
exe hash、repo/source情報を保存した。exe SHA-256は
`C87A46443F6AA93A399A9162ECB475A2094BDE45F7E75D81B7D2B94E30423172`。

| run | frame行数 | 最長間隔 | スクロール開始からその間隔の開始／終了 |
|---|---|---|---|
| 0（cold、除外） | 1,160 | 292.564ms | 7,725.421 / 8,017.985ms |
| 1 | 1,149 | 190.034ms | 7,720.131 / 7,910.166ms |

各系列のindex連続性・時刻の単調性・件数を確認した。両runとも100ms超は1件。
通常の間隔が約10msであることと、約7.72秒後に始まる長い間隔は別の観測として扱う。
この2回だけでは同じ原因だと確定しない。該当区間のviewport/行、UI適用・QML処理、CPU/待機の
トレースを重ね、同じ行を再訪した場合と新しい行の場合、memprobeを外した場合を比べる。

Qt側では起動直後に `PL2470H` から `PL2492H (2)` へ移り、スクロール窓の間は
後者、DPR=1、論理DPI=96、窓1440×900、位置(-1679,66)を記録した。
前後のOS設定は一致したが、この初回版はfriendly nameとOS IDを結合できない。
そのため後続修正でPIDのウィンドウから画面IDを採るようにした。
初回runのFPSやメモリを、比較可能性や性能予算の達成証拠として扱わない。

### PIDの画面IDを追加した確認

`f020b6b2` で段2を再度PASS（3分05秒）。UIの動作変更はなく、前節の両OS・3ケースの画像確認を維持する。
今回のログは `target/perf/p1-window-id-check/check-logs` に別保存した。
同じ引数で `target/perf/p1-frame-window-id` へ2回採取し、親が起動したPIDのネイティブ画面IDを確認した。
exe SHA-256は `D69559097B5B33CAA590B4E7EDE48296F9F07C55C8B37A6C6C5E586EF8DB25CD`。

| run | frame行数 | スクロール中のOS画面ID採取数 | 画面／設定Hz | 最長間隔 | 間隔の開始時刻（scrollから） |
|---|---|---|---|---|---|
| 0（cold、除外） | 1,166 | 102 | `\\.\DISPLAY3` / 100Hz | 244.626ms | 7,725.515ms |
| 1 | 1,171 | 100 | `\\.\DISPLAY3` / 100Hz | 283.505ms | 7,726.300ms |

スクロール中の画面IDは全サンプル同じでunknownはなく、OS設定の前後も一致した。
Qt側の窓・DPI・画面情報も前回と同じ。通常間隔は約10msで、各runの100ms超は1件。
これで画面ごとの設定Hzとアプリの表示先を対応できた。180Hzのprimary画面を使ったと仮定していない。
memory.csvの時刻は実際の採取間隔を表す。100msのsleepに処理時間が加わるため、12秒で120点になるとは限らない。

4回の長い間隔の開始は7,720〜7,727msに集中した。FPS平均の差だけで評価せず、この共通区間を次の調査対象にする。
GC、行固有のlayout/Canvas/文字処理、backgroundのUI適用、計数自体の負荷のどれかは未特定。
診断用runのWorkingSetや平均FPSの差を改善量として報告しない。native保持元の帰属と予算合否は未完了。

## 最新mainへの追従

作業中にmainへ追加された5コミットを含む `3ff44ad6` へ、追加分の3コミットをrebaseした。
`range-diff` は3件とも同じ差分と判定。統合後に同じ3つのperf動詞を含む段2をPASS（3分06秒）とし、
両OSのPNGも確認した。ログは `target/perf/p1-rebase-check/check-logs`。
この再検証は表示契約の確認であり、rebase後の新しい性能baselineではない。

上記の実機値が指す旧コミットIDは、採取時のものとして変更していない。
各診断ディレクトリに `source-cc5d8eb6.zip` / `source-f020b6b2.zip` を保存した。
後者にはSHA-256一致を確認した `measured-platitude-gg.exe` も保存し、rebase後に上書きされたreleaseと区別する。
追加分は席bに保持し、今回のmain反映対象だった `97f072e6` までの11コミットとは分離する。
