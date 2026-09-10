# identity 保存結果の通知競合

## 再現条件と確認した因果

2026-09-10、Windows、席 d、基点 `0e664333`。`identity-tip` は既存の半端保存 fixture を使う。
`write_identity` は git の読み返しを `AppMsg::Identity`、保存成否を `AppMsg::IdentitySaved` として
別々に `Feed::push` していた。1 回の push ごとに Qt の drain が起こされるため、
drain 末尾の notify が 1 本でも、この 2 メッセージを同じ drain が受ける保証はない。

前半だけを読むと、名前とメールが埋まって `identityState=ready` になり、
後半が持つ `identityUnsaved=true` はまだ無い。`IdentityGate.identityWanted` が落ち、
dialog が閉じる。その `onClosed` が `identityDismissed=true` を保持するため、
後半が届いても dialog は戻らない。`WindowDialogActs` は opened と unsaved の両方を待って止まる。

修正前の producer の最初の push の直後に、一時的に `feed.depth()==0` まで待つ処理を挿した。
これは先行バッチを UI が取り出すまで producer を止め、後半をそのバッチに入れないためのもの。
sleep で機械の速さに賭けず、待ちには 10 秒の backstop を付けた。実行コマンド:

```text
cargo xtask verify-ui identity-tip --watchdog-ms 4000 --no-census --no-board
```

修正前は 4.9 秒で FAIL(exit 0、`saved=false`、`grabbing=false`、`step=dismissed` 無し)。
`auto-act watchdog expired verb=identity-tip` が出て、最終 must_say に届かなかった。
通常実行が 1.4 秒で PASS した同じビルドで、drain 境界を指定するだけで赤になった。

修正は `IdentityWrite` 全体またはエラーを **1 つの `IdentitySaved`** に載せるもの。
`finish_identity` が値・保存成否・busy を適用してから、drain 末尾で notify する。
修正後も最初の push 直後に同じ待ちを置き、1.2 秒で PASS、3 段の step と撮影成功を確認した。
一時的な待ちは最終コードから除去済み。純 Rust の回帰テストは最初の drain に
保存成否が揃うこと、全保存成功、保存エラー時の既知 identity の保持を確認する。

生ログは席 d の `target/identity-split-repro.log`、`identity-atomic-repro.log`。
恒久的に必要な再現条件と結果は本記録に保存する。

## 並行検証と残る範囲

最終実装で Windows の 3 ランナーを並行起動し、それぞれ以下を既定 watchdog で 10 回実行。
`--no-build --no-census --no-board` を指定し、各 run は独立した demo/config/shot directory を使った。

- `identity-tip`: 10/10 PASS、1.2〜1.4 秒
- `ref-list 3 --preset stack`: 10/10 PASS、1.2〜1.4 秒
- `plan-reword-ask --preset plan`: 10/10 PASS、1.2〜1.4 秒

Linux コンテナでも `identity-tip` を最終実装で 10 回連続実行し、10/10 PASS(1.0〜1.1 秒)。
Rust の identity 回帰 3 テストも PASS。

これは最小スイートの並行検証であり、過去の gate 全量の負荷を再現したものではない。
終了時に止まる後 2 件は再現せず、解決とは扱わない。
identity の競合は実装上の到達可能性と修正前後を確認できたが、過去の run 自体に
drain 境界の記録は無い。その 1 回の原因や budget 変更との因果、発生頻度は断定しない。
