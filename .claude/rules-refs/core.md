# platitude-core 各論(git の実測挙動・実装の決定事項)

[.claude/rules/core.md](../rules/core.md) の参照資料 — **1 項目 1 行**、コマンド名・モジュール名・関数名で Grep して該当行と前後だけを読み(全読みしない)、追記も 1 行(コード・docs の「core.md」参照はここへ移った行も指す)。

## git サブプロセス規約(各論)

- **`GIT_LITERAL_PATHSPECS` は使わない** — `git stash push -u` が成功を報告しながら untracked を stash しなくなる(実測)。パスは `:(literal)` で包む(`process::literal_pathspec`)。`--no-index` の引数は pathspec ではない
- `git merge --continue` / `git rebase --continue` は引数を受け付けない(`--no-edit` も不可)。`--no-edit` は cherry-pick / revert のみ
- **`git switch --merge` は使わない** — conflict しても exit 0 で `MERGE_HEAD` を作らず `merge --abort` が効かない上、staged が 1 つでもあると拒否(実測)。未コミット変更は stash → switch → `stash pop --index` で運ぶ(`session::carry_across`)
- **`git stash pop` の非ゼロは「無処理」を意味しない** — 作業ツリー conflict はマージ済み+stash 残置、staged 衝突は丸ごと拒否(`conflicts in index. Try without --index.` → `--index` 無しで再試行)。判定は exit code でなく unmerged の有無(実測)
- **終了コードを自分で読むコマンドには `answers_by_code()` を付ける** — 付けないとコマンドログが勝手に開く。対象は `switch` / `rebase` / `cherry-pick` / `revert` / `stash pop` / `rev-parse refs/stash`(exit 1 = stash ゼロ本。持ったことのない全リポジトリが返す)/ **config の読み取りは `--get` / `--get-regexp` とも全部**(未設定 = exit 1 = 答え。identity / remote list / tracking branches / push plan の upstream / merge tool / autocrlf / mergetool.cmd — 網は `tests/it/config_reads.rs`)/ **HEAD の読み**(`symbolic-ref -q` = exit 1 が detached、`rev-parse --verify -q HEAD` = exit 1 が unborn。`refs::head_state` と `remote` の current_branch)/ untracked の `diff --no-index`(exit 1 = 差分あり = 常態)/ stash rename の移動確認 `rev-parse --verify` — 網は `tests/it/answer_reads.rs`。**`cat-file` には付けられない — 無い側は exit 128 で 0/1 の枠外、しかも「リポジトリが無い」も 128 なので枠を広げると本物の失敗が消える。読む前に `rev-parse --verify -q <spec>` で訊く**(無い側は全部 exit 1 = 答え、在る側は 0 + oid を stdout。unborn HEAD の `HEAD:` / added の `<parent>:` / index に無い `:0:` で実測 2.55。`preview::blob_is_there` が preview と source_text の両方の入口)。代償は git プロセス 1 本 = Windows で ~25ms(`cat-file` も `diff-tree` も同じ ~24ms でプロセス生成が支配的)。`cat-file --batch-check` なら exit 0 で "missing" を返せるが **stdin 必須で不可**(プロセス層は stdin を常に閉じる — `scratch.rs`)。失敗判定は書き込みの答え側(規約 §git が言ったことを読む場所)
- **`--autostash` はどこにも渡さない** — autostash の復帰は素の `stash apply` で staged が全部 unstaged になる(`--index` を渡す口が無い。実測)。carry は自前(`session::carry_across_rewrite`)、代償は `--continue` / `--abort` 後の自動復帰。止まった時は `pop` しない — unmerged のある index へ git は書かない(判定は `opstate::detect`)
- **cherry-pick と revert は「空になった」の答え方が別**(実測 2.55・`integrate_integration`): cherry-pick は `CHERRY_PICK_HEAD` + stderr `The previous cherry-pick is now empty`、revert は `--allow-empty` が無く `nothing to commit` で拒まれ stderr 空・stdout に status。状態も 3 通り — cherry-pick は marker あり / 単体 revert は無し / 複数 revert は sequencer だけ(`opstate::detect` は None → `opstate::sequence_pending` を見ないと残骸を「きれい」と誤答)。判定は `integrate::left_nothing_to_record` / `still_stepping` の 2 か所だけ
- **元から空のコミットは `--allow-empty` で写す** — 付けないと「後から空になった」と同じ文言で止まり区別が付かない。`--empty=drop` は git 2.45 からで最低バージョン外
- **拒否の見分けは `integrate::work_is_in_the_way` の 1 か所** — 素の rebase と `--interactive` は同じ文言で拒む(実測 `integrate_integration`)ので carry も `integrate::RebaseOutcome` も 2 つに割らない。未追跡だけの木は拒まれない = stash も取らない
- **`session_integration::rewrite_route` は先頭一致で読む** — status refresh の `rev-parse --git-path rebase-merge/msgnum`(`conflict::rebase_progress`)が部分一致では 4 手目の `rebase` に化ける(実測)
- **`stash pop` の非ゼロを conflict と読めるのは元の tree に unmerged が無かった時だけ** — unmerged のある index へは丸ごと拒否(`needs merge`)で残る conflict は元のもの。`conflicts_now` だけ見ると無処理の pop が成功に化ける(実測。`session::stash_pop` の `settled_first`)
- **`rev-list --exclude` は名前空間を外して書く** — `--branches` に `refs/heads/` を付けると何も除外されず「他の誰かが掴んでいる」が常に真(エラーも警告も無し。実測)。`--tags` / `--remotes` も同様。`--glob` / `--all` だけ `refs/` 始まり
- **`--glob=<pattern>` は階層にしか当たらない** — `--glob=refs/stash` は 0 件(`refs/stash*` なら拾う。実測)。素の rev `refs/stash` は stash 無しで `fatal: ambiguous argument`、`--ignore-missing` は間違った oid も黙って捨てる = 安全側と逆。`reachable::command` は `--glob` を採る
- **`--all` で HEAD は外せない** — `--exclude=<現在のブランチ> --all` でも先端が negative に残る(実測)。外したい walk は `--branches --remotes` を並べる
- `git config <key> <value>` に `--` を付けない(`--` が値として保存される)。ダッシュ始まりの値はそのまま通る
- **`config --get-regexp` は低いレベル順・実効値は最後の行** — 1 件目で判定すると system が常に勝つ(Git for Windows は system に `core.autocrlf=true` を書く。実測)。単一キーは `--get`、複数キーは最後の出現で上書き読み。TestRepo が隔離するのは撃つ git だけで、コード側は開発機の system / global を見る
- **identity(`user.name` / `user.email`)に独自バリデーションを足さない** — git が拒むのは空の name だけ(空 email は通り `<>` になる)。`<` `>` 改行は author 行から黙って落ち、config 書き込みは `\n` エスケープで注入は起きない(実測)
- **identity の 2 連書きは原子化できない**(実測: `git config` は 1 プロセス 1 キー・`--edit` はロックを取らない・自前 lock は config ライタ再実装・libgit2 は絶対制約で閉)— 書いた後に実効設定を読み直し、違えば 1 回だけ撃ち直す(`identity::set_identity`)。半端(新 name + 旧 email)は `Identity::is_complete()` 真で黙って居座る。競合でない失敗は何度でも同じ = 2 回で止める。不採用: 書く前に読んで失敗時に戻す(戻しが同じロックを踏む)
- **半端の再現は多値の `user.email`**(`identity_integration`)— `--add` 2 回のキーへの set は `cannot overwrite multiple values` で exit 5、隣の `user.name` は通る(実測)。`.git/config.lock` 先置きで両方不着地も作れる(読みはロック不要)
- **署名の有無を `%G?` だけで判定しない** — SSH 署名は `gpg.ssh.allowedSignersFile` 未設定だと未署名と同じ `N`。存在は `cat-file commit` の `gpgsig` ヘッダで確認。署名パスフレーズは扱わない(agent の GUI pinentry 必須 — サブプロセスに端末が無い)
  - **`--format=` のどのプレースホルダでも見分けられない**(2026-08-11 実測 git 2.55): 未署名と ssh 署名済みで `%G?` `%GS` `%GK` `%GG` `%GF` `%GP` `%GT` が完全一致 — 1 コマンド化の道は無い
  - **先に `cat-file commit`、ヘッダ無しなら `Absent` で終わり**(`identity::verify_commit`)— 未署名は 1 プロセスで済み gpg / ssh-keygen を起動しない。`%G?` の `N` は「判定できなかった」と読む。プロセス数は `identity_integration` の observer テストが固定
- **`ls-files --eol` は worktree の実ファイルを読む** — グロブ渡しは基準リポジトリ級で数十秒(kotlin 全件 24.7s。実測)。候補列挙(索引のみ 85ms)と `--eol`(確定した数パスだけ 42ms)を必ず別コマンドに分ける。未チェックアウトは `w/` が空
- **改行コードの判定は patch の bytes から**(`eol::read`)— `ls-files --eol` は「今どう見えるか」で、どの行が新しいかは diff しか知らない。`--eol` は未追跡と近傍標本だけ
- **`diff --ignore-cr-at-eol` の patch は `apply` できない**(実測 `patch does not apply`)— 表示用と staging 用で diff の bytes を分けない不変条件(`details::file_diff_raw`)を壊すので使わない
- **worktree の diff は index 空間で出る** — `autocrlf=true` でも patch に CR は現れず `apply --cached` / `apply -R` が通る。index blob に CR があるファイルは変換されない = 設定切替で全ファイルが modified にはならない(実測)
- **`git mergetool` は `--no-prompt` だけでは stdin を踏む**(`conflict::mergetool`)— ツール未設定だと確認を stdin に出す(閉じていて失敗)ので `--tool=` 必須。`--gui` も必須(無いと `merge.guitool` と食い違う)。`mergetool.writeToTemp=true` を撃つ(既定は一時ファイルが隣に出て WIP ペインが untracked で埋まる)。`keepBackup` は触らない(`.orig` は利用者の安全網)。引数なしで撃たない(全衝突ファイルを順に開き書き込みキューが塞がる)。自前 `cmd` ツールは exit code を信用されず BACKUP の mtime 比較 — 保存せず閉じると「失敗」でマーカーが戻る
- **`mergetool --tool-help` は Windows で約 8 秒**(実測)— 書き込みキューに載せず・誰も待たせず・プロセスで 1 回だけ(`conflict::available_tools` の `OnceCell`)。`--gui` でも絞られず、端末ツール除外は説明文の `(requires a graphical session)` パースしかない(取れなければ候補ゼロ = 無害側)。自前定義は `config --get-regexp '^mergetool\..*\.cmd$'` が 34ms なので即出す

- **Windows ではプロセス 1 個が約 18ms — 対話の予算は本数で決まる**(実測 `git --version` warm 18.1ms)。行選択の詳細は `git show` 1 本にまとめる(`details::commit_details`。実測 25ms 対 直列 2 本 68ms)。`--diff-merges=first-parent` が要る(無いとマージのファイル一覧が空)。レコードと一覧の切れ目は NUL 10 個で決める(`split_record`)— 形で探すとメッセージ中の `M\tsrc/main.rs` が変更ファイル表に化ける。空コミットはレコード後ろに改行すら無い
- **`git diff-tree <p1> <c>` は `git show <c>` より遅い**(kotlin で 42ms 対 24ms。実測)— プランビング側が速いと決めてかからない

## セッション・実装の決定事項(各論)

- **メモリの内訳は `cargo xtask perf --breakdown` で出す**(推測しない)。取りこぼしは「残り」が膨らむ形で出るので総量は嘘にならない。実測記録は [ci/baseline/perf-windows-x64.md](../../ci/baseline/perf-windows-x64.md)
- **refs に比例する名前は `crate::Name`(`compact_str`)で持つ** — 24 バイトまで inline(ref 名の実測平均 20.2 字)。app 側から型の名指しが要る形にしない(§core の API は純 Rust 型のみ)。**`Footprint` は `SmallVec` に明示 impl が要る** — 無いと `[T]` へ deref して溢れたバッファを数え落とす
- **バイト数より確保の個数が効く帯がある** — 305,213 個の削減が計数 −3.1MB なのに WorkingSet −12MB(差はヒープ管理領域 = 計数アロケータに見えない側)。`classes=` の**件数**を必ず見る
- **mimalloc は不採用(実測)** — WorkingSet +38〜55MB / private +83MB、環境変数で抑えても private +43MB。起動・応答に差なし。数字は [perf-windows-x64.md](../../ci/baseline/perf-windows-x64.md) §アロケータと確保の個数
- **refs に比例する常駐データは「1 要素の器」を作らない**(タグ 45,901 本で実測): `BTreeMap` は 1 件でも 11 スロットのノード(43.5MB → `RemoteTagIndex` 6.5MB)/ `HashMap<K, Vec<V>>` は push 1 回で 4 スロット(20.5MB → `LabelIndex` 6.4MB)/ hex 40 字の `String` は 1 行 1 確保。`String` → `Vec<u8>` では 1 バイトも減らない — 減るのは表現を変えた時だけ
- **メモリ予算の 44%(実測 131MB)は空リポジトリで既に埋まっている**(Rust ヒープは 1.2MB)。「全部 Qt」と読まない — Qt の床 84MB / 自前 QML シェル 32MB / ページ実体化 16MB(積み方は ci/baseline/perf-windows-x64.md)。software 描画は代替にならない — GPU −25MB だが 56fps で予算割れ(実測)

- interactive rebase は `GIT_SEQUENCE_EDITOR` に別実行ファイル `pg-todo-editor` を差す。**配布物に同梱必須**(本体と同じディレクトリ)
- **全行 `drop` のプランを拒むのは `--root` の時だけ**(`sequencer::rebase_interactive`)— `--root` は placeholder へ replay するので空 tree・空メッセージのコミットが残る(実測)。回避に範囲を 1 つ広げない — 下のマージが範囲に入り `plan_edit` が拒む
- **テストが差し込む実行ファイルは rename で置く** — Linux は write fd の下の exec が `ETXTBSY`(git ではなく `sh` が Text file busy を返し rebase 失敗に見える)。`session_integration` の replay 系 5 本が持ち回りで赤くなったらここ(実測: 24 スレッドで 8 回中 6 回赤 → `Once` + staging へ copy + `rename` publish で 0)
  - **`--test-threads` を下げて隠さない** — 4 なら通る(実測)が CI のコア数で戻る。Windows では出ない — `cargo xtask linux` でしか観測できない
  - `Once` は copy を 1 回にするためでなく**待たせる**ために要る(exec が copy 中に重ならない)。staging 名を使い回すなら Once と対 — でないと書きかけが publish される
  - **fork が継承した write fd は Once + rename では防げない**(子の `execve` まで隣の exec が `ETXTBSY`。実測 8 回中 1 回、fork〜execve 間 300ms 停止で 100%)。手当ては exec 側の `ETXTBSY` リトライ(`run_published_helper`)
- **baseline を取る待ちは、走っている git を沈黙に数えない** — 沈黙だけだと待ちが途中で抜ける(実測: 5830ms の walk 中に無イベント 2415ms)。`support::settled` は本数一致と沈黙の両方を要求。open 自体を数える時は `set_record_background(true)` を `RepoSession::open` 直後、open 後だけを数える時は `opening_snapshots` 後に有効化して clear する(`Opened` の後から refs / status が始まる)
  - 開いた時の tag 込み pass はチップが出揃った後(実測: `LabelsChanged` の 639ms 後)なので、固定待ちの baseline の外に落ちて「バッジが walk を起こした」に化ける(`remote_tags_integration::learning_what_the_remotes_carry_repaints_chips_without_swapping_the_graph`。負荷でしか出ない)
- **グラフの walk は `--date-order`(`--topo-order` 不採用)** — topo は孤立ルートを端へ沈める(実測: `origin/gh-pages` が 323 行中 322 行目)。この形のテストは「孤立側の後に本流へもう 1 コミット」まで書かないと topo でも緑(`session_integration::an_independent_history_sits_where_its_date_puts_it`)。親を子より先に出さない保証は同じ(GraphBuilder・stash の sift の前提)。レーンは pull request 型で激減(実測: jackson-module-kotlin 277 → 9)、速度差なし
- グラフは **2 段ストリーミング**(タグ無し即描画→無フリッカー置換。44k タグのフロンティア初期化コスト対策)+ `--max-count=2000`。バックグラウンド更新は `refresh_log()` = 一致なら無イベント(アイドル中のチラつき対策)。リセット→ストリーミングは open / Reload / タグ切替 / 件数変更のみ
- **行番号を渡すイベントは渡した先が見ている世代を名乗る**(`LabelsChanged` の `generation`)— walk が入れ替わるとチップが別コミットに貼り付き、`applied` が残るので自己修復しない。守りは「読みと送信を同じ `shared` ロック内」と世代タグの 2 つ揃い。**`Shared::generation` は log_gen ではない** — swap を省いたパスも上げるので、写すと以後のチップが全部落ちる(実測・`remote_tags_integration` が赤で捕捉)
- **アバターは取り込む時に作り直す**(`picture::normalize`、保管は 256px PNG のみ)。ファイルサイズ上限は画素を縛らない(4MB の PNG の内側に 4GB の絵。実測 [ci/baseline/avatar-shrink-windows-x64.md](../../ci/baseline/avatar-shrink-windows-x64.md))— 寸法はヘッダだけ読んでデコード前に弾く(`picture::MAX_PIXELS`)。形式は中身で決める(`avatar::EXTENSIONS` はダイアログ専用)。不変条件「保管庫に入るものは必ず描ける」— 破ると輪だけの空円になり理由を言う場所が無い。`png` の `zlib-rs` feature を立てない(C の zlib に倒れ 3OS ビルドの前提が変わる)
- **アプリ自身の 2 ファイルは `settings` モジュール**(`settings.toml` = 人が決めた値 / `state.toml` = 自動)— 1 ファイルだと state フラッシュがエディタでの手直しを消す。ウィンドウ位置を roaming に載せない。環境変数が無ければ推測せずファイル無しに倒す。`Store::locate` は純関数 = mac / Linux の配置も Windows でテストできる
- **読みは `toml::Table` からキーごとに取る**(derive 不採用)— `#[serde(default)]` はキー欠落だけで、型違いは文書ごと失う(実測)= 打ち間違い 1 字が全設定のリセット。値域外も型違いと同じに扱う(幅 0 は `"wide"` と同罪)
- **書きは temp + `rename`** — 書きかけはパースに失敗せず `180` が `18` と読める(実測)ので、キー毎フォールバックでは拾えない。rename 失敗は古いファイルを残して諦める(次の tick が撃ち直す)
- **1 つの store を持てるのは 1 プロセスだけ**(`Store::claim`。ジャンプリスト起動で後勝ちの潰し合いが実際に起きた)。鍵はカーネル(`File::try_lock`)= kill でも stale が残らない。**ロックは専用ファイル `lock`** — 2 ファイルは rename で置き換わるので、そこへのロックは孤児を守る(実測: Windows でも rename は open ハンドル越しに成功)。置き場所は state 側(roaming に置くと別マシンへ同期される)
- **拒否の理由になるのは `WouldBlock` だけ** — ロックの仕組みが答えない(ネットワーク共有等)ことを窓の開かない理由にしない。ephemeral な store はロックを持たない = 自動化は何本でも並走する
- **開発ビルドは自分の store を持つ**(`platitude-gg/dev` / `dev-<tree>`)。同じツリーの debug と release は 1 つ(人にとって 1 つのビルド)。初回だけ実 store から `seed_from`(`avatars/` も — 索引だけだと顔の出ない行が並ぶ)。`PG_CONFIG_DIR` はビルドに関係なくそのディレクトリ(2 回の実行で 1 つを共有する検証が乗る)
- **`PG_*` が 1 つでも立っていたらファイル無しに倒す**(例外は `PG_CONFIG_DIR` と `PG_LOG`)— 無いとスクショ実行が実設定にウィンドウ位置を書く。`PG_LOG` を例外にしないと、ログを上げて自分の窓を触った人が設定を失う
- **両ファイルのパスは `repo_key` を通す**(区切り正規化)。**`\\?\` / `\\.\` だけは触らない** — Windows へ宛てた接頭辞(パス解析を切る指示)で、書き換えると別の場所を指す
- **担当を決めるのはキャンセルトークン 1 つ**(`log_cancel`。呼び出し順 = 担当の順)。`log_gen` は spawn 順で呼び出し順と一致しない — 両方を判定に使うと双方が降りて Reload が無反応になる。**降ろされたパスは state を一切触らない** — reset すると送信済み記録まで消え、次の再構築が同じ絵を貼り直す(アイドル中のチラつき)。世代カウンタは順序付け専用
- **`restart_log` の着地はストリームとは限らない** — cancel された walk は `LogFinished` / `LogFailed` を出さず、後ろの `refresh_log` が `LogReplaced` で置き換える(中身は正しい)。待つ側が形を決め打つと race 待ちになる(`session_integration::pass_of` が両方を読む。実測: 負荷下で時間切れ)。**swap を省く判定は行とフッタの両方**(`Shared::sent_footer`)— 行だけだとフッタしか動かない変更を追い越しで取りこぼし「履歴は切られている」と言い続ける(再現: `a_window_change_only_the_footer_notices_still_lands`)。同オプションの再構築は今までどおり黙る
- **表示更新はポーリング**(watcher 不採用: WSL / ネットワークで通知が来ない。他 GUI も watcher + 手動 refresh の両持ち)。契機はフォーカスでなく可視性、間隔は `Metrics.pollIntervalMs`。**refs が動いたら必ずグラフを作り直す** — チップだけ貼り替えると walk が見ていないコミットを指して消える。コストは [ci/baseline/poll-cost-windows-x64.md](../../ci/baseline/poll-cost-windows-x64.md)
- **refs のキーは join の入力を全部覆う**(`build::refs_key`)。**動いていない tick は join を組まない** — 毎 tick だと kotlin で 43.4ms/tick(`refs_join_at_scale` 実測)。publish は同じ `Arc` を出し続ける(attach 待ちの consumer 用)。世代は索引が実際に変わった時だけ上げる(`remerge_remote_tags`)
- **同じ答えを 2 度 git に訊かない**: HEAD は `for-each-ref` の `%(HEAD)` に在る(`refs::head_in`。印が無いのは detached と unborn だけ)/ `core.autocrlf` と remotes は `Derived::get_or_try_init` の single-flight(破棄は `forget_derived`、read 中の破棄は世代不一致で再読)。判定は同時 miss を作ってプロセス数も固定する(`concurrent_diffs_share_the_line_ending_setting_read`; 答えだけなら二重実行でも緑)。`stream_log` だけは refs 未着地なら `head_state` を撃つ
- **同じスナップショットの読みは 1 本しか走らない**(`ReadSlot`)。**2 人目は「もう 1 周」を予約する(落とさない)** — 落とすと書き込み直後の refresh が書き込み前の答えのままになる(`session_integration::a_request_made_while_a_read_runs_gets_a_read_of_its_own` が読みを窓際で止めて踏む。落とす実装でも 3 連打型のテストは緑)。`refresh_poll` と書き込み後は対象外(`poll_slot` / `write_busy` が持ち場)
- **タグのリモート状態は `ls-remote --tags` で訊くしかない**(`refs/remotes/` 相当の記録が無い)。届かなかった remote は前回の答えを保つ。尋ねられず読むのは auto fetch 有効かつタグ表示中だけ(`session::catch_up_remote_tags`)。**この読みは書き込みキューを通さない** — 通すと `write_busy` が立ってポーリングが止まる。**ズレは fetch では直らない** — `--prune` はタグを残し、`--prune-tags` は exit 1(実測)
- **diff の構文色は `highlight`**(syntect + two-face、fancy-regex = 純 Rust)。onig は C コンパイラの要る build script を連れてくるので不採用(`zlib-rs` を切るのと同じ理由)。fancy で落ちる定義は 4 本だけで Kotlin / Rust は残る
- **テーマは同梱を使わず `highlight::PALETTE` で組む**(規約 §シンタックスハイライト)。既製 4 テーマは実機で全て却下 — 地の文字が `textPrimary` より 1 段暗い(2026-08-13 ユーザー判断)。syntect の `default-themes` feature も落とした
- **構文の読みはファイル先頭から hunk まで歩いて fork する**(`highlight::patch_colors`)— hunk ローカルは文脈が薄いのではなく規則が違う(2026-08-13 実測: QML の `readonly property` は位置で別トークン、先頭行だけ色が違った)。歩く上限は 5,000 行(約 180ms)。ファイルが hunk と食い違ったら使わない(`hunk_agrees`)
- **コンフリクトのマーカは状態機械で読む** — 2 つの側は順序ではなく選択肢なので、片側が開いた `/*` を反対側へ持ち込まない(`>>>>>>>` 後は ours の終端状態)。判定は 7 文字ちょうど + 行末か空白、かつ `is_combined` の中だけ = Markdown の見出し下線もマーカ引用も素通し
- **`load_diff` の色付けは `spawn_blocking`**(大きい diff でランタイムのワーカを占有しない)。JoinError は色なしで出し直す — fancy-regex には panic の実績(bat #3156)
- **構文色の実測(2026-08-12、release / Ryzen 9 9900X)**: Kotlin 2,000 行の patch で 71ms、初回ロード約 6ms、diff 1 枚のヒープ増は約 2.8MB で頭打ち
- **`a_poll_rebuilds_the_graph_once` は `refresh_poll_tracked().outcome()` の後で数える** — 旧 600ms 待ちは負荷下で poll / rebuild の途中を読んだ。`RefreshTask` は無変更も含む tick と、その tick が要求した swap の完了までを因果的に閉じる
- **flaky 方針の pilot 実測(2026-08-16 / Windows)** — `rebuild` 6 件・`query` 8 件・`state` 5 件を 3 test binary process 同時のまま 10 回、計 190 件で NG 0。Cargo 自体の build lock を並列性と取り違えないため binary を直接起動し、各 harness の test thread 数は下げていない
