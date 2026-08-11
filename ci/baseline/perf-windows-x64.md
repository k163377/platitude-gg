# 性能実測記録(Windows x64)

計測: 2026-08-11、commit 5b1cff8。

## 計測条件

- 対象: `JetBrains/kotlin` full clone — 227,323 commits、refs 53,728 本
  (タグ 45,901 / リモートブランチ 7,825)、commit-graph chain あり
- 表示は既定設定(`HEAD --branches --remotes --tags` を `--max-count=2000` の
  ウィンドウ)。自動化 env が設定ストアを空にするので既定値そのまま
- `cargo build --release -p platitude-app`、ウォームファイルキャッシュ、
  **実ウィンドウ**(offscreen では fps が表示性能にならない)
- ハーネスは `PG_AUTO_OPEN` / `PG_AUTO_SELECT` / `PG_AUTO_SCROLL` / `PG_LOG=info`。
  メモリは WorkingSet の 100ms サンプリング最大値
- 機材: Ryzen 9 9900X(12C/24T)/ 32GB / 1920x1080@100Hz / Windows 11 build 26200 /
  git 2.55.0
- 数値は 3 回連続実行の範囲(全数)

## 判定

| 予算 | 実測 | 判定 |
|---|---|---|
| 起動→グラフ初回表示 3s | 873–933ms | ✅ |
| 操作応答 100ms | 154–269ms | ❌ |
| スクロール 60fps | 178.0–178.3fps | ✅ |
| メモリ 300MB | 363.0–367.4MB | ❌ |

## 内訳

- 初回表示はタグ無しパス(2,000 行)。タグ込みの置換パスは起動から 3.8–4.0s で完了する
  (単一 drain・フリッカーなし)
- 操作応答はハーネスが撃つ行選択→詳細の 1 点で、タグ込み walk が裏で走っている最中に当たる
- メモリはスクロールベンチ込み。ベンチ無し(起動 + 行選択 + 差分表示のみ)は 330.0MB で、
  これも予算を超える
- fps は `onFrameSwapped` の実数。表示の 100Hz を上回るので、提示フレームで頭打ちに
  なっていない
