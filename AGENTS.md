claudeはこの資料を無視して

# Codex 向けの補足規約

開発の主環境は Claude。`CLAUDE.md` が共有規約の正本なので、プロジェクトの作業前に一度読む。
本ファイルは、その上に Codex 固有の実行手順を補う。

## 作業前に必要な文脈を読む

- 新しい作業や間が空いた後は、関連する最新の Claude セッションを読み、現状を把握する。
  プロジェクト・worktree・更新日時で探し、最後のユーザー依頼とアシスタントの本文を取り出す。
  最初は 1 セッションの限られた抜粋から始め、判断に必要な情報が足りない時だけ広げる。
  過去の報告は現在の Git の状態と照合する。
  Claude Cloud session の本文を読めなかった場合は、その事実を返信で人間へ報告する。
- 編集前に `git status --short --branch` と対象範囲の差分を読む。同じ作業中に読んだ情報は再利用し、
  元の情報が変わった時に読み直す。
- `crates/**` では `.claude/rules/code.md` と `structure.md`、`crates/platitude-core/**` では
  `core.md`、`crates/platitude-app/**` では `app-ui.md` も読む。これらは `.claude/rules/` からの
  相対パス。自動で読み込むのは Claude だけ。
- Claude のフック(`.claude/settings.json`)は Codex では動かない。Claude なら拒否される操作も
  自動では止まらないため、その規則を自分で守る。`.codex/hooks.json` の活動フックは Claude と
  Windows のスリープ保持機構を共有する。設定と手動の代替手順は `internal-docs/codex/ハーネス.md`。
- UI の検証・起動では `.claude/skills/verify-ui/SKILL.md` を使い、動詞表は選んだ操作の項だけを探す。
- 検査の実行前と作業の完了報告前に `.claude/skills/gate/SKILL.md` を読み、確認の段と gate の時機に従う。
- レビューだけの依頼や Claude 向けの修正指示書には、`.agents/skills/platitude-claude-review/SKILL.md` を使う。
  実装中や land 前のレビューでは、共有の `.claude/skills/review-points/SKILL.md` を直接使う。
- エージェント規約・スキル・ハーネス文書の編集には、`.agents/skills/platitude-policy-edit/SKILL.md` を使う。
  Codex 向けスキルは `.agents/skills`、Claude 向けスキルは `.claude/skills` で分けて管理する。
  共有規約は既存の正本を参照する。

## 席と作業状態

席の取得・継続・返却には `.agents/skills/platitude-seat/SKILL.md` を使う。
席の所有者は現在の `CODEX_THREAD_ID`。`xtask` はこの値を直接読む。
席の文字・ブランチ・絶対パスを直ちに報告し、割り当てられたツリーだけで作業する。
返却・rebase・GUI 起動・land の許可は `CLAUDE.md` に従う。
席の返却を報告する前に、解除結果と worktree のロック状態を確認する。
席の保持を解くのは land・返却・ユーザーが指示した乗っ取り
(<!--cmd:seat.takeover-->`PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover <letter>`)だけ。
Claude の会話と同じく、このチャットでの返信・依頼の完了・ユーザーの入力待ち・再開をまたいで席を保つ。
ユーザーが返却を指示した時だけ返し、land では共有の手順が返す。同じ席で作業を続ける。
rebase はその場でのユーザー指示がある時だけ行う。
作業中の変更は新しいコミットに積み、コミットログの squash / amend は land 時の再編で行う。

席での作業中は共有スリープ保持機構を使う。信頼済みのネイティブ活動フックが Codex の活動を記録し、
Desktop の debug ログで実際の承認・質問の要求と回答の観測を補う。
`cargo xtask` の <!--call:awake.codex.setup-->`awake codex setup` でログを一度設定し、
Desktop を完全終了して通常のアイコンから起動する。プロジェクトの活動フックは
Desktop の `Settings > Hooks`(CLI は `/hooks`)で確認する。
有効化と実験は [Codex ハーネス](internal-docs/codex/ハーネス.md) を参照する。
信頼済みの活動フックが動いていない場合は、作業開始時に `cargo xtask` の
<!--call:awake.codex.work-->`awake codex work` を実行し、1 時間の活動 TTL が切れる前に更新する
(長時間の検査中も同じ)。ユーザーへ応答を渡す前・ターン終了前に `cargo xtask` の
<!--call:awake.codex.idle-->`awake codex idle` を実行する。
スリープ制御を待機状態へ戻しても席は返さない。有効化のために `CLAUDE_PID` や `CLAUDECODE` を設定しない。
ネイティブフックは `PermissionRequest` の通知だけでは人待ちと判定しない。
ログの補助が使えない、または要求を対応付けられない場合は、ネイティブフックの活動判定を保ち、
`cargo xtask` の <!--call:awake.status-->`awake` で診断を確認する。
人待ちを推測したり、スリープを可能にするために席を返したりしない。

## 根拠を小さく、欠けなく読む

- パスの発見、シンボルの検索、必要な箇所の順で読む。シェルの出力は最初に 1,000〜2,000 トークンを目安にし、
  独立した読み取りをまとめる時は、各ファイル・差分が欠けずに届く合計の出力枠を取る。
- 出力の切り詰めは根拠の欠落。判断前に検索を絞るか、残りを区切って読む。
  JSONL のレコードや参照項目は、選んだ行が少なくても非常に長いことがある。
- 長時間のコマンドは出力全体を作業ごとのログへ保存し、終了コード・ログの場所・要約・関連する失敗箇所を示す。
  出力上限があっても、失敗した検査を成功扱いにしない。
- 元のプロセス・セッションのハンドルで新しい出力か終了を待つ。
  次にログの末尾・状態・プロセス一覧を見るのは変化があった後にする。
  待機のタイムアウトを完了や正しさの証拠にしない。独立した作業は、実行中の検査を有効に保てる時だけ行う。
- gate の既存の選択・ログ・キャッシュを再利用する。`--fresh`、リポジトリ全体の監査、
  通ったテストの再実行には、関係する変更か未解決の失敗が必要。
- サブエージェントは明示的に指示された時だけ使う。許可された範囲の成果に達したら止め、
  ブランチ/SHA・変更パス・検証結果とログ・未解決事項・次に許可されている操作を簡潔に引き継ぐ。
