# Codex の席とスリープ制御

席の claim は `CODEX_THREAD_ID` の会話に属する。返信・依頼の完了・人待ち・アプリ再起動で返さず、
同じ席で続ける。返すのは land、ユーザーが指示した release、その場で指示した takeover のみ。
main 追従の rebase もユーザーの指示がある時だけ。スリープ保持の解除は席の返却と別の操作。

## 通常起動で使うための初回設定

作業 checkout で <!--cmd:awake.codex.setup-->`cargo xtask awake codex setup` を一度実行する。
ユーザー環境変数 `CODEX_MAX_LOG_LEVEL=debug` を保存して Windows に変更を通知する。
Codex Desktop を完全終了し、通常のアイコンから起動する。起動用ラッパーや CLI の差し替えは不要。
現在のアプリを終了する操作は setup に含めない。

変更を席の branch で保持している間、primary checkout の hooks はまだ更新されない。
通常の primary checkout でこのchatを実験するには、ユーザーが main 反映を指示し、
反映後に以下の信頼・起動手順を行う。別のchatから他人の席を取得して実験しない。

Desktop の Settings → Hooks で対象プロジェクトを開き、Reload hooks で読み直す。
プロジェクトの `.codex/hooks.json` の内容を確認して信頼する(その source の Trust all)。
CLI の場合は `/hooks` を使う。
hook は `cargo run --profile hooks` を使うため、最初の実験前に以下でビルドしておく。
hook の編集後は再度信頼確認が必要([公式 hooks 仕様](https://learn.chatgpt.com/docs/hooks))。

```powershell
cargo build --locked -p xtask --profile hooks
```

setup は共有 `.awake/codex-log-level.previous` に元の値の有無と内容を保存する。再実行で上書きしない。
<!--cmd:awake.codex.setup.undo-->`cargo xtask awake codex setup --undo` は、現在値が `debug` の時だけ
元の値へ戻す。後からユーザーが変更した値は残し、Codex のログ補助を無効にする。
変更したログレベルを反映するには Desktop の完全終了・通常起動が必要。

debug は実行コマンド・質問・応答などの payload もローカルログへ書く。解析器はその内容を
`.awake` の診断や history へ複製しない。設定とログ書式は Desktop 26.1007.2314.0 の実装で確認した
非公開の仕組みであり、アプリ更新後は診断と実験で再確認する。通常の `info` には要求の payload がない。

## 共有 holder の役割

native hook が prompt、tool 開始・終了、Stop、Interrupt、SessionEnd、子agentの活動を記録する。
Claude の `r1` と Codex の `c1` は activity protocol 1、holder revision 16 を共有する。
`PermissionRequest` は権限判断より前の通知なので、人待ちにしない。自動審査中は保持を続ける。
hook は実行の許可・拒否や席の lock を変更しない。

`crates/xtask/src/awake/codex/log.rs` の観測器を共有 holder に組み込み、
`%LOCALAPPDATA%/Codex/Logs/YYYY/MM/DD/codex-desktop-*.log` の以下の行を読む。
Store版の `%LOCALAPPDATA%/Packages/OpenAI.Codex_*/LocalCache/Local/Codex/Logs` も読み、
同じファイルが両方から見える場合は file identity で重複を除く。

```text
... debug [conversation-manager] Received server request id=... method=... params={...} ...
... info [conversation-manager] Sending server response id=... method=... response={...} ...
```

- 実際の command / file / permissions 承認、blocking 質問だけを、その呼び出しの人待ちにする。
- app session・logger・renderer・host・request ID を同じ要求と応答の組に使う。
  thread ID・item ID・hook の開始時刻で open call に対応付ける。内容の類似度で推測しない。
- 応答で同じ要求の待機だけ解除し、実行結果が出るまで native call を作業中として保持する。
- MCP elicitation は thread と server、request ID で読む。同じ thread / server に open call が
  1つだけの時に対応付け、同じ call の複数ダイアログは1 call分として数える。複数 call が候補なら
  曖昧として native 判定を残す。古い要求で後続の call を解除しない。user verification は人待ちに数えない。
- 非同期の質問を保持解除の理由にしない。親・子・別chatは独立した thread と activity で数える。
  親 idle / 子 working は保持、全員人待ち・終了なら解除、別chatが作業中なら全体は保持。
- Codex の実プロセスを activity の所有者とする。共有プロセスの子を時間帯だけで会話へ帰属させない。
  終了済み call がバックグラウンドへ残したプロセスは自動では保持しない。

ログ補助は hook の記録を変更しない。ログ欠損・壊れた JSON・ID再利用・対応付けの曖昧さでは
native 判定を残し、診断へ示す。欠損で作業中の保持を解除しない。読取中に claim が変わった時も
対応付けを確定せず、次の読取でやり直す。ファイル identity でローテーションと部分行を扱う。
ファイルイベントで起き、取りこぼしは既存の15秒 pollingで補う。OSのスリープ期限を延ばす操作はしない。
現在と前日のログだけを読み取る。

## 診断と手動 fallback

<!--cmd:awake.status-->`cargo xtask awake` は activity、holder の保持判定・heartbeat・history と、
`codex-log-r16` の状態を表示する。

- `watching debug=observed age-ms=...` は読めた debug 行の新しさ。古い debug 行だけで設定反映済みとは判断しない。
- `overlay` は対応付けた item、`pending` は入力待ちの MCP server、`unmatched` / `ambiguous` は
  対応できなかった要求。`fallback: ...` はログ補助を捨てて native 判定を使っている状態。
- `transport=file-events` または `polling`。初回のログ未作成や watcher 未起動も polling で再確認する。

trusted hook が未稼働なら、Codex 内で次のコマンドを使う。
外部 PowerShell の `CODEX_THREAD_ID` 設定だけでは activity の実 Codex 祖先を確立できない。

1. 作業開始に <!--cmd:awake.codex.work-->`cargo xtask awake codex work`。Git lock から自分の席を読む。
   1時間の TTL 前に更新する。長時間のチェック中も同じ。
2. 人待ちへ渡す前・ターン終了前に <!--cmd:awake.codex.idle-->`cargo xtask awake codex idle`。
   スリープ保持だけ解除し、席を返さない。

`CLAUDE_PID` / `CLAUDECODE` は設定しない。Windows 以外の activity は既存 awake と同じ no-op。

## スリープ3分での実験

他の Claude / Codex の作業が止まった時に行う。他の作業があれば共有 holder の保持は正しい動作なので、
他の席や activity を強制解除しない。Desktop 自身の「実行中のスリープを防ぐ」設定が有効なら、
実験時は無効にして holder の判定を観測する。Windows のスリープを3分にし、画面消灯とは区別する。

1. 初回設定・hooks profile のビルド・完全終了・通常起動を行い、Settings → Hooks で信頼する。
   この席で作業を始める。外部 PowerShell から席で `cargo xtask awake` を読み、
   自分の `codex <session>` と holder の `holding`、新しい debug 行が観測されることを確認する。
2. Codex に「PowerShellで `Start-Sleep -Seconds 240` を実行し、同じプロセスの終了まで待つ。
   スリープ制御の手動コマンドは使わない」と指示する。操作せず3分を超えて保持されることを確認する。
3. 実承認を試す chat はユーザーが「承認を求める」に設定する。
   「外部ディレクトリを読むためのサンドボックス昇格付きで、読み取りと
   `Start-Sleep -Seconds 240` を同じコマンドで実行し、終了まで待つ」と指示する。
   自動審査では実承認画面が出ないため、この段階の待機実験には使えない。
4. 承認画面で回答せず、`awake` の `overlay` が増え、他に作業がなければ `released` になることを確認する。
   その後は外部 PowerShell も閉じ、入力せず3分以上放置する。Windows の実スリープを確認する。
5. 復帰して承認する。結果が出る前の240秒実行中に `holding` が戻り、実行終了・返信後に
   `released` へ戻ることを確認する。`git worktree list --porcelain` では席が同じchatに locked のまま。

`awake` の表示だけで実スリープ成功としない。必要なら `.awake/history` の時刻と Windows の
Kernel-Power 42 / Power-Troubleshooter 1 を照合する。
ID不一致では `unmatched` が増えて native が保持するので、その診断と Desktop 版を残す。
通常起動で debug が増えなければ環境変数が起動元へ反映されていない。Windowsへ再サインインして
通常起動し、観測を確認する。起動ラッパーで回避しない。

子agentの実験はユーザーが明示的に許可した時だけ行う。親を idle にして子1つが作業中なら保持、
全員が人待ちなら解除、別chatに作業を始めると保持、という順で確認する。

回帰テストは実 Windows holder の承認応答後・結果前の保持、auto review、子・別chatの分離に加え、
ログの部分行・ローテーション・古い応答・ID衝突・claim変更・MCPの後続 call・設定の復元を確認する。
fixture の検証と、通常起動後の実 UI / 実スリープの確認は別の証拠として報告する。
