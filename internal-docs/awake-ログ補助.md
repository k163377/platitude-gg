# Claude Desktop の許可ログによるスリープ制御補助

Windows の holder revision 6、activity protocol 1。実装は `crates/xtask/src/awake/desktop.rs`
(holder のスクリプトに埋め込まれる)。

hook が知らせない 2 つの区間を、デスクトップアプリの `main.log` の後追いで補う。
正式な通知の代わりではなく、実行前の保持は保証しない(§保証の境界)。

- 人が許可した後の、子プロセスを作らない呼び出し(MCP・WebFetch・WebSearch)を、許可から終了まで保持する
- アプリが自分で出す許可要求(Claude_Browser の `browser:open_file` 等)を待つ間は、その呼び出しを人待ちとして数える

Claude の許可判断と hook の記録(`.awake` の claim)は変更しない。補助の判断は各 holder の中だけで持ち、
ログが読めなくなれば捨てて hook だけの判定へ戻る。

## 事象の対応付け

既定は `%LOCALAPPDATA%/Claude/logs/main.log` と、同じ場所の `main1.log`(1 世代前)。
アプリは約 10 MB で `main.log` → `main1.log` → `main2.log` … と送る。次の 4 種類の行を読む(ID は匿名化)。

```text
2026-09-29 18:58:34 [info] Mapping internal session local_desktop to CLI session s
2026-09-29 18:58:35 [info] Emitted tool permission request request1 for browser:open_file in session local_desktop
2026-09-29 19:42:04 [info] Received permission response for request1: once (tool: browser:open_file)
2026-10-04 12:50:18 [info] Permission request request2 for AskUserQuestion aborted
```

CLI session は hook の `session_id`。CLI の許可のツール名は hook の `tool_name` と同じ綴り(`Bash` / `Edit` /
`mcp__ccd_settings__set_setting` 等)。アプリ自身の許可画面はツールでなく種類を名乗り、どのツールから出たかは
ログに無いので、その種類を出すサーバーの呼び出しならどれにでも対応付ける:

| ログの名前 | 対応付ける呼び出し | 出る場面 |
| --- | --- | --- |
| `browser:*`(`open_file` / `open_site` / `domain_transition` / `submit_credentials` ほか) | `mcp__Claude_Browser__*`・`mcp__claude-in-chrome__*` | ローカルファイル・サイトを開く(`navigate` / `preview_start` / `browser_batch`)、ドメインの移動、資格情報の送信 |
| `computer:*`(`request_access`) | `mcp__computer-use__*` | computer use がアプリの操作の許可を求める |

種類はアプリ 2.19675.0 の `app.asar`(`permissionSurface` と `handleToolPermission` の呼び出し元)で確かめた。

- 要求は、同じセッションで対応するツールの、要求の秒の末尾までに始まった未終了の呼び出しが 1 本だけの時に結び付ける
  (ログはローカル時刻・秒精度)。親と subagent などで候補が複数なら選ばず(ambiguous)、後で別の候補へ割り当て直さない
- 候補は読取りの始めに写した claim から取る。写した後に始まって許可を求めた呼び出しは写しに無く、直前の呼び出し
  (同じサーバーの続けての呼び出し)が開いたまま写っている。読取りの間に claim が変わった、または claim を
  比べられなかった(読めない名前・claim)なら、その読取りで決めた結び付けを取り消し、次の読取りの claim で
  決め直す(claim の変更が次の読取りを起こす)。ambiguous は取り消さない —— 候補が終わっていく中で選び直すと、
  走っている方を選びうる
- 要求中は、結び付いた呼び出しだけを人待ちとして数える。他のセッション・背景プロセスの保持は残る
- `once` / `always` だけを許可とする。同じ呼び出しの要求がすべて解決するまで再開しない。
  未知の値・ツール名や時刻の食い違いは許可と推定せず、hook の判定へ戻す(unknown)。
  `aborted`(答えを待たずに取り下げられた許可画面。実ログの 1 件は、OS のシャットダウンで CLI のプロセスが
  落ちた時に書かれた)は許可ではないが、もう人を待っていないので解決として再開する
- 終了済みの呼び出しへの応答で、別の呼び出しを再開させない
- `AskUserQuestion` の実行許可は質問への回答ではないので対象外。MCP の Elicitation は hook の別の待機理由のまま

## 読取り

- ファイル名・サイズでなく Windows のファイル ID で読取り位置を追う。`main.log` → `main1.log` の移動後も
  同じファイルの続きを読み、新しい `main.log` は先頭から読む。新ファイルが旧ファイルより大きくても取り違えない
- 行は改行まで待つ(書きかけの行は次の読取りで続きと合わせる)
- 追っていたファイルが全部入れ替わった・縮んだ時は、補助の状態を捨てて残ったログから組み直す(log-gap-replayed)
- 読めない、4 種類の事象を含むのに書式が合わない行がある、上限(1 ファイル 32 MiB・1 行 1 MiB・各履歴 10,000 件)を
  超えた時は、補助を捨てて hook の判定へ戻る(fallback)。正常なログに戻れば次の読取りで復旧する
- ログのディレクトリの FileSystemWatcher の通知で読み直し、通知が欠けても holder の 15 秒周期で読む
- hook は許可待ちなのに対応する要求が見つからない呼び出しは、初回観測から 5 秒以降の読取りで unmatched に数える
  (状態表示のためで、判定は hook のまま)

別 revision の holder が同時に動いていると、それぞれが自分のスクリプトで判定し、1 本でも保持すれば眠らない。
補助を持たない holder は人待ちでも保持し続けるので、この holder が解除しても眠らない。状態表示に別ビルドと出る。
revision 4 以降の holder は、自分のビルドの hook が 10 分動かず、使われている別ビルドの holder
(全部のビルドが静かなら、より上の revision の holder)が立っていれば、自分から降りる。
hook の印(`.awake/hooks-r<n>`)を書かない revision 3 以前の holder は自分では降りないので、
そのビルドで動くセッションが無くなったら名前ファイル `.awake/holder-r<n>` を消す(次の読取りで終わり、保持を解く)。

## 診断・復旧

1. <!--call:awake.status-->`awake` で holder ごとに、判定と、その holder が履歴に最後に書いた 1 行
   (何を理由に保持しているか)と、ログの最終読取り(`<pid> <時刻 ms> <状態> transport=<file-events|polling>`)を見る。
   stale な holder の過去の観測を現在の成功と扱わない。過去の判定は `.awake/history`
   ([実測記録](../ci/baseline/awake-control-windows-x64.md#判定の履歴))
2. <!--call:awake.log.off-->`awake log off` で補助を止める。各 holder は次の読取り(最大 15 秒後)から
   hook だけで判定する。電源設定・hook の記録は変えない
3. ログの要求・応答(または中断)・mapping の行を確かめる。入力本文やログ全体は転載しない。
   書式が変わったなら、パーサ(`desktop.rs`)と専用テストの fixture を一緒に直す。
   アプリがログの場所を変えたなら、`.awake/desktop-log.path` に新しい `main.log` の絶対パスを 1 行で書く
   (holder は起動時に読む。テストもこれで自分のログを指す)
4. 専用テストと gate を通し、<!--call:awake.log.on-->`awake log on` で再開する。
   止めていた間に起動した holder は、再開後は通知でなく 15 秒周期で読む。通知での応答を確かめる時は、
   その holder が終わってから新しい holder で確かめる
5. 実際の許可画面で、待機・許可後・終了を確かめる。他の保持要求が残っていれば、
   この holder の解除と PC 全体のスリープを別々に判定する

| 状態 | 意味 |
| --- | --- |
| disabled | `log off` 中。hook だけで判定 |
| watching overlay=N | 読取り成功。N は要求に結び付いた呼び出しの数(完全な対応の宣言ではない) |
| unmatched | hook は許可待ちだが、要求の行に結び付けられない |
| ambiguous | 候補の呼び出しが複数。選ばず hook の判定のまま |
| unknown | 応答の値・ツール・時刻が合わない。許可と決めつけない |
| unmapped | 要求の時刻までに始まった呼び出しはあるが、どれもツールが合わない |
| log-gap-replayed | 連続性を失い、残ったログから組み直した |
| fallback | 読取り・書式・上限の問題。補助を捨てて hook だけで判定 |

## テスト

専用テストは `crates/xtask/src/awake/tests/holder/desktop.rs`(13 本)。ユーザーのログには触れない。

```text
cargo test -p xtask --bin xtask awake::tests::holder::desktop
```

分割行、別セッション、同時 subagent、複数の要求、遅い mapping、重複、holder の再起動、ローテーション、
大きい置換、切詰め、消失、無効化、書式変更とそこからの復旧、未知の応答、AskUserQuestion との区別、
アプリの許可画面の種類から呼び出しへの対応(`preview_start`・`browser_batch`・claude-in-chrome・computer use は
結び付き、別サーバー・ブラウザ外の呼び出しは unmapped)、中断で消えた許可画面、読取りの間に claim が変わった
時・claim を比べられなかった時の決め直しと ambiguous の維持(出荷する holder の本体を 2 回読ませ、1 回目の
claim の写しの後に呼び出しを入れ替えて要求を書く。比べられない方は 1 回目の比較の手前で名前の読取りを失敗させる)
を確かめる。
最後の 1 本は通常の `apply` → WMI → 出荷する holder → Windows API の経路を通す(ログと Claude プロセスは fixture)。
他の awake テストは補助を止めた状態(`.awake` 相当の一時ディレクトリに `desktop-log.off`)で回る。

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

ログの後追いなので、許可からログを読むまでの隙間は残る。手元に入力があっても「眠る隙が無い」とは扱わない。
遠隔からの許可、時計の変更、秒精度の中の別の呼び出し、ログに出ない待機、未知のツール・アプリの更新には
対応できない場合がある。補助が止まっている・使えない時は、
[P3 の未達](P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake)の挙動に戻る。
API の成功・ログの認識と、実際の消灯・スリープ到達は別の証拠として扱う(§実スリープ試験)。

## 実スリープ試験

手動の専用テスト `crates/xtask/src/awake/tests/holder/physical.rs`。電源設定を一時的に変えて実際のスリープを許すので、
通常の gate では ignored。ユーザーが無人試験を許可した時だけ回す。`PGG_AWAKE_PHYSICAL_LOG` に記録先の絶対パスを置く。

```text
cargo test -p xtask --bin xtask a_desktop_wait_allows_physical_sleep -- --ignored --nocapture
```

親の Claude プロセスと許可ログは fixture、holder の OS 要求・画面の通知・S3 は実機。
実際の Claude の許可画面の一連の操作を確かめたことにはならない。
試験は AC スリープを 180 秒にして始め、`finally` で元の値へ戻す。DC・画面消灯の設定は変えない。
強制スリープの命令、マウス・キー入力、自動許可は使わない。復帰は試験用の 8 分の wake timer で求め、
終了時にタイマーと通知の登録を破棄する。

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
