# Claude Desktop の許可ログによるスリープ制御補助

Windows の holder revision 3、activity protocol 1。実装は `crates/xtask/src/awake/desktop.rs`
(holder のスクリプトに埋め込まれる)。

hook が知らせない 2 つの区間を、デスクトップアプリの `main.log` の後追いで補う。
正式な通知の代わりではなく、実行前の保持は保証しない(§保証の境界)。

- 人が許可した後の、子プロセスを作らない呼び出し(MCP・WebFetch・WebSearch)を、許可から終了まで保持する
- アプリが自分で出す許可要求(Claude_Browser の `browser:open_file` 等)を待つ間は、その呼び出しを人待ちとして数える

Claude の許可判断と hook の記録(`.awake` の claim)は変更しない。補助の判断は各 holder の中だけで持ち、
ログが読めなくなれば捨てて hook だけの判定へ戻る。

## 事象の対応付け

既定は `%LOCALAPPDATA%/Claude/logs/main.log` と、同じ場所の `main1.log`(1 世代前)。
アプリは約 10 MB で `main.log` → `main1.log` → `main2.log` … と送る。次の 3 種類の行を読む(ID は匿名化)。

```text
2026-09-29 18:58:34 [info] Mapping internal session local_desktop to CLI session s
2026-09-29 18:58:35 [info] Emitted tool permission request request1 for browser:open_file in session local_desktop
2026-09-29 19:42:04 [info] Received permission response for request1: once (tool: browser:open_file)
```

CLI session は hook の `session_id`、ツール名は hook の `tool_name` と同じ綴り(`Bash` / `Edit` /
`mcp__ccd_settings__set_setting` 等)。例外は `browser:open_file` で、`mcp__Claude_Browser__navigate` に対応付ける。

- 要求は、同じセッションで同じツールの、要求の秒の末尾までに始まった未終了の呼び出しが 1 本だけの時に結び付ける
  (ログはローカル時刻・秒精度)。親と subagent などで候補が複数なら選ばず(ambiguous)、後で別の候補へ割り当て直さない
- 要求中は、結び付いた呼び出しだけを人待ちとして数える。他のセッション・背景プロセスの保持は残る
- `once` / `always` だけを許可とする。同じ呼び出しの要求がすべて解決するまで再開しない。
  未知の値・ツール名や時刻の食い違いは許可と推定せず、hook の判定へ戻す(unknown)
- 終了済みの呼び出しへの応答で、別の呼び出しを再開させない
- `AskUserQuestion` の実行許可は質問への回答ではないので対象外。MCP の Elicitation は hook の別の待機理由のまま

## 読取り

- ファイル名・サイズでなく Windows のファイル ID で読取り位置を追う。`main.log` → `main1.log` の移動後も
  同じファイルの続きを読み、新しい `main.log` は先頭から読む。新ファイルが旧ファイルより大きくても取り違えない
- 行は改行まで待つ(書きかけの行は次の読取りで続きと合わせる)
- 追っていたファイルが全部入れ替わった・縮んだ時は、補助の状態を捨てて残ったログから組み直す(log-gap-replayed)
- 読めない、3 種類の事象を含むのに書式が合わない行がある、上限(1 ファイル 32 MiB・1 行 1 MiB・各履歴 10,000 件)を
  超えた時は、補助を捨てて hook の判定へ戻る(fallback)。正常なログに戻れば次の読取りで復旧する
- ログのディレクトリの FileSystemWatcher の通知で読み直し、通知が欠けても holder の 15 秒周期で読む
- hook は許可待ちなのに対応する要求が見つからない呼び出しは、初回観測から 5 秒以降の読取りで unmatched に数える
  (状態表示のためで、判定は hook のまま)

別 revision の holder が同時に動いていると、それぞれが自分のスクリプトで判定する。補助を持たない holder は
人待ちでも保持し続けるので、この holder が解除しても眠らない。状態表示に別ビルドと出る。
他のセッションの holder は終了させず、そのビルドで動くセッションを終えてから更新する。

## 診断・復旧

1. <!--call:awake.status-->`awake` で holder ごとに、判定と、ログの最終読取り
   (`<pid> <時刻 ms> <状態> transport=<file-events|polling>`)を見る。stale な holder の過去の観測を現在の成功と扱わない
2. <!--call:awake.log.off-->`awake log off` で補助を止める。各 holder は次の読取り(最大 15 秒後)から
   hook だけで判定する。電源設定・hook の記録は変えない
3. ログの要求・応答・mapping の 3 行を確かめる。入力本文やログ全体は転載しない。
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

専用テストは `crates/xtask/src/awake/tests/holder/desktop.rs`(8 本)。ユーザーのログには触れない。

```text
cargo test -p xtask --bin xtask awake::tests::holder::desktop
```

分割行、別セッション、同時 subagent、複数の要求、遅い mapping、重複、holder の再起動、ローテーション、
大きい置換、切詰め、消失、無効化、書式変更とそこからの復旧、未知の応答、AskUserQuestion との区別を確かめる。
最後の 1 本は通常の `apply` → WMI → 出荷する holder → Windows API の経路を通す(ログと Claude プロセスは fixture)。
他の awake テストは補助を止めた状態(`.awake` 相当の一時ディレクトリに `desktop-log.off`)で回る。

## 実測

2026-09-30。実ログ(`main.log` / `main1.log`)では mapping 46、要求 104 / 応答 104、すべて結び付いた。
書式の合わない行は 0。初回走査(2 ファイル・約 20 MB)は 449 ms、追記なしの差分読取りは 12 ms(単発)。
常駐中の holder の CPU は 1 分あたり約 90 ms、ワーキングセットは約 135 MB。

専用テストの最後の 1 本(fixture のログと Claude プロセス、holder と OS 要求は実物):

| 遷移 | 時間 |
| --- | ---: |
| 初回の Prompt → 保持 | 2747 ms |
| 他セッションの Stop 後、`browser:open_file` の要求だけが残る → 解除 | 120 ms |
| その許可の行の追記 → 保持 | 19 ms |
| Stop → 解除 | 24 ms |
| 子プロセスの無い WebFetch の許可待ち → 許可の行の追記 → 保持 | 19 ms |
| Stop → 解除 | 19 ms |

テスト群を既定の並列で回した時の単発の観測値で、負荷や通知の欠落による遅延の上限ではない。
`SetThreadExecutionState` は非 0 を返し、解除時の直前状態は `0x80000001`(system の継続要求のみ = display は要求していない)。

## 保証の境界

ログの後追いなので、許可からログを読むまでの隙間は残る。手元に入力があっても「眠る隙が無い」とは扱わない。
遠隔からの許可、時計の変更、秒精度の中の別の呼び出し、ログに出ない待機、未知のツール・アプリの更新には
対応できない場合がある。補助が止まっている・使えない時は、
[P3 の未達](P3-確認事項.md#開発環境スリープ止め-cargo-xtask-awake)の挙動に戻る。
API の成功・ログの認識と、実際の消灯・スリープ到達は別の証拠として扱う。実際のスリープ・画面消灯は試験していない。
