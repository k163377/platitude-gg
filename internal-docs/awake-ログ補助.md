# Claude Desktop の許可ログによるスリープ制御補助

hook が知らせない 2 つの区間を、デスクトップアプリの `main.log` の後追いで補う。
正式な通知の代わりではなく、実行前の保持は保証しない(§保証の境界)。

- 人が許可した後の、子プロセスを作らない呼び出し(MCP・WebFetch・WebSearch)を、許可から終了まで保持する
- アプリが自分で出す許可要求(Claude_Browser の `browser:open_file` 等)を待つ間は、その呼び出しを人待ちとして数える

実装は `crates/xtask/src/awake/desktop.rs`(holder のスクリプトに埋め込まれる)。事象の対応付け・読取り・状態の語・
診断と復旧の手順・専用テストは、そのファイルと `crates/xtask/src/awake/tests/holder/desktop.rs` の冒頭のコメントが正。
Codex の native activity と Desktop ログも同じ holder が読むが、対応付けは別の観測器が担当する([Codex ハーネス](codex/ハーネス.md))。

## 実測

2026-09-30。実ログ(`main.log` / `main1.log`)では mapping 46、要求 104 / 応答 104、すべて結び付いた。
書式の合わない行は 0。初回走査(2 ファイル・約 20 MB)は 449 ms、追記なしの差分読取りは 12 ms(単発)。
常駐中の holder の CPU とワーキングセットは[実測記録](../ci/baseline/awake-control-windows-x64.md#holder-の負荷)。

2026-10-05 の実ログの 3 行(mapping・`preview_start` が出した `browser:open_file` の要求・その応答)を
revision 6 の補助に流すと、要求中はその呼び出しを人待ち(`overlay=1`)、応答後は作業中と判定する。

専用テストの最後の 1 本(2026-10-05、revision 6。fixture のログと Claude プロセス、holder と OS 要求は実物):

| 遷移 | 時間 |
| --- | ---: |
| 初回の Prompt → 保持 | 1528 ms |
| 他セッションの Stop 後、`browser:open_file` の要求だけが残る → 解除 | 119 ms |
| その許可の行の追記 → 保持 | 18 ms |
| Stop → 解除 | 27 ms |
| 子プロセスの無い WebFetch の許可待ち → 許可の行の追記 → 保持 | 17 ms |
| Stop → 解除 | 18 ms |

テスト群を既定の並列で回した時の単発の観測値で、負荷や通知の欠落による遅延の上限ではない。
`SetThreadExecutionState` は非 0 を返し、解除時の直前状態は `0x80000001`(system の継続要求のみ = display は要求していない)。

## 保証の境界

ログの後追いなので、許可からログを読むまでの隙間は残る(取りこぼしうる形は `desktop.rs` の冒頭)。補助が止まっている・使えない時は、
[P3 の未達](P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake)の挙動に戻る。
API の成功・ログの認識と、実際の消灯・スリープ到達は別の証拠として扱う(§実スリープ試験)。

## 実スリープ試験

手動の専用テスト `crates/xtask/src/awake/tests/holder/physical.rs`(通常の gate では ignored)。
**ユーザーが無人試験を許可した時だけ回す**。撃ち方・電源設定の扱い・実機と fixture の境はファイル冒頭のコメント。
親の Claude プロセスと許可ログは fixture なので、実際の Claude の許可画面の一連の操作を確かめたことにはならない。

2026-09-30 JST の結果(元の AC スリープ 1800 秒、画面消灯 AC 60 秒)。1 passed、499.16 秒、設定の復元も成功。
試験に使った holder は revision 3。試験の呼び出し(`navigate`)の保持・解除の判定は revision 6 と同じで、revision 6 との違いは、起動時のスクリプトの受け渡し、使われなくなったビルドの holder が降りる判定、保持中の読取りの間引き、判定の履歴、アプリの許可画面に結び付ける呼び出しの範囲(revision 3 は `navigate` だけ)、claim を確かめられなかった読取りの結び付けの決め直し、中断の行:

| 時刻 | 観測 |
| --- | --- |
| 08:51:49.746 | 保持の開始。holder は system のみを要求 |
| 08:51:49.749 | Windows の DISPLAY 通知 state=0(消灯) |
| 08:55:16.765 | 3 分を超えた保持の区間でも global execution state=1、スリープ通知なし |
| 08:55:19.780 | 試験用の `browser:open_file` 許可待ちの行を追記 |
| 08:55:21.773 | global execution state=0 |
| 08:55:24.876 | Windows の SUSPEND 通知 |
| 08:55:25.783 | System ログの Kernel-Power イベント 42 |
| 08:59:51.902 | 復帰後の判定 PASS(消灯・保持中の非スリープ・解除後の実スリープ) |

Power-Troubleshooter イベント 1 の sleep time は 08:55:19.818、wake time は 08:59:53.012(復帰元は
powershell.exe の timer の可能性と記録)。イベントの記録時刻・通知時刻は実際の遷移の時刻と異なるので同一視しない。
