# Windows のスリープ保持: 実測

2026-10-04。holder revision 5、activity protocol 1(`cargo xtask awake`、`crates/xtask/src/awake`)。
デスクトップアプリのログによる補助と実スリープ試験の記録は [ログ補助](../../internal-docs/awake-ログ補助.md)。

## 判定

保持・解除の Windows API 呼び出し、互換ビルドの holder が共有する activity、読取りと hook の競合、
保持中の読取りの間引き、判定の履歴を確かめた。
要望全体の完了ではない。Claude から通知されない許可・入力待ちは
[P3 の未達](../../internal-docs/P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake)に残る。
ここのテストは、実際の Claude の許可画面からのイベント配信を証明しない。

## 実 API 試験

`awake::tests::holder::coordination::compatible_holders_apply_and_clear_real_windows_requests_together`。
テスト専用の一時ディレクトリと Claude 役のプロセスを使い、通常の `apply` から WMI で起動する holder(r5)と、
同じスクリプトで holder 名だけ r6 にした互換ビルドを同時に動かす。OS API は置き換えない。
session / hook の入力は fixture で、Claude 本体ではない。

`SetThreadExecutionState` の非 0 の戻り値は、成功とそのスレッドの直前の状態を示す
([Microsoft の仕様](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate))。
両 holder とも保持時の戻り値は `0x80000000`、解除時は `0x80000001` —— system の継続要求を設定・解除でき、
display の継続要求は加えていない。

| 遷移 | r5(WMI 起動) | r6(互換ビルド) |
| --- | ---: | ---: |
| 最初の Prompt → 保持 | 1906 ms | 1138 ms |
| s が Stop、t は質問待ち → 解除 | 15 ms | 47 ms |
| t の質問への回答 → 保持 | 47 ms | 23 ms |
| t も Stop → 解除 | 54 ms | 34 ms |

質問待ちになる前から s が作業中なら、t の質問だけでは解除しないことも同じテストで確かめる。
数値は単発の観測値で、遅延の上限ではない。最初の保持は
WMI の起動と PowerShell の C# 型のコンパイルを含み、成立するまで保持の無い区間がある ——
「呼び出しの実行前に必ず保持済み」の保証には使えない。

使われなくなったビルドの holder は `a_started_holder_of_a_quiet_build_quits` で確かめる。WMI で起動した r5 の
hook の印(`hooks-r5`)を 10 分より前に戻し、hook が動いている r6 と並べると、r5 は次の読取りで名前を外して
プロセスごと終わり、claim は残り、r6 が保持を続けた。降りる判定そのものは、holder 本体を 1 回だけ読ませる試験で
場合分けする(使用中の別ビルドがある / 1 本だけ / 全部静か / 相手の心拍が古い)。

awake の Windows テストは **81 passed / 0 failed / 3 ignored**(37.49 秒。律速は tick 待ちを 2 回含むテストで、
他席の負荷で 38〜54 秒に揺れる)。
ignored は、親テストが専用環境で呼ぶ子プロセスの入口 2 本と、手動の実スリープ試験 1 本。子コマンドの背景実行、
質問・許可待ち、複数セッション、subagent、通知、所有者の終了、読取りの妨害、同時起動、hook のパイプの解放、
互換ビルドの同居、照会中に始まった仕事、owner の一覧の後に始まったコマンド、使われなくなったビルドの holder が
降りること、ログ補助、保持中の読取りの間引き、履歴(理由の表記・切り詰め・書き手ごとの最後の行)を含む。

## holder の負荷

holder は event(claim の書き換え、デスクトップアプリのログの追記)のたびに読取りを始める。hook は 1 回の
ツール呼び出しで claim を 4 回書き、holder 自身の書き戻しも 1 回の event になるので、セッションが動いている間は
event が途切れない。保持している間の読取りは解除しかできない(保持は既に立っている)ので、event で起きる読取りを
間引く: **60 回分の猶予を使い切ると 5 秒に 1 回**で、猶予は 5 秒に 1 回分ずつ戻る(`holder::BURST` /
`holder::PACE`。数えるのは進むだけの時計)。保持していない間の読取りは待たせない。15 秒周期の読取り
(プロセスの開始・終了、transcript の伸び)は別枠で、心拍の間隔は最長でも 20 秒強。

本番の `.awake`(claim 8 件、うち作業中 3 セッション)で、猶予を使い切った後の 187 秒:

| | 値 |
| --- | ---: |
| 読取り | 11 回/分 |
| CPU | 1,625 ms(1 コアの 0.9%、読取り 1 回 49 ms) |
| ワーキングセット | 140〜180 MB |

猶予が残っている間は event ごとに読む(起動直後の 127 秒: 23 回/分、1 コアの 3.7%)。読取り 1 回の支配項は、
作業中のセッションの transcript の末尾 256 KB の読み直し。伸びていない transcript は読み直さないので、
開いたままの idle なセッションの claim に掛かるのは、ファイルを開いて長さを見る分と owner の照会だけ。
プロセスの照会は一覧(Toolhelp)1 回で全 claim に答える(一覧と owner 11 件への問い合わせで 7.7 ms)。
一覧は 1 回の読取りで 2 回まで取る: 始めに owner 用、transcript を全部読んだ後に子プロセス用
(コマンドは結果より先に起きるので、子を読む一覧は transcript より新しくなければならない)。

間引きは `a_held_machine_s_readings_are_paced_past_a_burst` が、holder 本体を偽の時計で回して確かめる
(保持中は猶予を超えた後 5 秒に 1 回、保持していない間は待たない)。一覧の順は
`a_command_started_once_the_owners_are_listed_keeps_its_call` が、owner の一覧の直後にコマンドを起こして確かめる。

## 運用中の記録

2026-09-30 10:30 〜 2026-10-04 10:00(JST)の記録。条件: holder は revision 4(判定の履歴を残さない)、
デスクトップアプリ側の保持(`ccKeepAwakeWhileWorking`)は On。
System ログのスリープ(Kernel-Power 42 / Power-Troubleshooter 1)33 回を、各セッションの transcript と
アプリの `main.log` の `[keep-awake]` 行に突き合わせた。holder 自身の判定は、いつ眠れたかから間接に読んでいる。

- **ターンの途中・背景コマンドの実行中に眠った例は 0 回**。アプリも同じ区間を保持するので、
  holder 単独の保持の証拠にはならない
- **22 回は、アプリが最後の保持を解いたのと同じ秒に眠った** —— ターン終了の約 30 秒後の解除のほか、
  下の質問待ちの stalled 解除 3 回と、リモート用の 5 分保持の解除を含む。
  その時点で holder は既に解除していた(要求が 1 本でも立っていれば眠れない)。残り 11 回のうち 8 回は
  アプリの解除から 52〜643 秒後(無入力タイマーの残りと holder の保持の続きを区別できない)、
  1 回は API からのスリープ(スタートメニュー等)、2 回は再起動・復帰の直後
- **質問(AskUserQuestion)待ちで眠ったのは 3 回で、どれもアプリが stalled で保持を解いた秒**(質問から
  60.7 分・60.7 分・82.9 分後)。5 分以上の質問待ち 9 回のうち、60 分より前に眠った例は無い。
  holder は質問待ちを数えないが、アプリは人を待っているターンも数え続け、進捗が 60 分無い時にだけ外す
  ([P3 の未達](../../internal-docs/P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake))
- アプリのログに出た許可要求 16 件は全部 AskUserQuestion(auto モードではツールの許可画面が出ない)。
  ログ補助が結び付ける対象は 0 件だった
- 9/30 17:00 以降の hook の失敗 630 件は全部、席で xtask を編集している最中か、cwd が別の cargo
  ワークスペースにある間の PreToolUse / PostToolUse。ターン終了 307 回の Stop hook に失敗は無く、
  所要は中央 217 ms・90% 点 537 ms・最大 2,444 ms

## 判定の履歴

holder(revision 5)の判定は `.awake/history` に残る。全 revision の holder が同じ 1 本へ、判定か、
保持の理由の組が変わるたびに 1 行書く。行の形(session の id は略記):

```text
1791092805997 r5 56572 holding process:8c49584e-…,turn:2390b74a-…,turn:8166f3c3-…
1791093405120 r5 56572 released -
1791093990004 r5 56572 quit -
```

左から、時刻(ミリ秒)、書いた holder の revision と pid、判定、理由。理由は整列済みのカンマ区切りで、
`turn:<session>`(応答か呼び出しが数えられている。subagent の分も session に寄せる)、
`process:<session>`(シェル呼び出しのプロセスが走っていて、その session の `turn` が数えられていない)、
`wake:<session>`(予約した再開が先にある)。`holding -` は、読取りの途中で claim が変わったので保持を続けた、の意。
4 MB を超えたら新しい側の半分を残す。数セッションが動く時間帯で 1 時間に 50〜90 行。
`cargo xtask awake` は、holder ごとにその holder が最後に書いた 1 行を出す。
スリープの記録(上の System ログ)との突き合わせは、この履歴で直接できる。

## 未実施・限界

- `powercfg /requests` は管理者権限が無く取得できない。`CallNtPowerInformation(SystemExecutionState)` は
  他アプリの要求も含む全体の値で、この実装の保持・解除の帰属には使わない
- Claude の実際の許可操作、アプリ独自の許可画面、実際の MCP の内部実行は再現していない。
  P3 の upstream の未達を fixture の成功で解消したとは扱わない
- 無入力タイマーの設定値(AC のスリープまでの時間)と人の入力は記録に残らない。運用中の記録の期間は
  判定の履歴も無いので、アプリの解除から眠るまでの間が無入力タイマーなのか別の保持なのかは判定できない
