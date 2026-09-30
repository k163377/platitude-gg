# Windows のスリープ保持: 実測

2026-09-30。holder revision 2、activity protocol 1(`cargo xtask awake`、`crates/xtask/src/awake`)。

## 判定

保持・解除の Windows API 呼び出し、互換ビルドの holder が共有する activity、読取りと hook の競合を確かめた。
要望全体の完了ではない。Claude から通知されない許可・入力待ちは
[P3 の未達](../../internal-docs/P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake)に残る。
ここのテストは、実際の Claude の許可画面からのイベント配信を証明しない。

## 実 API 試験

`awake::tests::holder::coordination::compatible_holders_apply_and_clear_real_windows_requests_together`。
テスト専用の一時ディレクトリと Claude 役のプロセスを使い、通常の `apply` から WMI で起動する holder(r2)と、
同じスクリプトで holder 名だけ r3 にした互換ビルドを同時に動かす。OS API は置き換えない。
session / hook の入力は fixture で、Claude 本体ではない。

`SetThreadExecutionState` の非 0 の戻り値は、成功とそのスレッドの直前の状態を示す
([Microsoft の仕様](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate))。
両 holder とも保持時の戻り値は `0x80000000`、解除時は `0x80000001` —— system の継続要求を設定・解除でき、
display の継続要求は加えていない。

| 遷移 | r2(WMI 起動) | r3(互換ビルド) |
| --- | ---: | ---: |
| 最初の Prompt → 保持 | 4954 ms | 1736 ms |
| s が Stop、t は質問待ち → 解除 | 42 ms | 20 ms |
| t の質問への回答 → 保持 | 51 ms | 23 ms |
| t も Stop → 解除 | 20 ms | 43 ms |

質問待ちになる前から s が作業中なら、t の質問だけでは解除しないことも同じテストで確かめる。
数値は awake のテスト群を既定の並列で回した時の単発の観測値で、遅延の上限ではない。最初の保持は
WMI の起動と PowerShell の C# 型のコンパイルを含み、成立するまで保持の無い区間がある ——
「呼び出しの実行前に必ず保持済み」の保証には使えない。

awake の Windows テストは **60 passed / 0 failed / 2 ignored**(50.85 秒)。ignored は、親テストが専用環境で
呼ぶ子プロセスの入口 2 本。子コマンドの背景実行、質問・許可待ち、複数セッション、subagent、通知、
所有者の終了、読取りの妨害、同時起動、hook のパイプの解放、互換ビルドの同居、照会中に始まった仕事を含む。

## 未実施・限界

- 実際のスリープ・画面消灯は試験していない。電源設定は変えない
- `powercfg /requests` は管理者権限が無く取得できない。`CallNtPowerInformation(SystemExecutionState)` は
  他アプリの要求も含む全体の値で、この実装の保持・解除の帰属には使わない
- Claude の実際の許可操作、アプリ独自の許可画面、実際の MCP の内部実行は再現していない。
  P3 の upstream の未達を fixture の成功で解消したとは扱わない
