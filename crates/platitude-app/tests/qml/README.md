# Canvasの撮影待ちの回帰テスト

`tst_ink.qml` はQt付属の `qmltestrunner` で実行する。Rustの登録型を使わず、
本体の `Ink.qml` と `InkCanvas.qml` を隔離したimportディレクトリにコピーして読む。
通常の `cargo test` には含まれない。

Windows / PowerShell（QtのbinをPATHへ追加済みの場合）:

```powershell
$moduleDir = 'target/ink-probe/platitude/ui'
New-Item -ItemType Directory -Path $moduleDir -Force | Out-Null
Copy-Item -LiteralPath crates/platitude-app/src/ui/Ink.qml,crates/platitude-app/src/ui/InkCanvas.qml -Destination $moduleDir
Set-Content -LiteralPath target/ink-probe/platitude/ui/qmldir -Value "module platitude.ui`nsingleton Ink 1.0 Ink.qml`nInkCanvas 1.0 InkCanvas.qml"
qmltestrunner -platform offscreen -input crates/platitude-app/tests/qml/tst_ink.qml -import target/ink-probe
```

Linuxも同じ2ファイルとqmldirを配置し、Qtの `qmltestrunner` に同じ引数を渡す。
親が非表示の場合、初回描画前の非表示化→再表示、実際のpaint完了後の再表示を検証する。
前2ケースは修正前に `Ink.owed = 1`（期待0）で失敗する。固定sleepは使わない。
