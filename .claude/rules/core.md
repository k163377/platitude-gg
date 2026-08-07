---
paths:
  - "crates/platitude-core/**"
---

# platitude-core 規約(git サブプロセス・セッション実装)

core のファイルを読み書きすると自動ロードされる。常時必要な不変条件は CLAUDE.md、app / QML 側は `.claude/rules/app-ui.md`、検証手順は verify-ui スキル。

## git サブプロセス規約

- コマンドは**引数配列**で組み立てる。シェル文字列の連結禁止
- パースは機械可読形式のみ: `--porcelain=v2` / `-z`(NUL 区切り)/ `--format=` を常用。人間向け・ローカライズされ得る出力をパースしない
- 実行時の環境変数: `LC_ALL=C`、`GIT_TERMINAL_PROMPT=0`(プロンプトでハングさせない。認証は credential helper に委譲)、status 等の読み取り系ポーリングは `GIT_OPTIONAL_LOCKS=0`
- **`GIT_LITERAL_PATHSPECS` は使わない** — git 内部の pathspec magic まで無効化し、`git stash push -u` が成功を報告しながら untracked を一切 stash しなくなる(実測)。パスは 1 件ずつ `:(literal)` で包む(`process::literal_pathspec`)。`--no-index` の引数は pathspec ではないので付けない
- 失敗メッセージは stderr 優先・空なら stdout(`git commit` の「nothing to commit」は stdout に出て exit 1 する)
- `git merge --continue` / `git rebase --continue` は**引数を一切受け付けない**(`--no-edit` も不可)。`--no-edit` を渡すのは cherry-pick / revert のみ
- **`git switch --merge` は使わない** — conflict しても exit 0 の「成功」で着地し、`MERGE_HEAD` を作らないので `merge --abort` が効かず(素の switch も「needs merge」で拒まれ、逃げ道は捨てる `switch --force` だけ)、さらに **staged が 1 つでもあると衝突ファイルでなくても拒否**する。未コミット変更を移動先へ運ぶのは **stash → switch → `stash pop --index`**(`session::checkout_merging`)。staged / unstaged の区別が残り、conflict 時は stash が残って戻れる
- **`git stash pop` の非ゼロ終了は「何も起きなかった」を意味しない** — 作業ツリー側の conflict なら**マージ済み**でマーカーを残し stash も残す(`Index was not unstashed`)が、staged 側が衝突すると**丸ごと拒否**して何もしない(`conflicts in index. Try without --index.` → `--index` 無しで再試行する)。判定は exit code ではなく status の unmerged 有無で行う(実測)
- 書き込みは**セッション単位のキューで直列化**する(ロックでは順序が保証されない — spawn したタスクが mutex を取る順は実行順と一致しない)
- **複数コマンドの合成は 1 手目が失敗したら止める**(`?` で伝播。途中まで進めた状態で次を撃たない)。落とし穴は**失敗が `Ok` に化ける経路** — `switch` の拒否(`CheckoutOutcome::Blocked`)と `stash pop` の非ゼロ終了は成功として返るので、そこだけは明示的に判定し、**戻せるものは戻す**(`session::checkout_stashing` / `checkout_merging` は拒まれたら stash を pop で戻す)。失敗後に走ってよいのは読み取りだけ(`catch_up_after` の fetch)
- `git config <key> <value>` に **`--` セパレータを付けない**(`--` 自体が値として保存される)。ダッシュ始まりの値はそのまま渡して通る
- **identity(`user.name` / `user.email`)に独自バリデーションを足さない** — git が拒むのは**空の name だけ**(空 email は通り author 行が `<>` になる)。`<` `>` と改行は author 行から黙って落とされ、前後の空白・句読点は削られる。config 書き込み時に改行は `\n` にエスケープされるので設定注入は起きない(実測)
- **署名の有無を `%G?` だけで判定しない** — SSH 署名は `gpg.ssh.allowedSignersFile` 未設定だと未署名と同じ `N` を返す。`git cat-file commit` のヘッダ(`gpgsig`)で存在を確認する。署名パスフレーズはアプリが扱わない(gpg-agent / ssh-agent の pinentry に委譲。**GUI pinentry 必須** — サブプロセスに端末が無い)
- **`ls-files --eol` は worktree の実ファイルを読む** — `w/` 列のために内容を全部読むので、グロブを渡すと基準リポジトリ級で数十秒(`JetBrains/kotlin` 全件 24.7s / 120MB のファイル 1 件 213ms、実測)。**候補列挙(`ls-files`、索引のみ = 64,012 件ヒットで 85ms)と EOL 読み(`--eol` に確定した数パスだけ = 42ms)を必ず別コマンドに分ける**。未チェックアウトのパスは `w/` が空で返る
- **`diff --ignore-cr-at-eol` で作った patch は `apply` できない**(実測 `patch does not apply`)。EOL のみの差に掛けると diff が空になる一方 status は modified のまま — 表示用と staging 用で diff の bytes を分けない不変条件(`details::file_diff_raw`)を壊すので使わない
- **worktree の diff は index 空間で出る** — `core.autocrlf=true` でも patch に CR は現れず、`apply --cached` / `apply -R` はそのまま通る(untracked の `--no-index` にも変換が効き `git add` と結果が一致する)。**index blob に CR があるファイルは `autocrlf=true` でも変換されない**ので、設定を切り替えても全ファイルが modified にはならない(いずれも実測)
- 対話エディタを開かせない(`GIT_EDITOR` / `GIT_SEQUENCE_EDITOR` を非対話に固定して rebase 等を駆動する)
- 全実行にタイムアウトとキャンセルを付ける。auto fetch は多重起動を防ぐ
- Windows ではコンソールウィンドウを出さない(`CREATE_NO_WINDOW`)
- UI(Qt)スレッドでサブプロセスの完了を待たない — git 実行は常にバックグラウンド
- **新しいオプションは最低 git バージョン([要望.md](../../internal-docs/要望.md))のマニュアルで存在確認してから使う** — 開発機の git は最新なので、`config get`(2.46)のように手元で動いて最低版に無いものが素通りする。照合の記録と手順は [git最低バージョン整合.md](../../internal-docs/git最低バージョン整合.md)(全発行コマンド照合済み)

## セッション・実装の決定事項

- 書き込みは `RepoSession` のキュー経由で直列化され、成功・失敗いずれでも refresh する。失敗は git の文言のまま `WriteFinished{error}` → 既存のエラー表示へ流れる
- **コマンドログは executor の observer 1 本で取る**(`process::CommandObserver`。spawn は `execute` の 1 箇所)。セッションは同じ observer に**利用者用と背景用の 2 つのハンドル**を挿し(`GitExecutor::observed`)、書き込みキューだけが利用者用を使う = 分類がキューの分岐 1 箇所で決まる。**auto fetch はキューを通るが背景扱い**(オフラインで毎分パネルが開くのを防ぐ)。表示用文字列(`describe`)とコピー用の完全形(`-c` 群 + 環境変数まで)は別で、記録しない時は `records()` で早期に降りて組み立てない
- **終了コードで答える問い合わせはコマンドログの失敗にしない**(`GitCommand::answers_by_code()`。`merge-base --is-ancestor` の exit 1 は答えなので、受け取っただけでログが飛び出さない)。**`run_unchecked` で非ゼロを分岐に使っている箇所は全部これが要る** — `sequencer::resolve` の `rev-parse --verify --quiet` は「最初のコミットに親は無い」を exit 1 で受けており、付け忘れると**根に届く squash / drop のたびにパネルが開いた**(実測・修正済み)
- interactive rebase は `GIT_SEQUENCE_EDITOR` に**別実行ファイル `pg-todo-editor`** を差す方式。**配布物に同梱必須**(本体と同じディレクトリ)
- グラフは **2 段ストリーミング**(タグ無し即描画→タグ込みを単一 drain で無フリッカー置換。44k タグの topo フロンティア初期化コスト対策)+ `--max-count=2000` ウィンドウ。**WIP(未コミット)を HEAD の子の仮想行**として、**stash を walk 参加の実行行**として描く(合成親コミットは sift で除去)— いずれも `platitude-core::session` 参照。**バックグラウンド更新(auto fetch 後・write 後・dirty ⇄ clean 変化)は `refresh_log()`** = オフスクリーン構築→送信済み行と一致ならイベントを一切出さない(アイドル中のチラつき対策)。リセット→ストリーミングは open / Reload / タグ切替 / 件数変更のみ
- **表示更新はポーリング**(watcher を入れない — WSL / ネットワークで通知が来ず、他 GUI は例外なく watcher + 手動 refresh の両方を持つ)。**契機はフォーカスではなく可視性**(最小化中のみ停止・最前面タブのみ)、間隔は `Metrics.pollIntervalMs`。1 tick = `RepoSession::refresh_poll`(refs + status のみ。前回未了と書き込み中はスキップ)→ どちらかが動いた時だけ `refresh_log()`。**refs が動いたら必ずグラフを作り直す** — チップだけ貼り替えると walk が見ていないコミットを指してチップが消える。コストは [ci/baseline/poll-cost-windows-x64.md](../../ci/baseline/poll-cost-windows-x64.md)
- **タグのリモート状態のデータ**: fetch が `ls-remote --tags` を続けて撃ち(`remote::list_tags`)、名前とピール済みコミットを session が remote 別に保持する(ブランチと違い `refs/remotes/` に相当する記録が無く、fetch 済みタグは手元のものと見分けが付かないため、訊く以外に手が無い)。届かなかった remote は前回の答えを保つ。**尋ねられずに読むのは auto fetch が有効 かつ グラフがタグを出している時だけ**(`session::catch_up_remote_tags`。前者は通信の許可・後者は優先度)。**この読みは書き込みキューを通さない** — 通すと `write_busy` が立ってポーリングが止まり、以後の書き込みが全部バッジ 1 つの後ろに並ぶ。開いた直後(`repo::open` 成功後)と、設定で auto fetch を入れた時の 2 箇所から入る。**ズレは fetch では直らない** — `--prune` は黙って手元のタグを残し、`--prune-tags` は `would clobber existing tag` で exit 1 する。**リモートにしか無いタグ**が残るのは fetch が落とせないコミットを指す時だけで、落とせるものは auto-follow が手元に作る = 状態が消える(いずれも実測)
