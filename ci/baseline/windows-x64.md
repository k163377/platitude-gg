# ネットワーク非通信 baseline 実測（Windows x64）— Phase 0

実装計画 §11.2 の前提(既定ビルドの Qt では配布物に通信の経路が残る)の根拠。P0 スパイク（`spike/`）の windeployqt 配布物を実測したもの。

- 計測対象: `pgg_spike.exe`（qtbridge 0.2.0 / Qt 6.10.3 msvc2022_64 / rustc 1.97.1, release）
- deploy コマンド: `windeployqt --release --compiler-runtime --no-translations --qmldir spike/src <exe>`
- 検査コマンド: `dumpbin /imports <exe>` / `dumpbin /dependents <dll>`（VS 2022 Build Tools）
- 配布物: 217 ファイル / 約111MB（qml/ の .qml 資材・vc_redist 含む）

## 1. 実行ファイル自身の import table（DLL 名のみ）

```
kernel32.dll  KERNEL32.dll  ntdll.dll
bcryptprimitives.dll            ← Rust std の乱数(BCryptGenRandom)。通信機能ではない
api-ms-win-core-synch-l1-2-0.dll
Qt6Core.dll  Qt6Gui.dll  Qt6Qml.dll
VCRUNTIME140.dll  api-ms-win-crt-{runtime,heap,math,stdio,locale}-l1-1-0.dll
```

**assert（アプリ自身の主張）**: exe の import table に
`Qt6Network.dll` / `WS2_32.dll` / `winhttp.dll` / `wininet.dll` は**含まれない**。

## 2. 同梱 Qt DLL の推移的依存（要点）

| DLL | ネットワーク関連の依存 |
|---|---|
| Qt6Qml.dll | **Qt6Network.dll（ハードインポート）** |
| Qt6Quick.dll | **Qt6Network.dll(ハードインポート)** |
| Qt6Core.dll | **WS2_32.dll**（OS の socket API） |
| Qt6Gui.dll | なし（d3d11/dxgi/d3d12 等の描画系のみ) |

### 帰結

「**Qt6Network を同梱しない**」は既定ビルドの Qt の Qt Quick 構成では**成立しない**
（Qt6Qml/Qt6Quick がロード時に要求するため、DLL を除くと起動不能になる）。
外すのは Qt のカスタムビルド — 実装計画 §11.2。

## 3. 同梱 DLL / プラグイン一覧（P5 allowlist の種）

トップレベル:
```
pgg_spike.exe  vc_redist.x64.exe
Qt6Core / Qt6Gui / Qt6Network / Qt6OpenGL / Qt6Qml / Qt6QmlMeta / Qt6QmlModels /
Qt6QmlWorkerScript / Qt6Quick / Qt6QuickControls2 / Qt6QuickControls2Basic /
Qt6QuickControls2BasicStyleImpl / Qt6QuickControls2Fusion / Qt6QuickControls2FusionStyleImpl /
Qt6QuickControls2Impl / Qt6QuickLayouts / Qt6QuickShapes / Qt6QuickTemplates2 / Qt6Svg (.dll)
icuuc.dll  d3dcompiler_47.dll  dxcompiler.dll  dxil.dll  opengl32sw.dll
```

プラグイン:
```
generic\qtuiotouchplugin.dll
iconengines\qsvgicon.dll
imageformats\{qgif,qico,qjpeg,qsvg}.dll
platforms\qwindows.dll
networkinformation\qnetworklistmanager.dll   ← 除外候補（QtNetwork 利用時のみ意味を持つ）
tls\{qcertonlybackend,qschannelbackend}.dll  ← 除外候補（同上）
qmltooling\qmldbg_*.dll（11個, qmldbg_tcp 含む） ← 除外候補（QML デバッガ。リリース配布に不要）
qml\**\*plugin.dll（QtQuick/QtQml の QML モジュール群 12個）
```

除外は `windeployqt --skip-plugin-types networkinformation,tls,qmltooling` で可能（P5 で採用判断・起動検証）。
