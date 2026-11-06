# PGG_AUTO_ACT 動詞の索引

[SKILL.md](SKILL.md) の参照資料で、**索引** — 1 動詞 1 行(近縁動詞は束ねる)に**名前・引数 / preset・何を見せるか**だけを書く。**報告行の読み方・待ち・罠は各動詞のドライバのコメントが正本** — `crates/platitude-app/src/auto/*.qml` の `act === "<動詞>"`(窓側は `Harness.autoAct === "<動詞>"`)の隣と、判定は `crates/xtask/src/verify/verbs/*.rs` の `Verb` 行。どちらも**動詞名を `"<動詞>"` の綴りで Grep する**(この索引では動詞名で行を引き、行末の「詳細は」の先を開く)。追記も 1 動詞 1 行で、それ以上の説明はドライバのコメントへ書く(何本かの動詞が共有する読み方だけ §動詞ごとの読み方・仕込み)。

**動詞を足す・消す・改名する時は登録表 `crates/xtask/verb-lines.txt` の行も揃える**(足した行には核の外で立てる部品を書く — run の `shown:` が会った物を言う。消した・改名した行は落とす、段分け表も) — `cargo xtask verbs` が崩れを言い、残した行は gate がその名前で走ろうとして `auto_act unknown=` から watchdog まで行く。

## 動詞一覧

- `details-failure --preset perf`: 実在しないOIDの詳細を要求し、失敗の表示を撮る — 詳細は `AutoActDetailsVerbs.qml`

- `perf none|details|diff|scroll|scroll-none|font-walk --preset perf`: 性能ドライバーの表示契約(性能値は `cargo xtask perf` の実測)— 詳細は `verify/verbs/graph.rs` の `perf` 行と `PagePerfDriver.qml` / `WindowPerfDriver.qml`

- `perf colour|diff-scroll|sequence --preset perf-sequence`: 固定Kotlin diffの最終色付け、可視diffスクロール、2 OIDを2周する表示契約 — 詳細は同上

- `commit`(引数はメッセージ。省略時は `chore: commit from the headless run`): commit して HEAD の行へ着地する — 詳細は `AutoActWipVerbs.qml`
- `amend`(同じく引数はメッセージ。省略時は `--no-edit`。`--preset marker` も撃つ): amend して着地する — 詳細は `AutoActWipVerbs.qml`
- `amend-row`(`--preset conflict-staged` / `feature/clash --preset clashing` も撃つ): WIP ペインに amend の行が立つか — 詳細は `AutoActWipVerbs.qml`
- `amend-tip --preset undone`: amend が HEAD を空にする時の commit ボタンの拒否とツールチップ — 詳細は `AutoActWipVerbs.qml`
- `edit-message-refused <要約を 1 語> --preset undone`: `amend-tip` の詳細ペイン版(保存ボタンの拒否)— 詳細は `AutoActHistoryVerbs.qml`
- `amend-reset-author --preset authorship`(引数は `amend` と同じ): HEAD の author を identity で取り上げる amend — 詳細は `AutoActWipVerbs.qml`
- `stash-file` / `switch` / `switch-remote` / `squash` / `edit-message` / `edit-message-leave`(`switch` を止まった木・conflict の残る木で撃つと watchdog まで待つ — その 2 つは `switch-stopped` / `switch-conflicted`)— 詳細は `AutoActWipVerbs.qml` / `AutoActSwitchVerbs.qml` / `AutoActHistoryVerbs.qml`
- `switch-remote-twice origin/feature/remote-only`: リモートにしか無いブランチのチップの連打を門が止めること(判定は報告行だけ)— 詳細は `AutoActSwitchVerbs.qml`
- `stash` [`named`]: ツールバーの `stash` で木ごと退避する — 詳細は `AutoActWipVerbs.qml`
- `stash-lands --preset conflict-staged`: 作業ツリーの行に立ったまま `stash` を押した時の読み手の着地 — 詳細は `AutoActWipVerbs.qml` / `AutoActLandings.qml`
- `stash-state ready|clean|conflicts|unborn`(4 本で 1 組。`clean` は `--preset tags` 等の綺麗な木、`conflicts` は `--preset conflict-kinds`、`unborn` は `--preset empty`): ツールバーの `stash` ボタンが木をどう読んだか — 詳細は `WindowBandActs.qml`
- `reword <新しい subject を 1 語>`(**引数が要る**): HEAD のメッセージを書き換える — 詳細は `AutoActHistoryVerbs.qml`
- `edit-message-focus 0 --preset co-authors`: description 枠に caret を入れた時の色(休息側の `co-authors 0` と対で読む)— 詳細は `AutoActHistoryVerbs.qml`
- `edit-message-away`(引数・preset 不要): caret を入れてから詳細ペインの帯を押し、保存行と caret が下りること — 詳細は `AutoActHistoryVerbs.qml`
- `cherry-pick`(引数は ref 名 / 完全な oid / `row:<n>`。下の `tip_landed` を読む)
- `stage-hunk` / `stage-line` / `push` / `force-push` / `force-push-hold`(`stage-line untracked:vendor/loose.txt --preset embedded` が行を持たない先へ移る側)— 詳細は `AutoActDiffVerbs.qml` / `AutoActPublishVerbs.qml` / `WindowBandActs.qml`
- `push-retry --preset diverged --allow-write-failure`: 非早送りの拒否を受けてから force で着地させる(判定は絵だけ)— 詳細は `AutoActPublishVerbs.qml`
- `reset-soft` / `reset-mixed` / `reset-hard`(引数は完全な oid か `row:<n>`。省略時は HEAD の 1 つ下の行): ブランチを戻し、着地を報告行で判定する — 詳細は `AutoActHistoryVerbs.qml` と `verify/verbs/sequencer.rs`
- `reset-hard-confirm`(引数は `reset-soft` と同じ): hard の回数押し行を出した所で止め、行末の note を読む — 詳細は同上
- `commit-menu` / `reset-menu` / `branch-card` / `tag-card`(引数はコミットの oid・省略で HEAD の 1 つ下の行。止まった merge の下は `--preset others-merging`。判定は overlay.png 側の報告行)/ `fetch` / `settings` — 詳細は `AutoActHistoryVerbs.qml` と `verify/verbs/graph.rs`(`fetch` は `AutoActPublishVerbs.qml`、`settings` は `AutoActFindVerbs.qml`)
- `github-card` / `github-choice` / `diff-github` / `github-ref` / `github-remote` / `github-file`(**`--preset github` で撃つ**。引数に `:others` を足すと OTHER REMOTES のカードまで開く): GitHub のページへのリンクの GITHUB カード(規約 §GitHub のページへのリンク)— 詳細は `AutoActGithubVerbs.qml` と `verify/verbs/github.rs`
- `menu-wait` / `menu-wait after`(`--preset slowhook`)/ `menu-wait stash` / `menu-wait stash-ref`(`--preset stashes`)/ `menu-wait replay`(`--preset replay`): 書き込みが走っている間に開いたメニュー(規約 §メニュー)— 詳細は `AutoActHistoryVerbs.qml` の `menuWaitTimer`
- `open-picker` [`worktree`]: ピッカーが開くフォルダ(ダイアログはどちらの PNG にも写らないので判定は報告行だけ)— 詳細は `verify/verbs/window.rs` / `WindowDialogActs.qml`
- `open-not-a-repo` / `open-bare` / `open-unreadable` / `open-not-a-repo-retry` / `open-not-a-repo-cancel` / `open-fail-tab` / `open-fail-tab-bare` / `open-fail-tab-unreadable`
- `open-fail-tab-log`(**開けないフォルダ**。規約 §リポジトリを開く。**引数を渡さなければ xtask がそのフォルダを作る**。`untrusted` は実リポジトリを引数に `--app-env GIT_TEST_ASSUME_DIFFERENT_OWNER=1`): 前半 5 つがピッカー経由のダイアログ、後半 4 つがタブの中の画面 — 詳細は `WindowDialogActs.qml`
- `clone-dialog` / `clone-go` / `clone-refused`(**引数なしで走る** — 取り寄せ元は開いているリポジトリ): リポジトリを取り寄せる箱(`CloneDialog.qml`)— 詳細は `WindowDialogActs.qml`
- `quit-waits` / `quit-locked`(**`--preset slowhook` で撃つ**): 書き込み中に閉じようとした窓と待ちのダイアログ — 詳細は `WindowDialogActs.qml`
- `quit-save-held`(**引数なし・preset 不要**): アプリ自身の設定保存が出ている最中に閉じようとした窓 — 詳細は `WindowDialogActs.qml` と `verify/verbs/window.rs`
- `preview` / `preview-unstaged` / `preview-staged`(**引数はパス(必須)**。画像は `--preset pictures`): ファイルのプレビュー — 詳細は `AutoActDiffVerbs.qml` と `verify/verbs/diff.rs`
- `diff-escape b.txt --preset dirty`: Esc が diff を下ろすこと(判定は行だけ)— 詳細は `AutoActDiffVerbs.qml`
- `plan-escape` / `plan-escape-held`(`--preset plan`): Esc がプランを下ろすのは出口が 1 押しの時だけ(`Discard` の 2 状態は `RebasePlanPane.qml`。判定は行だけ)— 詳細は `AutoActPlanVerbs.qml`
- `preview-close art/logo.png --preset pictures`: 画像を開いて閉じる(判定は行だけ)— 詳細は `AutoActDiffVerbs.qml`
- `dbl-local` / `dbl-remote` / `move-branch` / `commands` / `commands-fail`
- `name-branch <名前> --preset deep`(**引数と preset が要る**): 一番上の行に名前を付ける = 書き込み動詞が共通で通る barrier の主張を見張る席 — 詳細は `AutoActNavBoxVerbs.qml` と `verify/verbs/nav.rs`
- `commands-fail-shut` [`fold`](**`--allow-write-failure` が要る**): 印が赤いまま、パネルが下りている姿 — 詳細は `AutoActFindVerbs.qml`
- `commands-escape`(引数なし。**`--allow-write-failure` が要る**): 同じパネルを Esc で下ろす(`RepoPage.escapePressed`)— 詳細は `AutoActFindVerbs.qml`
- `commands-select` / `commands-copy`: ログのドラッグ選択と Ctrl+C(`CommandsPane.qml` / `models/commands/selection.rs`)— 詳細は `AutoActFindVerbs.qml`
- `commands-sweep`: ログを最終行の下の地から掴めること — 詳細は `AutoActFindVerbs.qml`
- `commands-clear`(**`--allow-write-failure` が要る**): ヘッダの `Clear` が行とヘッダの 1 行を空にする — 詳細は `WindowAutoActDriver.qml`
- `fetch-fail <回数> --preset unreachable --allow-write-failure`(1〜2 で警告の形、3 で止まった形): 連続した fetch の失敗 — 詳細は `AutoActPublishVerbs.qml`
- `fetch-busy`: 通信中のボタン(枠が turn していない側の待ち)— 詳細は `WindowBandActs.qml`
- `open-fetches 3 --preset behind`(引数は fetch が着いた後のグラフの行数): 開いただけで走る fetch — 詳細は `AutoActFileRowVerbs.qml`
- `fetch-tip on` / `fetch-tip off --preset noremote`(対で読む): 押せない fetch ボタンが hover に何も出さない側 — 詳細は `WindowBandActs.qml`
- `fetch-resume --preset unreachable --allow-write-failure`: 3 回失敗させてから、止まったボタンを押す — 詳細は `AutoActPublishVerbs.qml`
- `fetch-hover 0` / `fetch-hover 1|3 --preset unreachable --allow-write-failure`: ポインタが乗った時のボタンと、そこで開く文面 — 詳細は `AutoActPublishVerbs.qml`
- `fetch-tip-link --preset unreachable --allow-write-failure`: ツールチップの中の行き先を押す — 詳細は `WindowBandActs.qml`
- `fetch-recover`(既定 preset で自己完結。**`--allow-write-failure` が要る**): fetch の失敗が立てたヘッダの 1 行と赤い `>_` を、後続の成功が自動で下ろす — 詳細は `WindowAutoActDriver.qml`
- `fetch-recover-held`(**`--allow-write-failure` が要る**): 同じ回復を、読者が先に開けておいたパネルの上で — 詳細は `WindowAutoActDriver.qml`
- `nav-jump <section>[:<row>]`(撮るのは `worktree:1` / `worktree:5 --preset worktrees` 等): 左メニューの行の単クリックが連れて行く先(規約 §左メニューの所作)— 詳細は `AutoActNavVerbs.qml`
- `nav-dbl <section>:<name>`: 左メニューの行のダブルクリック(タグ行は書き込みの外)— 詳細は `AutoActNavVerbs.qml` / `AutoActCompletion.qml`
- `nav-reclick` / `nav-reclick-away`(引数 `<section>[:<row>]`。`--preset basic` の BRANCHES は `branch:1`): 畳んだレールが開いたセクションでの改名の所作(`ReclickGesture.qml`。対で読む)— 詳細は `AutoActNavVerbs.qml`
- `nav-rename-far --preset manytags`: スクロールで見えなくなった行に箱を開く — 詳細は `AutoActNavBoxVerbs.qml`
- `nav-branch-box` / `nav-rename-box`(引数 `<section>:<ref>[:<幅>][:away]`。`:away` は `--preset manytags` の TAGS): 左メニューの行の名前の箱を開いたまま止める(対で読む)— 詳細は `AutoActNavBoxVerbs.qml`
- `nav-rename-drop fold|away`: 空の箱のまま立ち去る 2 通り(規約 §左メニューの所作「Escape と他所のクリックで取り消す」)— 詳細は `AutoActNavBoxVerbs.qml`
- `nav-filter <文字列>`: 左メニュー上端の入力欄で 5 セクションを一斉に絞り込む — 詳細は `AutoActNavVerbs.qml`
- `nav-tip <section>:<row>[:<filter>]`(登録は `remote:0` / `branch:0`(意匠の 1 枚)/ `branch:2 --preset nested`): 開かない行の hover のツールチップ(開く行は `nav-open`)— 詳細は `AutoActNavBoxVerbs.qml` と `verify/verbs/nav.rs`

- `nav-open`(**左メニューの行が下へ開いて段で答える側** — 引数 `branch:<row>[:<filter>]` / `remote:<row>[:<filter>]` / `worktree:<row>` / `head[:<filter>][:<畳む行>]`(現在ブランチの張り付き行 `HeadPinRow`)。引数ごとの preset は登録表の行 — 詳細は `AutoActNavBoxVerbs.qml` の `navTipTimer`、引数ごとの判定は `verify/verbs/nav.rs`)
- `nav-open-foot`(引数なし / `away` / `bar`、**`--preset stack` 必須**。末尾までスクロールした一覧の最後の行が開く側 — 詳細は `AutoActNavBoxVerbs.qml` の `navTipTimer`)
- `nav-open-then`(引数 `away` / `edit` / `menu` / `filter` / `rightclick` / `rightclick-name` / `sweep` / `tap`(行は `branch:2` 固定)。行が開いた後に手が次にやること — 詳細は `AutoActNavBoxVerbs.qml`)
- `nav-drag-open`(引数 `<section>:<row>[:<filter>]`(既定 `branch:2`)。閉じた行を待たずに掴む = 行がその場で開き、押下が行の席のフルネームの選択になる(規約 §左メニューの所作)— 詳細は `AutoActNavBoxVerbs.qml` の `dragOpenTimer`)
- `nav-open-tip`(引数 `<section>:<row>`(既定 `worktree:3`)、`--preset worktrees`。開いた行に手を置き続けた時の補足 = WORKTREES の行のパス全体 — 詳細は `AutoActNavBoxVerbs.qml` の `openTipTimer`)
- `nav-open-tag`(引数 `<タグ名>[:remote|:drift][:tip|:name]`、`--preset tagremotes`。TAGS の行がその名前を持つリモート 1 本 1 段で開く側 — 詳細は `AutoActNavBoxVerbs.qml` の `navOpenTagTimer`)
- `tag-line-menu`(引数 `<タグ名>:<段番号>`、`--preset tagremotes`。開いた TAGS の行の段を右クリックし、その段のリモートを名指した TAG カードまで開く — 詳細は `AutoActNavBoxVerbs.qml`)
- `tag-chip-menu`(引数 `<タグ名>@<グラフの行番号>`、`--preset tagremotes`。グラフ行のそのタグのチップを的にしたメニューから TAG カードまで開く — 詳細は `AutoActRefVerbs.qml` の `tagChipTimer`)
- `nav-open-held`(引数なし。名前の箱が開いている状態で行に手を置き、どの行も開かないこと — 詳細は `AutoActNavBoxVerbs.qml` の `openHeldTimer`)
- `nav-peek-open`(引数は行番号(既定 0)。畳んだレールの覗きの中で行が開く側 — 詳細は `AutoActNavVerbs.qml` の `peekOpenTimer`)
- `nav-pin-edge`(引数なし、**`--preset stack` 必須**。現在ブランチの張り付き行が、自分の行がスクロールで出て行った上の縁に乗る側 — 詳細は `AutoActNavVerbs.qml` の `pinEdgeTimer`)
- `nav-follow` / `nav-follow-lit`(引数 `<section>:<row>[:<filter>]`(**必須**)、登録表の行は `--preset tracked-elsewhere`。開いた段の名前を押す / その名前に手を置く(規約 §左メニューの所作)— 詳細は `AutoActNavBoxVerbs.qml` の `followTimer`)
- `nav-pin-seat`(引数 `<畳む行>[:<手を置く行>]`、`--preset nested`(1 段だけの形は `0 --preset unpublished`)。畳みで席を取った張り付き行を、手を置かずに撮る側 — 詳細は `AutoActNavVerbs.qml` の `pinSeatTimer`)
- `nav-fold`(引数なし / `no-tags`。畳んだレールの数字が印の下に立つか(`counts=`)— 詳細は `AutoActNavVerbs.qml` の `navRailTimer` と `NavProbe.countsInk`)
- `tags-eye`(引数 `<タグ名>` / `<タグ名>:back`、**`--preset tagonly` の `v1.1-kept`**。TAGS 帯末尾の目でグラフからタグを外す・押し戻す — 詳細は `AutoActNavVerbs.qml` の `tagEyeTimer`)
- `nav-peek`(引数はセクション名 `branch` / `remote` / `worktree` / `stash` / `tag`)
- `nav-peek-away` / `nav-peek-into` / `nav-peek-out`
- `nav-peek-again`(引数はセクション名、既定 `branch`。クリックで閉じた覗きを、手を動かさずもう一度クリックして開く — 詳細は `AutoActNavVerbs.qml` の `peekAgainTimer`)
- `nav-peek-shut`(引数は同じセクション名(判定語は `branch` を名乗る)。開いてから手を退く / リストへ入る / リストの外へ抜ける / セルを押すの 4 通りで、開いたままが正なのは `-into` だけ — 詳細は `AutoActNavVerbs.qml`。中身が 0 のセクションは `nav-peek` に `--preset empty`)
- `menu-peek`(引数 `behind`(既定)/ `under` / `own` / `own-swap`。畳んだレールのセクションと、立っているメニュー(規約 §メニュー・`SectionPeekPopup.qml`)— 詳細は `AutoActNavVerbs.qml` の `menuPeekTimer`)
- `nav-close`(引数はセクション名。そのセクションの見出しだけを閉じ、セクションが上に詰まったままかを絵で見る — 詳細は `AutoActNavVerbs.qml`)
- `nav-add-remote`(引数なし / `folded`、主題は `--preset noremote`。REMOTES 帯の末尾の `+`(`folded` は畳んだレールの 0 の REMOTES セル)からリモートを書き留めるダイアログを開く — 詳細は `AutoActNavVerbs.qml` の `navAddRemoteTimer`)
- `doors-held`(引数は他所のブランチ名、**`base --preset plan` で撃つ**。replay が走っている間、戸だけ押さえて他はフルの重さのまま残る窓 — 詳細は `AutoActNavVerbs.qml` の `navHeldTimer`)
- `push-default`(引数 `<remote>`、`--preset unpublished`。リモートに origin の印(push の行き先 + `checkout.defaultRemote`)を付ける — 詳細は `AutoActPublishVerbs.qml` の `pushDefaultTimer`)
- `push-target`(引数なし / `forkmark`、preset ごとに形が変わる(登録表の行)。ツールバーの push が「どこへ送るか」と言っている先 — 写真では判定できない。詳細は `AutoActPublishVerbs.qml` の `pushTargetTimer`、preset ごとの判定は `verify/verbs/remote.rs`)
- `push-hover`(引数なし、preset ごとに形が変わる(登録表の行)。操作パネルの push に手を置いた時のツールチップ = 送る数・上書きで落とす数を言う唯一の場所 — 詳細は `AutoActPublishVerbs.qml` の `pushHoverTimer`)
- `remote-menu`(引数 `<remote>` / `<remote>:marked`。リモート自身の行の右クリック(overlay.png)— 詳細は `AutoActPublishVerbs.qml` の `remoteMenuTimer`)
- `remote-url`(引数 `<remote>` / `<remote>:marked`。URL の訂正の形 = 印へのもう 1 つの入口(overlay.png)— 詳細は `AutoActPublishVerbs.qml` の `remoteUrlTimer`)
- `publish-remotes-marked`(引数なし、`--preset unpublished`。はじめての push の行き先の一覧に印が乗っている側(素の `publish-remotes` と対)— 詳細は `AutoActPublishVerbs.qml` の `publishMarkedTimer`)
- `nav-peek-rename`(畳んだレールの覗きの行から改名の箱を開く = メニュー側の入口。一覧は戻らない(規約 §左メニューを畳む・`SectionPeekPopup.pinned`)— 詳細は `AutoActNavVerbs.qml`)
- `nav-unfold`(`--preset manytags`。左メニューを畳む / 畳んだレールの 1 セクションを開く / 畳んで戻す(報告行 `nav_rail`)— 詳細は `AutoActNavVerbs.qml` の `navRailTimer`)
- `diff-fold` / `diff-unfold` / `diff-fold-by-hand` / `diff-fold-by-rename`
- `diff-keep-folded`(引数はパス(登録表の行は `b.txt --preset dirty`)。diff と左メニューの畳みの組 — diff が畳む / `✕` で戻す / 一覧を戻すと diff が閉じる(`-by-hand`)/ 改名の箱は一覧を戻さないので diff も残る(`-by-rename`)/ 手で畳んであれば畳んだまま。詳細は `AutoActNavVerbs.qml` の `navRailTimer`)
- `rename-branch` / `rename-tag` / `rename-stash`
- `branch-at-tag`(いずれも引数は新しい名前)
- `create-tag`(**引数は `v9.9-new` 固定**(`v9.9-new:box` は箱を開けた所で止まる)。グラフのコミットメニューからタグを作る道を端まで通す(規約 §タグを作る・送る)— 詳細は `AutoActRefVerbs.qml`、判定は `verify/verbs/nav.rs`)
- `rename-tag-box`(**引数は打つ名前**。TAGS の先頭行の改名の箱をその名前入りで開き、押す前に断る規則(規約 §答えの要らない報せ・§左メニューの所作「名前の衝突」)を撮る。**`rename-branch-box`** は同じ箱を今のブランチの行で開き、`<branch>:<打つ名前>` で HEAD 以外の行に開く — 詳細は `AutoActNavBoxVerbs.qml` の `tagNameBoxTimer`、名前ごとの判定は `verify/verbs/nav.rs`)
- `nav-tag-box`(引数は `nav-branch-box` と同じ `<section>:<ref>[:<幅>][:away]`。同じ名前の欄の 3 つ目のモード(タグ)— 詳細は `AutoActNavBoxVerbs.qml` の `navNameBoxTimer`)
- `delete-remote-tag` / `delete-tag-both`(引数は `<tag>:remote`、`--preset tags`(`delete-tag-both` は `v1.0` だけ)。タグをリモート側だけ / 両方から消し、その名前の読みが変わったことで判定する(規約 §左メニューの所作 の削除の表)— 詳細は `AutoActRefVerbs.qml` の `tagMenuTimer` / `tagGoneTimer`)
- `tag-menu` / `push-tag`(引数 `<tag>`、`<tag>:drift`、`<tag>:remote`、`<tag>:read`、`tag-menu` だけ `<tag>:drift:tip`。タグの ref メニューの TAG カードを立てる / その push 行を押す — 詳細は `AutoActRefVerbs.qml` の `tagMenuTimer`、判定は `verify/verbs/remote.rs`)

- `delete-branch`(**引数はブランチ名(必須)**。左メニューの ref メニューで素の `--delete` を押し、着地でメニューが閉じるか・拒否で行が `branch -D` に化けるかを見る。着地の判定は `base --preset plan`)— 詳細は `AutoActRefVerbs.qml`
- `delete-branch-refused`(引数はブランチ名。拒否で化けた行を出したまま止める。`feature/topic-a --allow-write-failure`)— 詳細は `AutoActRefVerbs.qml`
- `delete-branch-go`(引数はブランチ名。化けた `-D` 行の回数押しまで走らせる。`feature/topic-a --allow-write-failure`)— 詳細は `AutoActRefVerbs.qml`
- `delete-branch-chip`(**引数はマージ済みブランチ名 — `base --preset plan` で撃つ**。グラフ行の入口から同じ削除を押し、拒否が来ないと分かった時にメニューが消えるかを見る)— 詳細は `AutoActRefVerbs.qml`
- `delete-blocked-tip`(引数は `[<ブランチ名>][:remote]`・省略で現在ブランチ。押せない削除行のツールチップを強制表示する。登録は `:remote` と `feature/topic-a --preset worktrees`)— 詳細は `AutoActRefVerbs.qml`
- `menu-highlight`(引数はブランチ名・省略で現在ブランチ。キーボードの道でハイライトを作る = 重ね色のハイライトを絵で見られる唯一の経路)— 詳細は `AutoActRefVerbs.qml`
- `delete-branch-early`(引数はブランチ名。メニューを開いて**待つだけ・書き込み無し** — 開いた時に答えが揃った削除行を報告する。現在ブランチは削除行が無く即 FAIL)— 詳細は `AutoActRefVerbs.qml`
- `delete-branch-early-far`(引数は `parked`、**`--preset deep-parked` 必須**。`delete-branch-early` の `from=git` を毎回通す側。判定は報告行だけ)— 詳細は `AutoActRefVerbs.qml`
- `chip-menu`(**引数はブランチ名(必須)**。チップ右クリック相当 — 行のメニューをその名前へ向けて開く)— 詳細は `AutoActRefVerbs.qml`
- `chip-menu-current`(引数なし。現在ブランチのチップ — 第 1 階層に削除の表が立つ。**`chip-menu` と対で読む**)— 詳細は `AutoActRefVerbs.qml`
- `list-menu`(引数 `<行>[:<カードの行>]`(既定はどちらも 0)。**重なりの一覧の行の右クリック**。`--preset tags` / `--preset worktrees` の行が登録されている)— 詳細は `AutoActGraphRowVerbs.qml`
- `delete-gone`(**`v0.3-local --preset basic` で撃つ**。押した瞬間に行とチップが消え、git はその後ろで走っている途中を撮る唯一の動詞(規約 §消す操作は先に画面から消す))— 詳細は `AutoActRefVerbs.qml`
- `delete-stood-down`(**`delete-gone` の対 — 同じ `v0.3-local --preset basic` で「立て替えが終わる」まで待つ**。絵では判定できず報告行で読む)— 詳細は `AutoActRefVerbs.qml`
- `delete-tag` / `delete-stash`(`delete-remote` / `-go` は下のリモートブランチの並びの行が正)— 詳細は `AutoActRefVerbs.qml`
- `delete-stash-row`(**メニューが開いたまま止まり**、引数 `go` で行の回数押しを走らせる)— 詳細は `AutoActWipVerbs.qml`
- `worktree-menu` / `worktree-graph` / `worktree-remove` / `worktree-remove-refused` / `worktree-held`(**worktree の WORKTREE カード**(規約 §左メニューの所作 の削除の表 / §メニュー の入れ子)。**引数は worktree のフォルダ名**(`worktree-graph` は `<folder>[:go]`)、土台は `--preset worktrees`。`-menu` / `-graph` はカードを立てて止まり、`-remove` / `-graph <folder>:go` は一覧から消えるまで、`-held` は押下の途中を撮り、`-refused` は `topic --preset carried --allow-write-failure`。このタブが立つ worktree は preset では撮れない(`--repo` で撃つ))— 詳細は `AutoActNavVerbs.qml`
- `worktree-add-menu` / `worktree-box` / `worktree-new` / `worktree-add` / `worktree-new-refused`(**WORKTREE カードの worktree を作る 2 行**(規約 §worktree を作る)。**引数は `<kind>:<ref>[=<名前>][@graph]`**(kind は `branch` / `remote` / `tag` / `worktree` / `commit`)。`-menu` は行を立て、`-box` は箱で止まり、`-new` / `-add` はタブが新しい worktree に立つまで、`-refused` は `--allow-write-failure` が要る。土台は `--preset basic` / `worktrees` / `rebase-conflict` / `worktree-places`)— 詳細は `AutoActWorktreeVerbs.qml`
- `worktree-rename-box` / `worktree-rename`(**worktree の「間を空けた 2 回目のクリック」= 改名の所作**(規約 §グラフ行のダブルクリック「箱のプレースホルダ」/ §左メニューの所作)。**引数は `<folder>[=<名前>][@graph]`**、土台は `--preset worktrees`。`-box` は箱か断りの理由で止まり、`worktree-rename` は名前を送って一覧が替わるまで待つ。**このタブが立っている worktree の改名は `worktree-rename-stand`**)— 詳細は `AutoActWorktreeVerbs.qml`
- `stash-apply-row`(グラフ行の Apply)— 詳細は `AutoActWipVerbs.qml`
- `stash-pop-row`(グラフ行の Pop。本題は `--preset stashes`。pop した行がグラフから消え、名前がコミット箱へ戻るまで待つ)— 詳細は `AutoActWipVerbs.qml`
- `delete-force` / `delete-tag-go` / `delete-stash-go` / `discard-file` / `delete-file`(`delete-force` は UI を通らず `-D` を撃つ。名前にしか無いコミット行がグラフから消える側は `feature/clash --preset clashing`)— 詳細は `AutoActRefVerbs.qml` / `AutoActFileRowVerbs.qml`
- `discard-staged`(引数はパス。staged の行のメニューを出したまま止まり、行の文言を報告する)— 詳細は `AutoActFileRowVerbs.qml`
- `discard-file-go` / `delete-file-go` — 詳細は `AutoActFileRowVerbs.qml`
- `discard-staged-go`(行の回数押しを走らせる)— 詳細は `AutoActFileRowVerbs.qml`
- **file-row の一族(`file-menu*` / `discard-file*` / `delete-file*` / `discard-staged*` / `discard-many*` / `stage-many*` / `take-side-*` / `open-mergetool`)は WIP の行が実体化してから撃つ** — デリゲートはモデルより 1 レイアウト遅れて生まれるので、`AutoActFileRowVerbs.fileRowsTimer` が名指しの行を待つ。**行が来ない run(引数の渡し忘れ・パスの打ち間違い)は watchdog が診断**
- **書き込む側(`*-go` / `take-side-*` / `open-mergetool`)は着地まで待って撮る** — 書き込み境界は断られた write にも来るので、`AutoActDriver.treeBarrier` が名指した行がそのバケツから消えたのを確かめてから撮る(報告行 `tree_settled …`)。**書き込みが git に拒まれた run は行が残るので watchdog が診断**する
- `diff-file` / `line-tools` / `hunk-tools`(引数は `<パス>` か `<bucket>:<パス>`。**この 3 つと `stage-hunk` / `stage-line` / `keep-place` / `code-send` / `code-grow` / `code-shrink` / `code-swap` / `line-back` / `discard-hunk` 系は行が届いてから撃つ** — 全員が `diff_row act=` を報告し、**パスを渡し忘れると `diff_arg act=<動詞> named=false` の 1 行だけで終わる**。`line-tools` は行の `+`、`hunk-tools` は hunk 見出しの 2 語を手の下の形で撮る)— 詳細は `AutoActDiffVerbs.qml`
- `diff-select` / `diff-copy` / `diff-menu` / `diff-copy-removed`(**diff の本文の選択とコピー**(規約 §diff の中身をコピーする)。**4 つとも `notes.txt --preset manyhunks`**、同じ 1 本のドラッグ)— 詳細は `AutoActDiffVerbs.qml`
- `diff-find` / `diff-find-step` / `diff-find-lines` / `diff-find-options` / `diff-find-split` / `diff-find-drop` / `diff-find-keep` / `diff-find-close` / `diff-find-seed` / `diff-find-long`(**diff の中を探す**(規約 §diff の中を探す)。**10 とも `notes.txt --preset manyhunks`**)— 詳細は `AutoActDiffFindVerbs.qml`
- `diff-split` / `split-tools` / `split-copy`(**diff を 2 列で読む**(`DiffRowDelegate.qml`)。**3 つとも `notes.txt --preset manyhunks`**、`diff-split` は `src/steps.rs --preset coloured` でも撃つ)— 詳細は `AutoActSplitVerbs.qml`
- `diff-blank`(**`marks.txt --preset widechars` 専用**。行末より右の空白はその行の末尾 — ピクセルで入る唯一の動詞)— 詳細は `AutoActDiffVerbs.qml`
- `diff-sweep`(**`b.txt --preset dirty` で撃つ**。diff の本文を最終行の下の地から掴めること)— 詳細は `AutoActDiffVerbs.qml`
- `diff-bar`(**`notes.txt --preset manyhunks` で撃つ**。diff の縦バーが掴めること。絵は証人にならない)— 詳細は `AutoActDiffVerbs.qml`
- `keep-place`(**`notes.txt --preset manyhunks` 専用**。読み進めた位置で partial stage し、読み直しの後も位置が残るか)— 詳細は `AutoActDiffVerbs.qml`
- `colour-place`(**`src/steps.rs --preset coloured` が本番**。後から届く色が、読んでいた位置と横の行き先のどちらも奪わないか)— 詳細は `AutoActColourVerbs.qml`
- `eol-hover`(引数はパスか `<バケツ>:<パス>`、`--preset eol`。WIP のファイル行の改行コード警告 `!` のカードを開く。overlay.png に写る)— 詳細は `AutoActWipVerbs.qml`
- `eol-gone`(引数なし / `button` / `inside` / `button:inside`、`--preset eol`。手をカードの中に置いたまま警告を 1 回の書き込みで消し、カードが下りるか)— 詳細は `AutoActWipVerbs.qml`
- `eol-walk [<段>]`(段は `gone` / `back`、`--preset eol`。手の下で一覧を 1 行送り、光が来た行へ移って rest の後にその行のカードが出るか)— 詳細は `AutoActWipVerbs.qml`
- `eol-stay [<段>]`(同じ仕込みで一覧を 1px 送る = 手は同じ行に留まり、rest の後にカードが戻るか)— 詳細は `AutoActWipVerbs.qml`
- `peek-hold`(引数 `<how>[:released]`、how は `none` / `menu`(既定)/ `rename`。**畳んだレールのセクションの保持**(`SectionPeekPopup.qml`))— 詳細は `AutoActHoverHoldVerbs.qml`

- `menu-block`(引数 `<owner>[:<stage>]`、owner ごとの preset は登録表の行。**メニューの裏の hover**(規約 §メニュー)— 段と owner は `AutoActHoverHoldVerbs.qml` の `menuBlockTimer`、判定は `verify/verbs/panes.rs`)
- `author-swap`(引数 `<from>:<to>`(`author` / `mates` / `parents`)、`--preset octopus-mates`。**author 行のカードは 1 枚ずつ** — 詳細は `AutoActHoverHoldVerbs.qml` の `authorSwapTimer`)
- `list-reread`(引数 `<row|pin>:<how>`(how = `commit` / `amend` / `branch` / `branch-chip` / `pending`)、引数ごとの preset は登録表の行。**読み直しの下のチップの一覧**(デザイン規約 §hover のツールチップ「外乱では閉じない」)— 詳細は `AutoActHoverGoneVerbs.qml` の `listRereadTimer`)
- `eol-change`(引数 `unstaged` / `staged`、`--preset eol`。**EOL カードの文言が同じパスのまま変わる** — 詳細は `AutoActHoverGoneVerbs.qml` の `eolChangeTimer`)
- `wip-reread`(引数 `same`(既定)/ `above` / `inside` / `staged` / `gone` / `fixed`、`--preset eol`。**状態の読み直しで作業ツリーの行が動いた時の手** — 詳細は `AutoActHoverGoneVerbs.qml` の `wipRereadTimer`、判定は `verify/verbs/diff.rs`)
- `nav-reread`(引数 `above`(既定、`--preset manytags`)/ `short` / `gone`(`--preset basic`)。**読み直しの下の左メニューの開いた行** — 詳細は `AutoActRereadVerbs.qml` の `navTimer`)
- `recover-reread`(引数 `inside`(既定)/ `row`、`--preset discards`。**読み直しの下の破棄記録のカード** — 詳細は `AutoActRereadVerbs.qml` の `recoverTimer`)
- `diff-reread`(引数 `above`(既定)/ `gone`、`--preset manyhunks`。**読み直しの下の diff の行** — 詳細は `AutoActRereadVerbs.qml` の `diffTimer`)
- `tip-slide`(引数 `files[:<stage>]`(stage = `shown` / `slid` / `still` / `moved`(既定))、`--preset long`。**行と一緒に動く tip** — 詳細は `AutoActHoverGoneVerbs.qml` の `tipSlideTimer`)
- `card-latch [<段>]`(段は `card` / `menu` / `dismissed` / `back`(省略は `back`)。HEAD 行のカードをメニューの blocker が下ろして latch し、2px の揺れで解けて rest の後に戻る — 詳細は `AutoActCardFlowVerbs.qml` の `cardLatchTimer`)
- `card-rewrite [<段>]`(段は `inside` / `rewritten`(省略は `rewritten`)、`--preset dirty`。手が中にあるカードは、HEAD の行が commit で書き換わっても残る — 詳細は `AutoActCardFlowVerbs.qml` の `cardRewriteTimer`)
- `commit-face`(**`--preset signed`**。commit ボタンの顔の右上の署名チェックのツールチップ — 詳細は `AutoActWipVerbs.qml` の `commitFaceTimer`)
- `eol-commit`(引数 `amend` 可、**`--preset eol`**。警告が index に乗った木でコミットボタンにポインタを置く(何もコミットしない)— 詳細は `AutoActWipVerbs.qml` の `eol-commit` の枝と `eolCommitTimer`)
- `code-send`(**`wide.txt --preset widelines`** と 2 本目の `columns.txt --preset widechars`。**diff を横へ送る手 2 つと、下端のバー**(規約 §diff を横へ送る)— 詳細は `AutoActDiffVerbs.qml` の `codeSendTimer`)
- `code-grow late.txt` / `code-shrink top.txt` / `code-swap late.txt`(3 つとも **`--preset widelate`** 専用で、**絵は証人にならない**ので判定は報告行。**横の行き先がどこから来て、何に属しているか** — 後から組まれた行が幅を決める / 同じパスが短くなっても横位置を保って幅だけ縮む / 別ファイルへ切り替えると幅も位置も捨てる。詳細は `AutoActDiffVerbs.qml` の `codeGrowTimer` / `codeShrinkTimer` / `codeSwapTimer`)
- `line-run <パス>`(**`notes.txt --preset manyhunks` 専用**。**行を続けて 3 本ステージできること** — 詳細は `AutoActDiffVerbs.qml` の `lineRunTimer`)
- `diff-follow`(引数は `<パス>` か `<bucket>:<パス>`、場合ごとの preset は登録表の行。**側が空になった時にペインが次に読むもの**(規約 §diff の中のステージ)— 判定は `landed=`、詳細は `AutoActDiffVerbs.qml` の `followTimer`)
- `line-back <パス>`(**`notes.txt --preset manyhunks` 専用**。**diff の 1 行ステージと、右パネルからのファイル操作が同じ diff を指していること** — 詳細は `AutoActDiffVerbs.qml` の `lineBackTimer`)
- `stage-all` / `unstage-all`(**`--preset dirty`**。**バケツの帯の `Stage all` / `Unstage all` を実際に押し**、空にしたバケツの帯が残るか(規約 §その他の操作)— 詳細は `AutoActFileRowVerbs.qml` の `bucketAllTimer`)
- `resolve-all`(**`--preset conflict-kinds`**。`stage-all` / `unstage-all` と逆向き = 全部解決済みにしたら conflicts の帯が消える(規約 §その他の操作)— 詳細は `AutoActFileRowVerbs.qml` の `bucketAllTimer`、判定は `verify/verbs/diff.rs`)
- `discard-hunk`(hunk 見出しの回数押しボタンを出したまま止まる)
- `discard-hunk-go`(そのボタンを完走させる。完了は `stage-hunk` / `stage-line` と同じ待ちで、同項に書いてある)
- `discard-many` / `discard-many-go`(引数は一緒に選ぶもう 1 行のパス(`--preset dirty` なら `c.txt`)。先頭行 + その行を Ctrl クリックで選んで破棄の行を出す / 完走させる — 詳細は `AutoActFileRowVerbs.qml` の `runFileRowAct`)
- `stage-many` / `stage-many-go`(引数・preset は同じ。同じ 2 行を選んでから先頭行の `+` にポインタを置く / 押す。**先頭行は `rowAt(0)` = フォルダ行も掴むので、先頭がファイル行になる木で撃つ** — 詳細は `AutoActFileRowVerbs.qml` の `runFileRowAct`)
- `wip`(作業ツリーの一覧)
- `wip-embedded`(引数はその行のパスを**末尾の `/` まで込みで**(`vendor/nest/` / `vendor/fresh/`)、**`--preset embedded`**。**作業ツリーの中に居る別リポジトリの行**について diff ペインが言うこと — 詳細は `AutoActWipVerbs.qml` の `embeddedTimer`)
- `wip-tally`(同じ一覧を開き、**グラフの未コミット行の集計**を報告する。4 種類が同時に出る preset は無い — 建て方は `AutoActWipVerbs.qml` の `wip-tally` の枝)
- `wip-commit-half`(引数なし / `during`、**`--preset dirty`**。**staged の分だけをコミットして、木に残りを残す** = 行が「移る」所 — 詳細は `AutoActWipVerbs.qml` の `halfDuringTimer` / `halfSettledTimer`)
- `wip-message`
- `wip-message-focus`(commit 欄の description。**対で読む** — 引数(省略可)の body を打つ所までは同じで、`-focus` だけが caret を入れる — 詳細は `AutoActWipVerbs.qml` の `wip-message` の枝)
- `find` / `find-next`
- `find-prev`(Ctrl+F の検索。引数があればその文字列を箱に入れる(**空白を入れない**)。`-next` / `-prev` はさらに Enter / Shift+Enter 相当を 1 回。引数ごとの preset は登録表の行 — 読み方は `AutoActFindVerbs.qml` の `find` の枝と `findSettled`、判定は `verify/verbs/nav.rs`)
- `find-scroll`(本番は **`5 --preset deep`**。**検索が描かれた後にスクロールする側** — 絵で見るのはレーン。詳細は `AutoActFindVerbs.qml` の `findScrolled`)
- `find-hint`(**打った行に付く補足 = カードの下の 1 行**。引数を箱に入れ、手が止まったと製品が言うまで待って撮る。`head:<n>` は HEAD の id の頭 n 文字 — 詳細は `AutoActFindVerbs.qml` の `findHinted`、引数ごとの判定は `verify/verbs/nav.rs`)
- `find-hint-key`(引数は `find-hint` と同じ形、本番は `head:3`。**止まった後の 1 打で補足が消える側** — 詳細は `AutoActFindVerbs.qml` の `findHinted`)
- `find-further`(**`--preset deep` 専用**、引数 `root` / `zzz`。**窓が切れた 0 件の補足の押下** = `No results · Load <n> more commits` の後半 — 詳細は `AutoActFindVerbs.qml` の `findFurther`)
- `band-find`(**操作パネル右端の search を押した時**。プランが立っている間は撮らない — 詳細は `AutoActFindVerbs.qml` の `band-find` の枝)
- `find-drop`(引数なし / 先に箱へ入れる語。**カードが立っている間に押下が他所へ落ちた時** — 空なら閉じ、語があれば残る(規約 §コミットを探す)。**対で読む** — 詳細は `AutoActFindVerbs.qml` の `findDropSettled`)
- `name-box-drop`(引数 `<行>[:<打つ名前>]`、**行は 1 以上**。同じ押下を**グラフ行の名前欄**に当てる側 — 詳細は `AutoActPaneVerbs.qml` の `name-box-drop` の枝)
- `op-exit`(`--preset rebase-conflict` / `rebase-staged` / `rebase-empty` / `cherry-pick-conflict` / `conflict` で全状態が撮れる。止まった操作の出口カードと、隠れるメッセージの枠・commit ボタン(規約 §進行中の操作から出る)— 詳細は `AutoActWipVerbs.qml` の `opExitTimer`、判定は `verify/verbs/sequencer.rs`)
- `op-exit-go`(**引数はダッシュ抜きの語**(`continue` / `skip` / `quit` / `abort`、既定は無い)、preset はその行が出ている状態。出口カードの 1 行を完走させる — 詳細は `AutoActWipVerbs.qml` の `op-exit` の枝。着地は `op-exit-lands`)
- `op-exit-lands`(引数は同じ語、既定 `continue`。**`--preset rebase-staged` が本番**。**同じ押下の着地** = 止まった操作を下ろした読み手がどこへ置かれるか。**写真では判定できない** — 詳細は `AutoActLandings.qml` の `opExitLandTimer`、`quit` の着地は `verify/verbs/sequencer.rs`)
- `rebase-edit-stop-out`(**`--preset plan`**。**`rebase-edit-stop` の続き** — 同じ `edit` 停止をカードの `--continue` で下ろし、`op_exit_landed` を読む。詳細は `AutoActPlanVerbs.qml` の `planEditStopTimer`)
- `switch-lands`(引数 `<ブランチ>[:<stash の本数>]`。**2 本で 1 組** — `feature/clash:0 --preset clashing` と `feature/topic-a:2 --preset switch-carry`。**移動が落ち着いた所**(`switch` では撮れない)— 詳細は `AutoActSwitchVerbs.qml` の `switch-lands` の枝と `switchLandsTimer`)
- `switch-mark`(**引数は `<ブランチ>:asks` か `<ブランチ>:plain`(必須)**、形ごとの preset は登録表の行。**左メニューの ref メニューの `switch` 行が着る黄の `!`** — 詳細は `AutoActSwitchVerbs.qml` の `switch-mark` の枝)
- `move-ask`(引数はリモート ref(`origin/main`)、**`--preset diverged` が要る**。**ローカルが自分のコミットを持っているリモート行に降りる `Move <branch> here?` のバー**(書き込み側は `move-branch`)— 詳細は `AutoActSwitchVerbs.qml` の `moveAskTimer`、判定は `verify/verbs/switch.rs`)
- `ask-over-notice`(引数はリモート ref(既定 `origin/main`)、**`--preset diverged` が要る**。**質問バーと報せバーが同時に立った時、Esc がどちらのものか**。絵では答えられない — 詳細は `AutoActSwitchVerbs.qml` の `askOverNoticeTimer`、判定は `verify/verbs/switch.rs`)
- `switch-stopped` / `switch-stopped-go`(**引数は `<ブランチ>[:<stash の本数>]`(必須)**、バーの 2 形は登録表の行で**対で撮る**。**止まった操作の上で switch した時に降りる質問バー**と、それに答えた着地(3 形目は `switch-conflicted[-go]`)— 詳細は `AutoActSwitchVerbs.qml` の `switch-stopped` の枝)
- `switch-held <ブランチ> --preset worktrees`(**引数・preset とも必須**。**別の worktree が持っているブランチを押す** = このタブがその worktree に立つ — 詳細は `AutoActSwitchVerbs.qml` の `switch-held` の枝と `switchHeldTimer`)
- `conflict-sides`(引数は `conflicts:<パス>`、**`--preset conflict-typed` が本番**。conflict の diff で**行頭の帯がどちら側を名乗ったか**を数える — 詳細は `AutoActDiffVerbs.qml` の `stageRowTimer.endAtRows` の `conflict-sides`)
- `diff-tick`(引数は `diff-file` と同じ形、`--preset conflict-typed`。**開いている diff に対してページの tick(`RepoPage.pollDiff`)を 1 回撃つ** — 詳細は `AutoActDiffVerbs.qml` の `stageRowTimer.endAtRows` の `diff-tick`)
- `take-side-ours`
- `take-side-theirs`(引数はパス。**conflict のある preset が要る**(`--preset conflict-kinds`)。conflict 行のメニューから片側を採り、着地まで待って撮る — 詳細は `AutoActFileRowVerbs.qml` の `runFileRowAct`)
- `open-mergetool`(引数はパス、**`--preset conflict-kinds` が要る**。同じメニューから外部ツールへ渡す — ツールは run が建てる(`verify/repos.rs` の `seed_merge_tool`)。報告行 `merge_tool` の読み分けは `AutoActFileRowVerbs.qml` の `runFileRowAct`)
- `merge-branch`
- `merge-stops`(引数はブランチ名。**`--preset clashing` が要る**。同じ 1 押しの**もう 1 つの着地** = 選択は WIP 行へ行き、エラー帯もコマンドログの持ち上げも出ない(規約 §進行中の操作から出る)— 詳細は `AutoActHistoryVerbs.qml` の `mergeStoppedTimer`)

- `merge-commit`(引数なし、**`--preset conflict-staged` 必須**。止まった merge を、箱を空にしたまま commit ボタンで仕上げる — 詳細は `AutoActWipVerbs.qml`、判定は `verify/verbs/sequencer.rs`)
- `merge-standing`(引数なし、`--preset conflict-staged` か merge が止まった `--repo`。箱を空にしたまま止まり、後ろに立つ merge の文が placeholder に出る所 — 何も書かない。詳細は `AutoActWipVerbs.qml`)
- `drop-stops`(引数は `drop-commit-go` と同じ、**`--preset drop-stops` + `row:2` 必須**。stash 経由で回り直した replay が止まった着地 — 詳細は `AutoActHistoryVerbs.qml` の `opStoppedTimer`)
- `cherry-pick-stops`(**`feature/clash --preset clashing` 必須**。押した cherry-pick が conflict で止まった着地 — 詳細は同上)
- `revert-stops`(引数は `revert-commit` と同じ、**`--preset revert-clashes` + `row:1` 必須** — 詳細は同上)
- `rebase-onto`(引数はブランチ名)
- `replay-running`(引数はブランチ名、**`base --preset replay` 必須**。走行中の replay の画面 = `REBASING n/m` のバッジ・下りた扉・ポインタ脇のリング — 詳細は `AutoActHistoryVerbs.qml` の `replayRunningTimer`)
- `rebase-stops`(**`main --preset rebase-clashes` 必須**。押した rebase が止まった着地 — 詳細は `AutoActHistoryVerbs.qml` の `opStoppedTimer`)
- `revert-commit`(引数は**完全な oid** か **`row:<n>`**、省略で HEAD。先端に答えが立つ着地 — 視界の側を試す背の高い履歴の作り方と罠は `AutoActHistoryVerbs.qml` の revert の枝、完了は `tipLandedTimer`)
- `integrate-menu`(**引数はブランチ名、または `<タグ>:tag`**。`--preset basic` の `feature/topic-a` と `v0.2:tag` の **2 本で 1 組**。ref メニューを立てたまま `rebase` 行の push 済みの note と灰色の行を報告行で読む — 詳細は `AutoActHistoryVerbs.qml` の `openIntegrateMenu`、判定は `verify/verbs/sequencer.rs`)
- `squash`(引数は `drop-commit-go` と同じ形 — 完全な oid か `row:<n>`、省略で HEAD。`row:1 --preset squash-hooked --allow-write-failure` は `prepare-commit-msg` が断る側 — 詳細は `AutoActHistoryVerbs.qml`、判定は `verify/verbs/sequencer.rs`)
- `fold-first-commit` / `drop-last-commit`(**`--preset one-commit --allow-write-failure` 必須**。引数は `fold-first-commit` が `squash`、`drop-last-commit` が `drop-commit-go` と同じ。git に訊く前に行メニュー側で断られる 2 つ — 詳細は `verify/verbs/sequencer.rs` の行)
- `drop-commit`
- `drop-commit-go`(引数は完全な oid か `row:<n>`(**汚れた木では行 0 が WIP**)、省略で HEAD のもの。drop の行を出ている形(回数押し / クリック)で押し切る。両形は `--preset diverged` / `--preset basic`、`--preset drop-collides` は stash の回り道が戻す時に conflict した着地 — 詳細は `AutoActHistoryVerbs.qml` の `dropTimer`)
- `rebase-plan`(**`--preset plan` 必須**。HEAD の 3 行下から開いたプランに squash / drop を着せた、プラン編集モードの全景 — 詳細は `AutoActPlanVerbs.qml` の `planOpenTimer`、判定は `verify/verbs/sequencer.rs`)
- `plan-off-branch`(**`row:0 --preset one-commit` 必須**、`--allow-write-failure` は要らない。プランが開く前に preview が断る面の代表 1 本 — 詳細は `AutoActPlanVerbs.qml`、判定は `verify/verbs/sequencer.rs`)
- `plan-loading`(`--preset plan`。行が着く前のプラン面 — 詳細は `AutoActPlanVerbs.qml` の `planLoadingTimer`)
- `plan-fold-carry`(`--preset plan`。プランの並べ替えを手の形で往復させ、通過で畳みが降格しないこと — 詳細は `AutoActPlanVerbs.qml` の `planOpenTimer`)
- `rebase-plan-run`(`--preset plan`。HEAD の 2 行下から開き、最古行を `drop` にして `runPlan` まで完走 — 詳細は `AutoActPlanVerbs.qml` の `planRanTimer`)
- `rebase-edit-stop`(`--preset plan`。HEAD の 1 行下から開き、最古行を `edit` にして実行 = **木がきれいなまま止まる停止**の着地 — 詳細は `AutoActPlanVerbs.qml` の `planEditStopTimer`)
- `plan-reword-verb`(`--preset plan`。**プランが立ったまま**、行 0 の verb を `reword` → `pick` → `reword` と往復させる — 詳細は `AutoActPlanVerbs.qml` の `planRewordTimer`)
- `plan-reword-out`(`--preset plan`。行 0 を `reword` にして打鍵したまま `Discard` した後の枠 — 詳細は同上)
- `plan-reword-ask`(`--preset plan`。**`plan-reword-out` の続き** — 閉じた後にもう一度打鍵する。詳細は同上)
- `plan-details-held`(`--preset plan`、引数なし。**プラン中の右ペインに残った扉**(親ハッシュ・基準のハッシュ・ファイル行)と `Discard` を順に押す — 詳細は `AutoActPlanVerbs.qml` の `planHeldTimer`)
- `publish-dismiss`(引数なし、`--preset unpublished`。**質問バーを `✕` で取り下げた後、上がり切るまでに何を着ているか** — 詳細は `AutoActPublishVerbs.qml` の `publishDismissTimer`)
- `publish-upstream`(引数は向こうでの名前・既定 `brand-new`、preset は既定(`basic`)で足りる。**追跡 ref がまだ無い upstream を持つブランチで push を押した形**(規約 §はじめてリモートへ送る・質問が開く場所は `PublishFlow.startPublishAsk`)— 詳細は `AutoActPublishVerbs.qml` の `publishUpstreamTimer`)
- `publish` / `publish-taken` / `publish-remotes` / `publish-add` / `publish-go` / `publish-enter`(`publish-enter` は `publish-go` と同じ答えを**名前の欄の Enter で出す形**(規約 §立っている質問は 1 か所で聞く)。回数押しになる形では使えない)
- `publish-new-go`(**引数 `<name>|<url>` 必須**。publish 系は同じ first-push 経路を別の所で止める: `publish` = 照会済みの質問(remote 0 本なら Add remote dialog)/ `publish-taken` = branch `taken`(引数で上書き可)/ `publish-remotes` = 行き先 popup(overlay.png)/ `publish-add` = Add remote dialog(引数 `<name>|<url>`、空欄可。overlay.png)/ `publish-go` = 実際の click / hold で答えて write barrier まで / `publish-new-go` = dialog で新 remote を足して照会に答え write barrier まで。土台は `--preset unpublished`(`taken` / `carried` / `outsider` の 3 形)と `--preset noremote` — 詳細は `AutoActPublishVerbs.qml` の各 timer、形ごとの判定は `verify/verbs/remote.rs`)
- `publish-tip`(引数・preset は `publish-taken` と同じ。その質問の pill に手を置いた時のツールチップ = 上書きで落とす数を言う唯一の場所(overlay.png)— 詳細は `AutoActPublishVerbs.qml` の `publishTipTimer`)
- `replace-remote`(引数 `<remote>/<old>:<new>`。**質問で止まる** — 書き込みは `-go` だけ。詳細は `AutoActNavBoxVerbs.qml` の `replaceAskTimer`、判定は `verify/verbs/switch.rs`)
- `replace-remote-go`(回数押しを最後まで進めて実行。**向こうが立っているブランチは渡せない**(`origin/feature/topic-a` のような別の行で撃つ)— 詳細は `AutoActNavBoxVerbs.qml`)
- `replace-remote-tip`(引数は `replace-remote` と同じ。同じ質問の pill に手を置いた時のツールチップ = 回数押しの理由と走る順(overlay.png)— 詳細は `AutoActNavBoxVerbs.qml` の `replaceAskTimer`)
- `rename-local-upstream` / `rename-local-upstream-go`(引数 `<新しい名前>[:replace|add|leave]`。今立っているブランチを改名した後に降りる、リモート側をどうするかの質問。答えを省くとバーは既定の `add` のまま、`-go` は `replace` を撃てない — 詳細は `AutoActSwitchVerbs.qml`、判定は `verify/verbs/switch.rs`)
- `rename-local-upstream-tip`(引数は新しい名前だけ、**答えは `replace` に固定**。その pill のツールチップ(overlay.png)— 詳細は同上)
- `rename-tag-remote` / `rename-tag-remote-go`(引数 `<tag>:<新しい名前>[:replace|add|leave]`、preset `basic` なら `v0.2:v0.2.1`。両側が持つタグを改名した後の同じ質問 — こちらは `replace` も撃てる(`-go` の既定)。詳細は `AutoActNavBoxVerbs.qml` の `tagRemoteRenameTimer`、判定は `verify/verbs/nav.rs`)
- `rename-tag-remote-tip`(引数 `<tag>:<新しい名前>`、**答えは `replace` に固定**。その pill のツールチップ(overlay.png)— 詳細は同上)
- `set-upstream` / `set-upstream-list` / `set-upstream-enter`
- `set-upstream-go`(**ブランチが何に対して測られるかの質問**(規約 §ブランチが測られる相手を決める)を BRANCH カードの行から開く。引数は `<branch>[:<答えるリモートブランチ名>]`、`:missing` = 手元に無い名前、**末尾の `:` だけ = 名前の欄を空にした形** — 4 動詞の違いと形ごとの preset は `AutoActSwitchVerbs.qml` の `upstreamAskTimer`、判定は `verify/verbs/remote.rs`)
- `unset-upstream` / `unset-upstream-go`(引数 `[chip:]<branch>`。**BRANCH カードの upstream を外す行**(規約 §ブランチが測られる相手を決める)を読む / 押し切る — 詳細は `AutoActSwitchVerbs.qml` の `unsetUpstreamTimer`、枝ごとの preset と判定は `verify/verbs/remote.rs`)
- `delete-remote`
- `delete-remote-go`(引数 `<remote>/<branch>`。**畳みが開いてメニューが立ったまま止まり**、-go が行の回数押しを完走させる)
- `delete-both-go` / `delete-upstream-go`(引数はローカルブランチ名、**`--preset stack` の `side` / `next`**。削除の表の `Delete both` / 読みへの `push --delete` を回数押しで完走させる — 詳細は `AutoActRefVerbs.qml`)
- `pull-menu`(**pull の行が立つ席と、立たない席**(規約 §取り込んで合流させる)。引数 4 通り: 無し / `origin/main` / 今立っていないローカルブランチ(`release --preset others-behind`)/ 他のリモート ref = 出ない側。**組で読む** — 詳細は `AutoActRefVerbs.qml` の `pullMenuTimer`、判定は `verify/verbs/remote.rs`)
- `pull-blocked`(**押せば断られると押す前に分かる木で、行が無効になっている姿と hover の文**(規約 §取り込んで合流させる)。引数 `[chip][:<branch>]` — 形ごとの preset と文は `verify/verbs/remote.rs`、詳細は `AutoActRefVerbs.qml` の `pullBlockedTimer`)
- `pull-go` / `pull-ahead`(`pull-go --preset behind` / `pull-ahead --preset basic`。**行を実際に押して**、git が取り込む / `Already up to date.` を返す 2 通りを撮る。割れた木は `pull-blocked` の側 — 詳細は `AutoActHistoryVerbs.qml` の `pullPressTimer`)
- `pull-branch-go`(引数はブランチ名、**`release --preset others-behind`**。今立っていないブランチの行を押す = 早送り — 詳細は `AutoActHistoryVerbs.qml` の `pullBranchTimer`)
- `push-branch-menu` / `push-branch-ask` / `push-branch-go`(**今立っていないブランチのメニューの、`pull` の隣の `push` 行**(規約 §リモートへ送る)。引数 `[chip:]<ブランチ>`、土台は **`--preset others-behind`**(他に `main --preset localbase-dotmark` / `feature/login --preset github`)。行を読む / `scratch` の行で質問に止める / 押し切る — 詳細は `AutoActBranchPushVerbs.qml`、名前ごとの判定は `verify/verbs/remote.rs`)
- **報せバーの 10 本**(= `write_notice` を報告する動詞の全部。うち自分の行を持たない 3 本 = `fold-first-commit` / `drop-last-commit` / `plan-off-branch`、残り 7 本は下の子項目。規約 §答えの要らない報せ)。`--allow-write-failure` が要るのは 8 本 —— **`plan-off-branch` と `report-tone` は要らない**。完了は共通で `AutoActDriver.barrierNotice`、報告行も共通の `write_notice …` — 読み方は `AutoActDriver.qml` の `noticeLine`。
  - **git の run が断る preset の上の素の動詞**(書き込み境界がバーを待ち、同じ `write_notice` 行を足す。どれも `--allow-write-failure` が要る — 何が断るかと判定は `verify/verbs/*.rs` の `WithPreset` の行): `push --preset prepush` / `toolarge` / `checkedout` / `remote-lock`、`force-push --preset nonff`、`drop-commit-go row:1 --preset prerebase` / `plan-unsigned` / `plan-lock`、`drop-commit-go row:2 --preset plan-untracked`、`rename-stash <名前> --preset stash-lock` / `stash-droplock`、`commit-refused --preset unsigned` / `hooked-auto`、`stage-line b.txt --preset stage-lock`、`squash row:1 --preset squash-hooked`、`carried-apply-refused topic --preset carried-lock`、`publish --preset publish-unreachable`、`worktree-add branch:feature/topic-a --preset postcheckout`(作られたが `post-checkout` が失敗した側)。`push-tag v1 --preset tag-hidden` は報せにならず判定は絵。**最低版の git では別の答えになる形は段分け表の `host` 行**(host だけが回す): `push --preset prepush-config`(2.54+)、`rename-taken held --preset reftable-lock` / `rename-stash renamed --preset reftable-droplock`(2.48+)— files 側のロックの形は `rename-taken held --preset reflock` / `rename-stash renamed --preset stash-lock` / `stash-droplock`
  - `remote-refused origin/main --preset protected`(**向こうがブランチを keep**。`delete-remote-go` と同じ入口・同じ回数押しを、押しても消せないリモートへ撃つ — 詳細は `AutoActRefVerbs.qml`、判定は `verify/verbs/remote.rs`)
  - `tag-refused v1.0:remote --preset protected`(**同じ拒否をタグで**。`delete-remote-tag` と同じ行を押す — 詳細は `AutoActRefVerbs.qml` の `tagMenuTimer`)
  - **lease の拒否(こちらが古くて消せなかった)は preset に無い** —— `demo-repo stack` を作り、origin を `git clone --bare` した 2 つ目の bare で `side` / `next` / タグの先端を進め、`remote set-url --push origin <その bare>` で push 先だけ差し替えた clone を `--repo` で渡す(開く時の fetch は古い先の origin を読むので、画面の先端と push 先が必ず食い違う)。`delete-both-go side` / `remote-refused origin/next` / `delete-remote-tag v6.3:remote` / `delete-tag-both v6.2:remote` を `--allow-write-failure` で(must_say は各動詞の成功側の行なので FAIL と出る。判定は絵)
  - `push-outdated --preset outrun`(**こちらが古くて送れなかった**。ツールバーの素の `push` を押す — 詳細は `AutoActPublishVerbs.qml`、`outrun` の仕込みは `verify::seed`)
  - `commit-refused --preset hooked`(**手元の `pre-commit` が断った**。`commit` と同じ press。`--preset unsigned` は署名できない側 — 詳細は `AutoActWipVerbs.qml`、判定は `verify/verbs/panes.rs`)
  - `stale-part unstaged:b.txt --preset dirty --allow-write-failure`(**こちら自身が断つ側**。git を 1 本も撃たない — 詳細は `AutoActWipVerbs.qml` の `stalePartTimer`)
  - `report-tone half-rename`(**door の側だけ**を見る。引数は `half-rename` 1 つ — 詳細は `AutoActWipVerbs.qml`、判定は `verify/verbs/panes.rs`)
  - `notice-link`(**報せの 2 行目の行き先を押す**。パネルが立って名乗るのを待つ。`write_notice` は報告しないので報せバーの 10 本には数えない — 詳細は `AutoActWipVerbs.qml` の `noticeLinkTimer`)
  - `notice-over-diff src/app.txt --preset hooked`(**同じ拒否を、中央に diff を開いたまま**。`clears=true` が主張 — 詳細は `AutoActWipVerbs.qml`)

- `graph-divider`(引数なし / `pin`(`--preset deep`)/ `pin-lit`(**`--preset deep-detached` 必須**)。グラフ列の仕切りにポインタを置き、選択と hover の地の上に線が描き直されるかを読む(`pin` は HEAD の張り付き行の上)。上限に当たったドラッグの輪は `divider-refuse graph-max` — 詳細は `AutoActPaneVerbs.qml` の `graphDividerTimer`)
- `divider-refuse`(引数は場所と向き: `graph-max` / `graph-min` / `label-min` / `label-max` / `sidebar-min` / `details-min` / `log-min`(`--preset basic`)/ `desc-max` / `desc-min`(`--preset long`)。窓の中の掴める境界 9 通りの「これ以上無い」 — 詳細は `AutoActDetailsVerbs.qml`)
- `graph-min --preset co-authors`(グラフ列を床まで縮め、止まった幅を読む — 詳細は `AutoActPaneVerbs.qml`、判定の数はトークン算術 = `verify/verbs/graph.rs` の行)
- `graph-step`(引数は符号付きの歩数、`--preset basic`。上下矢印で履歴を辿り、視界の落ち方と右の追いつきを読む(規約 §矢印で履歴を辿る)— 詳細は `AutoActStepVerbs.qml` の `graphStepTimer`)
- `graph-step-edge` / `graph-step-far`(`-far` は `graph-step-far 10 --preset spread`。視界が動く 2 つ = 端へ寄せる / 真ん中へ — 詳細は同上)
- `graph-step-diff`(引数は選択中のコミットが触った、行の在るパス(`basic` の HEAD は `docs/guide.md`)。diff を開いたまま 1 歩 = 矢印が裏のグラフを動かさない — 詳細は同上)
- `graph-step-named` / `graph-step-dirty`(**対で読む**。名前箱を開いた行は矢印を断り、書きかけのメッセージは断らない — 詳細は同上)
- `graph-step-hold`(引数は総歩数(既定 6)。矢印を押しっぱなしにした時に詳細を何回読むか — 詳細は `AutoActStepVerbs.qml` の `graphHoldTimer`)
- `wip-shut` / `changes-shut`(**対で読む**、`--preset dirty`(`unstaged:b.txt` / `renamed-from.txt`)。同じ行の 2 度目のクリックで diff を閉じた後、2 つの一覧がどちらも何も光らせない — 詳細は `AutoActStepVerbs.qml` の `fileStepTimer`)
- `changes-step` / `changes-step-edge` / `wip-step`(引数は最初に押すファイル — `changes-*` はパス(`--preset long`)、`wip-step` は `<bucket>:<パス>`(既定 unstaged、`--preset dirty`)。ファイル一覧の上下矢印 = 1 打 1 ファイル(`FileRowWalk.qml`)— 詳細は同上)
- `changes-fold` / `changes-unfold`(**対で読む**。引数はフォルダ行の鍵(既定 `assets/icons`、`--preset long`)。コミットの CHANGES ツリーのフォルダ行を畳む / 叩き直して開く — 詳細は `AutoActStepVerbs.qml` の `changesFoldTimer`)
- `diff-step` / `diff-step-edge`(引数はパス、`untracked.txt --preset dirty`。diff 自身の上下矢印 = 1 打 1 行で視界が動く(`DiffRowWalk.qml`)— 詳細は `AutoActStepVerbs.qml` の `diffStepTimer`)
- `graph-tail --preset deep`(窓の切り口 = 積んだぶんより歴史が長い時にグラフの末尾へ出る footer — 詳細は `AutoActPaneVerbs.qml` の `graphTailTimer`)
- `graph-tail-more --preset deep`(その footer を押して次の一歩を積み、読んでいる行の下に増える — 詳細は `AutoActPaneVerbs.qml` の `tailMoreTimer`)
- `wip-lanes --preset conflict`(未コミット行が下ろしているレーンを描画トークンで報告する。絵では答えられない — 詳細は `AutoActWipVerbs.qml` の `wipLanesTimer`)
- `graph-head` / `graph-head-below` / `graph-head-back` / `graph-head-go` / `graph-head-lit`(**5 つで 1 組**。`graph-head` / `-back` は `--preset deep`、`-below` / `-go` / `-lit` は **`--preset deep-detached` 必須**。HEAD の行が画面外に居る間だけ縁に立つ張り付き行(`GraphHeadPin.qml`)— 詳細は `AutoActPaneVerbs.qml` の `graphHeadTimer`)
- `graph-bar` / `graph-bar-away`(**対で読む**。レーンの横スクロールバーの出入りと明るさ — 詳細は `AutoActPaneVerbs.qml` の `graphPanTimer`)
- `pane-bar` / `pane-bar-away`(**対で読む**、`--preset long`。右パネルの板バー(`PaneScrollBar`)の明暗 2 段 — 詳細は `AutoActPaneVerbs.qml` の `paneBarTimer`)
- `text-bar` / `text-bar-away`(**対で読む**、`7 --preset edges`。description 枠の中のバーの明暗 — 詳細は `AutoActPaneVerbs.qml` の `textBarTimer`)
- `bar-track --preset long`(縦バーの溝の押下と長押し(`AutoScrollBar.pressTrack`)— 詳細は `AutoActPaneVerbs.qml` の `barTrackTimer`)
- `bar-arrow --preset long`(縦バーの両端の矢印のクリックと長押し — 詳細は `AutoActPaneVerbs.qml` の `barArrowTimer`)
- `middle-scroll`(引数なし = レーン列 / `message` = subject 列、`--preset stashes`。中クリックで横へ流れるのはレーン列だけ — 詳細は `AutoActPaneVerbs.qml` の `graphPanTimer`)
- `middle-scroll-exit`(引数 `held`(既定)/ `click`、**`--preset deep`**。中クリックの身振りの終わり方 2 つ(`MiddleAutoScroll.qml`)— 詳細は `AutoActPaneVerbs.qml` の `middleExitTimer`)
- `middle-hand <面>`(面は `files` / `block` / `description` / `summary` / `chosen` / `plan` / `wip-files` / `wip-block` / `wip-description` / `wip-summary` / `tags` / `peek` / `ref-list` / `commands` / `carried`、preset は面ごと(登録表の行)。スクロールする面の中クリックの手、1 run に 1 面(規約 §中クリックの自動スクロール)— 詳細は `AutoActHandVerbs.qml`)
- `middle-close`(引数はタブの index(既定 0)。**タブが 2 つ以上要る** = `--preset basic --preset stashes`。タブを中クリックで閉じる — 詳細は `WindowTabActs.qml` の `middleCloseTimer`)

### 一覧(続き)

**段落側に仕込みがある動詞は、引数・preset を §動詞ごとの読み方・仕込み で動詞名 Grep** して読む(報告行の読み方と罠は各行末のドライバのコメント):

- `tab-widths`(帯の run の配り方。引数は**到達する状態**(`ample` 既定 / `short`)。must_say — 詳細は `WindowTabActs.qml` の `tabWidthActTimer`)
- `tab-mark`(タブの hover と `✕` の出方。引数は手を置くタブ — 詳細は `WindowTabActs.qml` の `tabMarkActTimer`)
- `tab-name`(同名リポジトリが並んだ時のタブ名と、hover のフルパス。引数はどのタブを指すか。must_say — 詳細は `WindowTabActs.qml` の `tabNameActTimer`)
- `tab-drag`(タブを掴んで並べ替えた後の順。引数 `<from>:<to>` — 詳細は `WindowTabActs.qml` の `tabDragTimer`)
- `tab-hold`(掴んだまま = 席から外れて描かれているタブ。引数はどのタブか — 詳細は `WindowTabActs.qml` の `tabHoldTimer`)
- `tab-edge`(掴んだタブを帯の端へ押し当てて、列の方を流す。引数はどのタブか — 詳細は `WindowTabActs.qml` の `tabEdgeTimer`)
- `tab-pin`(今居るタブが run の外へ出ている間、縁に立つ張り付きタブ。引数はどのタブを前に出すか。must_say — 詳細は `WindowTabPinActs.qml` の `tabPinTimer`)
- `tab-pin-go`(その張り付きタブを押して、帯を本物の席まで流す。must_say — 詳細は `WindowTabPinActs.qml` の `tabPinGoTimer`)
- `tab-open-go`(同じ staging に 13 個目のリポジトリを開いて、帯がその席まで流れる。引数は開くリポジトリ = 省略で xtask が建てる。must_say — 詳細は `WindowTabPinActs.qml` の `tabOpenGoTimer`)
- `tab-open-moved-on`(`tab-open-go` の到着の後に窓のどこかを押し、窓を広げてから床へ戻す = 押下で頼みが終わり、帯は動かず代役が立つ。must_say — 詳細は `WindowTabPinActs.qml` の `tabMovedOnTimer`)
- `carried-open --preset carried`(**preset 必須**。別の worktree の未コミット行をダブルクリックして、このタブをその worktree に立たせる = タブは増えない(規約 §タブの所作)— 詳細は `WindowTabActs.qml` の `carriedOpenTimer`)
- `worktree-stand [行番号] --preset worktrees`(WORKTREES の行をダブルクリックして、このタブをその worktree に立たせる。引数はセクションの行番号(既定 `1`)— 詳細は `WindowTabActs.qml` の `worktreeStandTimer`)
- `worktree-draft --preset worktrees`(コミットメッセージを書いてから別の worktree へ立ち、戻って来る = 立ち位置ごとに憶える下書きの往復 — 詳細は `WindowTabActs.qml` の `worktreeDraftTimer`)
- `worktree-rename-stand <folder>=<名前> --preset worktrees`(このタブが立っている worktree のフォルダを改名し、タブが新しいフォルダへ立ち直る(規約 §タブの所作)。絵ではどれも読めない — 詳細は `WindowTabActs.qml` の `worktreeFollowTimer`)
- `worktree-kept [away] --preset basic --preset worktrees`(**preset 2 つ必須**。立ち位置を移したタブを離れて(`away`)/ 戻る(省略)(規約 §タブの所作「立っている場所はタブ自身も言う」)— 詳細は `WindowTabActs.qml` の `worktreeKeptTimer`)
- `worktree-tip [行番号] --preset worktrees`(立ち位置を移したタブに手を置いて、hover が立っている worktree のパスを言うか(規約 §hover のツールチップ)— 詳細は `WindowTabActs.qml` の `worktreeTipTimer`)
- `carried-read <worktree 名> --preset carried`(**preset・引数必須**(`topic` / `here` / `an-extremely-long-worktree-folder-name-for-the-edge-case`)。別の worktree の未コミット行を選び、`WORKTREE` モードで読む — 詳細は `AutoActWipVerbs.qml` の `carriedReadTimer`、判定は `verify/verbs/panes.rs`)
- `carried-stand <worktree 名> --preset carried`(同じペインを、まだ何も読んでいない状態で。preset・引数は `carried-read` と同じ — 詳細は `AutoActWipVerbs.qml` の `carriedStandTimer`)
- `carried-menu <worktree 名> --preset carried`(別の worktree の未コミット行の右クリック(規約 §別の worktree を読む)。preset・引数は `carried-read` と同じ、`here --preset carried-conflicted` は灰の形。絵は scene.png — 詳細は `AutoActWipVerbs.qml` の `carriedMenuTimer`)
- `carried-apply <worktree 名> --preset carried` / `carried-apply-refused <worktree 名> --preset carried --allow-write-failure`(そのメニューの `Apply changes to this worktree` を押す / git が断る形 — 詳細は同上)
- `wip-landing --preset carried`(**既定の着地が乗るのはこの窓の作業ツリー行を持つ pass だけ**。preset 必須。must_say — 詳細は `WindowBadgeActs.qml` の `wipLandingTimer`)
- `wip-landing-stopped --preset carried-clashing`(同じ主張を、止まった操作が負う着地で。preset 必須。must_say — 詳細は `AutoActHistoryVerbs.qml`)
- `tab-carry`(タブ切替が何を連れて行き何を捨てるか。must_say — 詳細は `WindowTabActs.qml` の `tabCarryTimer`)
- `corner <ペイン>`(右下の git バージョン表示の譲り方。引数必須。報告行 `git_corner` — 詳細は `AutoActDetailsVerbs.qml` の `cornerTimer`)
- `open-again`(開いているリポジトリをもう一度開く — そのまま既存タブへ。詳細は `WindowTabActs.qml` の `openAgainTimer`)
- `identity`(identity 未設定の質問ダイアログ — 詳細は `WindowDialogActs.qml` の `identityTimer`)
- `identity-half`(半分だけ保存された identity から続ける — 詳細は同上)
- `identity-tip`(半端 save の後の帯の状態カード。overlay.png 側。must_say — 詳細は `WindowDialogActs.qml` の `identityTipTimer`)
- `solo`(2 個目の起動が断られる報告。must_say `solo blocked=true` — 詳細は `WindowTabActs.qml` の `soloActTimer`)
- `band`(タブ行がタイトルバーを兼ねる帯 — 数字で読む。報告行 `band`。チップや行を見せたい時は、その行を待つ動詞(`nav-jump` / `ref-list` / `row-card`)で撮る — 詳細は `WindowBandActs.qml` の `bandActTimer`)
- `app-menu` / `app-menu-reclick`(☰ を 1 回押して立てたまま / 2 回押して閉じる。**対で読む**。must_say — 詳細は `WindowBandActs.qml` の `appMenuActTimer`)
- `ops-panel [<幅>]`(**操作パネルが立った姿** = 状態の表を撮る動詞。preset は登録表の行(`panel` / `longnames` / `worktrees`)。must_say と絵の主張 `ops_panel` — 詳細は `WindowBandActs.qml` の `opsPanelTimer`)
- `ops-stand` / `ops-stand-repos` / `ops-stand-worktrees [<絞り込み>]`(リポジトリ名のカードとその段。`repos` は**タブが 2 枚要る**(`--preset panel --preset stashes`)— 詳細は `WindowBandActs.qml` の `opsDoorTimer`)
- `ops-branch` / `ops-branch-folder <フォルダ>`(ブランチ名のカード = 左メニューの BRANCHES と同じ木。後者はそのフォルダのカードまで開く。**`--preset panel`** — 詳細は同上)
- `ops-branch-pick <ブランチ>` / `ops-worktree-pick <フォルダ名>` / `ops-repo-pick`(**カードの行を押した先**。`ops-branch-pick rig --preset panel` / `ops-worktree-pick a --preset panel` / `ops-repo-pick --preset panel --preset stashes` — 詳細は `WindowOpsActs.qml`)
- `window-fill`(最大化が四隅まで届く報告。must_say `fills=true` — 詳細は `WindowFrameActs.qml` の `fillActTimer`)
- `window-floor`(窓の床 = これ以上小さくできない大きさ。must_say — 詳細は `WindowFrameActs.qml` の `floorActTimer`)
- `details-fit`(詳細ペインの列が収まる報告。must_say `details_fit fills=true fits=true` — 詳細は `AutoActDetailsVerbs.qml` の `detailsFitTimer`)
- `details-parents <行>[:<幅>|:min]`(作者行の親の行が部屋に収まる形(`HashPlate.qml`)。**`--preset octopus`**(行 0 = 8 親・行 8 = 2 親)と **`--preset stashes`**(行 0)— 詳細は `AutoActDetailsVerbs.qml` の `detailsParentsTimer`、判定は `verify/verbs/panes.rs`)
- `details-align <行>[:<幅>|:min]`(作者行の 2 行がそれぞれ 1 本のベースラインに載ること。登録表の行は `0` / `16:min --preset octopus-mates` と `7 --preset edges` — 詳細は `AutoActDetailsVerbs.qml` の `detailsAlignTimer`)
- `details-parents-card <行>[:<幅>|:min]` / `details-parents-go <行>[:<幅>|:min]`(`+N` に手を置いて開く親の一覧カード / その 2 行目を押す。**`+N` が立つ形でないと開かない**(`--preset octopus` 行 0)— 詳細は `AutoActDetailsVerbs.qml` の `parentsCardTimer`)
- `badges <幅>[:<タブ本数>]`(帯の状態が 3 つ揃った形。**止まった操作と conflict を同時に持つ preset が要る**(`conflict` / `cherry-pick-conflict`)。回すのは引数なし / `1400:6` / `1000:6` / `floor:16` の 4 形 — 詳細は `WindowBadgeActs.qml` の `badgesActTimer`、判定は `verify/verbs/window.rs`)

- `badges-hover <幅>[:<タブ本数>]`(引数は `badges` と同じ、`--preset conflict`。その群を hover で開いたカード — 詳細は `WindowBadgeActs.qml` の `badgesActTimer`、判定は `verify/verbs/window.rs`)
- `badges-hover-early <幅>[:<タブ本数>]`(同じカードを、帯が群を置く前にポインタを降ろして開く形。引数・preset は `badges-hover` と同じ — 詳細は `WindowBadgeActs.qml` の `Component.onCompleted`、判定は同上)
- `graph-stale`(引数なし・preset は何でもよい。帯の `STALE GRAPH` = 行は全部立っているのに全部古い形を、カードを開いた姿まで — 詳細は `WindowBadgeActs.qml` の `staleActTimer`、判定は `verify/verbs/window.rs`)
- `graph-stopped`(引数なし。同じバッジのもう半分 = walk が途中で止まって行が足りない形 — 詳細は同上)
- `band-actions-none`(`--preset longnames`。操作パネルの 4 つのボタンが 1 つも語を譲っていない形 — `band-actions*` の 5 動詞の詳細は `WindowBandActs.qml` の `actionsActTimer`、判定は `verify/verbs/window.rs`)
- `band-actions [<幅>]`(`--preset longnames`。語を残したまま最も狭いセル — 詳細は同上)
- `band-actions-fold`(`--preset longnames`。語を捨てて印だけになった形 — 詳細は同上)
- `band-actions-alert`(**`--preset longnames` + `--allow-write-failure` が要る**。畳んだ印の角に push の `!` が立った形 — 詳細は同上)
- `band-actions-stopped`(**`--preset unreachable` + `--allow-write-failure` が要る**。同じ角の `!` を fetch 側で = 止まった fetch の印 — 詳細は同上)
- `old-git`(最低バージョンを割った git で普通に動いている窓 = 帯の 4 つ目のバッジ — 詳細は `WindowBadgeActs.qml` の `oldGitActTimer`、判定は `verify/verbs/window.rs`)
- `old-git-card`(そのバッジを hover で開いたカード — 詳細は同上)
- `old-git-fold`(黄だけが立った窓を床まで縮めた `…` — 詳細は同上)
- `no-lfs [<幅>|floor]`(**`--preset lfs` が要る**。Git LFS が要るファイルが在り、git が LFS を走らせられない窓 = 帯の `NO LFS`。`floor` は黄の `…` に畳んだ姿 — 詳細は `WindowBadgeActs.qml` の `noLfsActTimer`、判定は `verify/verbs/window.rs`)
- `no-lfs-card [<幅>|floor]`(**`--preset lfs`**。そのバッジを hover で開いたカード — 詳細は同上)
- `badges-all [<幅>[:<タブ本数>]]` / `badges-all-hover`(**`--preset lfs-conflict` が要る**。引数は `badges` と同じ。帯の 6 つが全部立った形とそのカード — 詳細は `WindowBadgeActs.qml` の `badgesActTimer`、判定は `verify/verbs/window.rs`)
- `state`(設定・画面状態の永続化。`--config-dir` を渡した 2 回目に `--restore` — 詳細は `WindowFrameActs.qml` の `stateActTimer`)
- `settings-git`(設定画面の `Git` の分類 — `GLOBAL` の IDENTITY / MERGE EDITOR の 2 章と、その下の `REPOSITORY OVERRIDE` 群。引数なし。`Application` の側は `settings <分>`)
- `rename-taken <名前>`(**`--preset reflock` + `--allow-write-failure` が要る**。箱が知り得ない理由で git が断る名前を本当に submit した時の箱と報せバー。撃ち方: `rename-taken held --preset reflock --allow-write-failure` — 詳細は `AutoActNavBoxVerbs.qml` の `rename-taken` の枝と `renameTakenTimer`、判定は `verify/verbs/nav.rs`)
- `switch-conflicted-go <ブランチ>[:<stash の本数>]`(**`--preset cherry-pick-quit` が要る**。unmerged な index の上で switch した時の質問バーに答えた着地。**対の `switch-conflicted` と組で撃つ**(`switch-conflicted feature/clash` と `switch-conflicted-go feature/clash:1`)。`switch-conflicted main` は今のブランチを押す形で、質問は立たない(`pressed=false bar=false`)— 詳細は `AutoActSwitchVerbs.qml` の `switch-stopped` の枝、組で読む理由は `verify/verbs/switch.rs`)
- `settings-eol true|input|false|inherited`(**引数は必須**、タブは 1 本でよい。`REPOSITORY OVERRIDE` の改行コードの章を選ぶ — 詳細は `WindowSettingsActs.qml` の `endingsTimer`、判定は `verify/verbs/identity.rs`)
- `settings-switch`(引数なし。左の分類の列を押して `Application` → `Git` へ移る唯一の経路 — 詳細は `WindowSettingsActs.qml` の `categorySwitchTimer`、判定は `verify/verbs/identity.rs`)
- `settings-escape [app|git]`(設定画面の出口 — `✕` と Escape が乗っている `SettingsDialog.escapeOut` を通す。引数は分類(省略で `app`)— 詳細は `WindowSettingsActs.qml` の `escapeTimer`、判定は `verify/verbs/identity.rs`)
- `settings-leave`(引数なし。保存していない identity を置いて出ようとし、画面が留まる形 — 詳細は `WindowSettingsActs.qml` の `leaveTimer`、判定は同上)
- `settings-processes [<数>]`(`Application` の `GIT PROCESSES` の章の欄を打つ。本番は `3` と引数なし(既定)— 詳細は `WindowSettingsActs.qml` の `processesTimer`、判定は `verify/verbs/identity.rs`)
- `settings-refresh [<床>:<天井>:<他の worktree の読み方>:<1 つ目>:<2 つ目>]`(`Application` の `REFRESH` の章を動かす。読み方は `auto` / `fixed` / `off` — 詳細は `WindowSettingsActs.qml` の `refreshTimer`、引数ごとの判定は `verify/verbs/identity.rs`)
- `settings-refresh-leave [<床>:<天井>:<他の worktree の床>:<他の worktree の天井>]`(`REFRESH` の章の欄を確定しないまま設定画面を出る。空の部分は空にした欄 — 詳細は `WindowSettingsActs.qml` の `refreshLeaveTimer`、判定は同上)
- `settings-git-path [<絶対パス>|stored-missing|in-use|other]`(`Application` の `GIT EXECUTABLE` 章 — 撃つ git のパスと、その先が答えた版。`other` は `--other-git <版>` と組、古い版の絵は `--old-git` で持ち込む — 詳細(キーワード・存在しないパスの綴り方)は `WindowSettingsActs.qml` の `gitPathTimer`、判定は `verify/verbs/identity.rs`)
- `settings-git-leave`(引数なし。差し出している git を置いて設定画面を出ようとする — 詳細は `WindowSettingsActs.qml` の `gitLeaveTimer`、判定は同上)
- `settings-tools` / `settings-tools-loading`(設定画面の `Git` の Merge editor 候補一覧を開いたまま。後者は loading の edge を latch する側 — 詳細は `WindowSettingsActs.qml` の `toolsTimer`)
- `settings-hand chapters|tools`(設定画面の 2 面の中クリックの手 = `middle-hand` の設定画面の側。`tools` は **`--preset mergetools` 必須** — 詳細は `WindowSettingsActs.qml` の `handTimer`、判定は `verify/verbs/panes.rs`)
- `settings-tools-enter`(その欄の Enter を一覧の両側で 1 run に撃つ(規約 §選ぶ欄と打つ欄)— 詳細は `WindowSettingsActs.qml` の `toolEnterTimer`、判定は `verify/verbs/identity.rs`)
- `settings-repo [<タブ番号>]` / `settings-repo-pick [<タブ番号>]`(**`--preset basic --preset stashes` でタブ 2 本**。設定画面の `Git` の分類 = この画面で一番背の高い姿。引数は帯の何番目のタブを選ぶか(省略で前に居るタブ)、`-pick` は更に一覧を下ろす — 詳細は `WindowSettingsActs.qml` の `repoSettingsTimer`、判定は `verify/verbs/identity.rs`)
- `file-menu` / `file-menu-untracked` / `file-menu-staged` / `file-menu-conflict`(ファイル行の右クリック、バケツ別。引数はパス(`--preset dirty`、conflict は `both.txt --preset conflict-kinds`)— 詳細は `AutoActFileRowVerbs.qml` の `file-menu` の枝と `menuLine`、判定は `verify/verbs/panes.rs`)
- `amend-author`(**`--preset authorship`**。amend の著者引き継ぎ表示 — 詳細は `AutoActWipVerbs.qml` の `amendAuthorTimer`、判定は `verify/verbs/panes.rs`)
- `stash-menu`(stash 行の右クリック)
- `name-box <行番号>[:<チップ列の幅>]`(グラフ行の名前入力欄を開いたまま止める。幅 `0` は列の床 — 詳細は `AutoActPaneVerbs.qml` の `name-box` の枝)
- `menu-hover [<行番号>]`(コミット行メニューを開いてから、その行の hover カードを頼む。**`row-card` と対で読む** — 詳細は `AutoActRowMenuVerbs.qml` の `menuHoverTimer`、判定は `verify/verbs/graph.rs`)
- `menu-list behind|under|own|swap`(チップの重なり一覧と、立っているメニュー(規約 §メニュー)。**`behind` は `ref-list` と対で読む** — 詳細は `AutoActRowMenuVerbs.qml` の `menuListTimer`、判定は `verify/verbs/graph.rs`)
- `ref-list [<行>]` / `ref-list-card [<行>]`(チップ列の展開と、行カードとの対読み。引数を省くと HEAD の行 — 詳細は `AutoActGraphRowVerbs.qml` の `ref-list` の枝と `coverOf`、判定は `verify/verbs/graph.rs`)
- `graph-head-list [click]`(**`--preset deep`**。HEAD の張り付き行のチップを hover で展開した姿。`click` はカードの 1 行目を押すところまで — 詳細は `AutoActGraphRowVerbs.qml` の `headListTimer`、判定は `verify/verbs/graph.rs`)
- `ref-list-lit <行>[:<カードの行>]`(本番は `0:1 --preset tags`。一覧の行に手を止めた姿 — 詳細は `AutoActGraphRowVerbs.qml` の `listLitTimer`、判定は同上)
- `ref-list-choose <行>[:<カードの行>]`(既定 `1:0`、`--preset tags`。一覧の行を Ctrl 付きでクリックした姿(規約 §複数のコミットを選ぶ)— 詳細は `AutoActGraphRowVerbs.qml` の `ref-list-choose` の枝と `listChooseTimer`、判定は同上)
- `row-part <行番号>:<行に沿った x>:<chip|row>`(`--preset tags`。hover がグラフ行のどこで分かれるかを 1 点で訊く — 詳細(本番の点)は `AutoActGraphRowVerbs.qml` の `row-part` の枝と `rowPartTimer`、判定は同上)
- `ref-list-leave <行|head>:<off|back|menu|行番号>`(チップの一覧が、手がチップ列を離れた後にどうなるか。組は `0:off` / `0:1` / `0:back` / `0:menu`(`--preset tags`)と `head:off` / `head:back` / `head:menu`(`--preset deep`)— 詳細は `AutoActGraphRowVerbs.qml` の `leaveTimer`、判定は同上)
- `graph-choose [<行>:<行>…]` / `graph-choose-range [<最初>:<最後>]`(複数のコミットを持つ選択(規約 §複数のコミットを選ぶ)。`graph-choose` は Ctrl で 1 行ずつ(既定 `1:3:5`)、`-range` は Shift で一掃(既定 `1:5`、2 つ渡す)— 詳細は `AutoActChooseVerbs.qml` の `chooseTimer`、判定は `verify/verbs/graph.rs`)
- `graph-choose-dbl [<行>:<行>…]`(選択を組み立てる手は選択だけを動かす = 最後の行で Ctrl のまま 2 回押す — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer.doubled`、判定は同上)
- `graph-choose-drop [<行>:<行>…]`(持っているコミットを COMMITS の一覧から Ctrl クリックで外す — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer.dropped`、判定は同上)
- `graph-choose-show [<行>:<行>…] --preset deep`(既定 `40:42:44` = 選んだ後にグラフを流してから押す。COMMITS の一覧の行の素のクリックで、グラフがそのコミットの行を見える所へ送る = 画面外なら中央へ、見えていれば動かない — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer.shown`、判定は同上)
- `graph-choose-show-diff [<パス>]`(引数・押す行は `graph-choose-diff` と同じ。積み重ねの diff が中央に開いている時の同じクリックで、diff が閉じてグラフが戻る — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer.shownOverDiff`、判定は同上)
- `graph-choose-diff [<パス>]`(既定 `src/topic.txt`、押す行は `3:5:6` 固定。和集合のファイル行を開くと選んだコミットの patch が縦に積まれる — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer.stacked`、判定は同上)
- `graph-choose-said` / `graph-choose-sweep`(引数は `graph-choose` と同じ、**`--preset co-authors`**。選択の一覧(COMMITS)の行のカードと、行の字を四隅から掴む手 — 詳細は `AutoActChooseVerbs.qml` の `chosenRowTimer`、判定は同上)
- `graph-reclick [<行>]` / `graph-rename <行>:<付ける名前>`(グラフ行の「間を空けた 2 回目のクリック」= 改名の所作。`graph-rename` は git まで通す書き込み動詞 — 詳細(preset ごとの行)は `AutoActGraphReclickVerbs.qml` の `reclickGraphTimer`、判定は `verify/verbs/graph.rs`)
- `graph-rename-fit <行>[:<列幅>[/<レーン列幅>][:<打つ文>]]`(同じ所作で開いた改名の箱の立ち位置と、まわりの押下(`GraphRowChips.qml`)— 詳細は `AutoActGraphReclickVerbs.qml` の `graph-rename-fit` の枝と `fitLine`、引数ごとの判定は同上)
- `graph-reclick-scrolled <行> --preset deep` / `graph-reclick-across <行>:<カードの行>`(所作が壊れる 2 つの形の再現 = 2 回目の直後に履歴を流す / 1 回目を行・2 回目をカードに — 詳細は `AutoActGraphReclickVerbs.qml` の `reclickGraphTimer` / `reclickListTimer`、判定は同上)
- `graph-reclick-still <行>` / `graph-reclick-mark <行>`(`--preset tags`。待ちの間の 2 つの規則(`RefChip.waiting` / `RowHoverHost.qml`)= 何も動かさない / 印を出す — 詳細は `AutoActGraphReclickVerbs.qml` の `reclickGraphTimer`、判定は同上)
- `graph-reclick-lanes [<行>]`(`--preset tags`。レーン列の帯越しの所作 — 詳細は `AutoActGraphReclickVerbs.qml` の `laneClickTimer`、判定は同上)
- `graph-reclick-elsewhere [<行>]` / `nav-reclick-elsewhere <section>[:<row>]`(`--preset tags`、左メニューは `branch`。改名は「そこを 2 回続けてクリック」— 間に何か挟んだら 1 回目からやり直す — 詳細は `AutoActGraphReclickVerbs.qml` の `elsewhereClickTimer` / `AutoActNavVerbs.qml` の `elsewhereTimer`、判定は `verify/verbs/graph.rs` / `nav.rs`)
- `rename-box-out same-row|other-row|escape|away|typed-away`(`--preset tags`、行は 0 固定。箱から出る 5 本の道 — 詳細は `AutoActNavBoxVerbs.qml` の `boxOutTimer`、判定は `verify/verbs/graph.rs`)
- `ref-list-follow [<行>[:<カードの行>]]` / `ref-list-follow-lit`(チップのカードの段の名前を押す / 手を置く(`RefChip.mate` / `mateGoes`)。`--preset basic` の HEAD の行と `1:0 --preset tracked-elsewhere` の 2 方向 — 詳細は `AutoActGraphRowVerbs.qml` の `listFollowTimer`、判定は `verify/verbs/graph.rs`)
- `graph-reclick-list <行>[:<カードの行>]` / `ref-list-pick <行>[:<カードの行>]`(同じ所作と、その隣のダブルクリック(移動 = 書き込み動詞)を、チップが開くカードの行に入れる。本番は `5:0` — 詳細は `AutoActGraphReclickVerbs.qml` の `reclickListTimer`、判定は同上)
- `ref-list-held <行>:<カードの行> --preset worktrees`(**引数・preset とも必須**、本番は `1:0`。カードの行のうち、別の worktree が持つブランチの行のダブルクリック = その worktree への道 — 詳細は `AutoActGraphReclickVerbs.qml` の `listHeldTimer`、判定は同上)

- `fetch-ref-list <行>[:<タグ名>]`(`--preset tags`。`<行>:<タグ名>` は `--preset tagremotes`。fetch を撃ってから行のチップを展開し、基準のリモートと別のコミットに立つタグの段を読む — 詳細は `AutoActPublishVerbs.qml` の `fetchedRefListTimer`、判定は `verify/verbs/graph.rs`)
- `signature <行> --preset signed`(詳細ペインの名前の右肩に立つ印。**引数は印を持つコミットの行**(行 0 は作業ツリー = `signature 1` が最新のコミット)— 詳細は `AutoActTipVerbs.qml` の `signatureTimer`)
- `signature-tip <行> --preset errsig`(署名のツールチップ。**他の preset では FAIL** — 詳細は `AutoActTipVerbs.qml`、判定は `verify/verbs/identity.rs`)
- `stash-tip <行>|older`(読み取り専用になった summary 箱のツールチップ。stash の行でも HEAD より古いコミットの行でも同じ動詞 — 詳細は `AutoActTipVerbs.qml`)
- `field-menu [empty|typed|selected|readonly]`(**欄の右クリックメニュー**(`FieldMenu`)を summary 箱の状態ごとに。前の 3 つはコミット箱、`readonly` は HEAD の 1 つ下の行の summary。判定は報告行の `live=` — 詳細は `AutoActDetailsVerbs.qml` の `fieldMenuTimer`、判定は `verify/verbs/panes.rs`)
- `key-menu <面>[:…]`(**キーボードから開く右クリックメニュー**(規約 §メニュー のキーボード・`KeyMenu`)。面は `graph` / `graph:request` / `graph:pointer`(**開かないのが正**)/ `files:<path>`(`--preset dirty`)/ `diff:<path>`(`notes.txt --preset manyhunks`)/ `nav:<kind>:<name>` — 詳細は `AutoActKeyMenuVerbs.qml`、判定は `verify/verbs/panes.rs`)
- `path-tip [<面>][-tree][@<行>]`(ファイル行の hover が出すフルパスのツールチップ。`-tree` はツリーのフォルダ連鎖行。`--preset longpaths` で省略された行を、席の絵は `wip@2 --preset dirty` で撮る — 詳細は `AutoActTipVerbs.qml` の `pathTipTimer`、判定は `verify/verbs/nav.rs`)
- `tip-copy`(**ツールチップの字が持ち帰れる欄であること**。引数なし・`--preset longpaths`。**写真では判定できない** — 詳細は `AutoActTipVerbs.qml` の `tipCopyTimer`、判定は `verify/verbs/nav.rs`)
- `tip-sweep`(**ツールチップの字を「言葉の周りの帯」から掴めること**(規約 §hover のツールチップ)。仕込みは `tip-copy` と同じ — 詳細は `AutoActTipVerbs.qml` の `tipSweepTimer`、判定は `verify/verbs/nav.rs`)
- `card-sweep [<行>]`(**hover カードの字を「行の間・padding・短い行の脇」から掴めること**(規約 §hover のツールチップ)。引数は `row-card` と同じグラフ行で既定は 1 — 詳細は `AutoActGraphRowVerbs.qml` の `cardSweepTimer`、判定は `verify/verbs/graph.rs`)
- `author-card` / `author-card-open`(author 名の平常時とカード。`--preset authorship` — 詳細は `AutoActTipVerbs.qml`、判定は `verify/verbs/panes.rs`)
- `co-authors <行>[:<幅>|:min]` / `co-authors-open <行>`(日付行の credit 表示とカード(規約 §co-author の表示)。`--preset co-authors`、最小幅で畳む形は `0:min` / `8:min --preset octopus-mates`。幅は `details-parents` と同じ指し方。名前が欠片になっていないことは絵で見る — 詳細は `AutoActTipVerbs.qml` の `coAuthorTimer`、判定は `verify/verbs/panes.rs`)
- `details-select-away <先の値>+<次の値>[:<行>]`(**選択は窓に 1 つ**(規約 §右のペインの字は掴める)。例 `author+hash` / `date+mate --preset co-authors`。**写真では答えられない** — 詳細は `AutoActDetailsVerbs.qml` の `detailsSelectTimer`、判定は `verify/verbs/panes.rs`)
- `details-sweep <値>[:<行>]`(**値の範囲選択を「判定の無い余白」から取れること**(規約 §右のペインの字は掴める)。値は `author` / `date` / `mate` / `hash` / `parent`(`mate` は `--preset co-authors`・他は `--preset authorship`)— 詳細は `AutoActDetailsVerbs.qml` の `detailsSweepTimer`、判定は `verify/verbs/panes.rs`)
- `details-hand [<行>]`(**ハッシュの板の所作 —— 単クリックはフルハッシュのコピー・ドラッグは画面の 8 桁の選択**(規約 §右のペインの字は掴める)。`--preset authorship`。**写真では 1 つも答えられない** — 詳細は `AutoActDetailsVerbs.qml` の `detailsHandTimer`、判定は `verify/verbs/panes.rs`)
- `hash-tip [<行>]` / `hash-tip-counting [<行>]`(**コピーの答えは板の 1 語**(規約 §hover のツールチップ)。`--preset authorship`。後者は tip が出る前に押す順 — 詳細は `AutoActDetailsVerbs.qml` の `hashTipTimer`、判定は `verify/verbs/panes.rs`)
- `details-select <値>[:<行>]`(**右パネルの値がドラッグで掴める欄であること**(規約 §右のペインの字は掴める)。値と preset は `details-sweep` と同じ、切れた欄は `mate:7 --preset edges`。**写真では判定できない** — 詳細は `AutoActDetailsVerbs.qml` の `detailsSelectTimer`、判定は `verify/verbs/panes.rs`)
- `row-card <行>`(グラフ行の hover カード — 詳細は `AutoActGraphRowVerbs.qml` の `rowCardTimer`、判定は `verify/verbs/graph.rs`)
- `row-card-return [<行>]`(**カードへ降りた手が同じ行へ戻る往復**。既定は行 1。横跳びそのものは実窓でしか見えない — 詳細は `AutoActGraphRowVerbs.qml` の `cardReturnTimer`、判定は `verify/verbs/graph.rs`)
- `card-message 7 --preset edges` / `card-message-esc 7 --preset edges`(カードが**切ったメッセージの下に出す 1 行を押す** / 同じ歩きを Escape まで。**切るメッセージを持つのはこの preset のこの行だけ**(他は watchdog まで無言)— 詳細は `AutoActGraphRowVerbs.qml` の `cardMessageTimer`、判定は `verify/verbs/graph.rs`)
- `card-note-lit 7 --preset edges`(**同じ 1 行に手を止めた姿**。休息側の `row-card 7` と対で読む — 詳細は `AutoActGraphRowVerbs.qml` の `cardNoteTimer`、判定は `verify/verbs/graph.rs`)
- `details-grow` / `details-grow-squeeze` / `wip-grow` / `wip-grow-squeeze`(description 枠の grip — 詳細は `AutoActDetailsVerbs.qml` の `descGrowTimer`、判定は `verify/verbs/panes.rs`)
- `nav-rename`(左メニューの改名入力欄 — 詳細は `AutoActNavBoxVerbs.qml`)
- `replace-remote-box`(リモートブランチの差し替えの入力欄 — 詳細は `AutoActNavBoxVerbs.qml`)
- `avatar-rest` / `avatar-hover` / `avatar-tip` / `avatar-assign` / `avatar-badge` / `avatar-settings` / `avatar-row-lit` / `avatar-remove` / `avatar-combo`(アバター一式: 静止 / ペンのバッジ / バッジの文 / 与える / バッジ / 設定一覧 / Remove の hover(赤い側)/ 回数押しで消した後 / 候補の combo — 詳細は `AutoActFindVerbs.qml` / `WindowSettingsActs.qml`、判定は `verify/verbs/identity.rs`)
- `avatar-enter`(**候補欄の Enter が `Choose avatar…` の扉を開く**(`SettingsAppPane.qml`)。**絵では判定できない** — 詳細は `WindowSettingsActs.qml` の `avatarEnterTimer`、判定は `verify/verbs/identity.rs`)
- `recover [fold,lit,blink]` / `recover-open [<n>|none][,fold,lit,card,filter:<語>,restore,part:<n>]`(破棄記録(`platitude_core::discards`)の席・一覧・帯・カード・復元。**`--preset discards`** が全種を持ち(番号は `xtask::demo::discards`)、**`--preset discards-deep`** は窓の外の 1 件 — 引数と報告行は `AutoActRecoverVerbs.qml`、判定は `verify/verbs/recover.rs`)
- `recover-escape --preset discards`(仮表示の Esc: 下りる・読み直しの下でも下りる・キーボードが一覧へ戻る・F5 の下で黄色の `Not loaded` を言わない。**絵では判定できない** — 詳細は `AutoActRecoverVerbs.qml` の `escapeTimer`、判定は `verify/verbs/recover.rs`)

## 動詞ごとの読み方・仕込み

**先端に答えが立つ 3 つ(`merge-branch` / `cherry-pick` / `revert-commit`)は 1 本の報告行を共有する** — `tip_landed follows= onscreen= op= head= selected= row=` で、**合格条件は `follows=` / `onscreen=` に `op=` を足した 3 つ**(`op=merge` / `op=cherry-pick` / `op=revert`)。選択が新しい先端のコミットへ移り、その行が画面に在ることを言う(規約 §履歴を合流させる)。**merge / cherry-pick は `--preset diverged` の `origin/main`**(`--preset basic` は木が汚れていて `git cherry-pick` が落ちる)。**取り込み済みの merge も先端に着くのが正**(`merge-branch v0.1 --preset basic`)。完了の門と罠は `AutoActHistoryVerbs.qml` の `tipLandedTimer`、判定は `verify/verbs/sequencer.rs`。

**「止まった側」の 4 動詞は 1 本の報告行を共有する** — `cherry-pick-stops` / `revert-stops` / `rebase-stops` / `drop-stops`(merge だけは `merge-stops` = 出口の形が違うので報告行も別)。共有する 1 本は `write_stopped wip= conflicts= error= log= cont= home= stashes= op= files=` で、**合格条件は `wip=true conflicts=true error=false log=false cont=true`**(`rebase-stops` / `drop-stops` は `home=repo` まで、`drop-stops` はさらに `stashes=1`)。**preset は「まだ止まっていない木」でなければならない**・**失敗した run の木は捨てる**。各値の意味・待ち・preset は `AutoActHistoryVerbs.qml` の `opStoppedTimer`、判定は `verify/verbs/sequencer.rs`。

- `tab-widths [ample|short]` / `tab-mark [<タブ>]`(**タブの幅と `✕`**。xtask が長さの違う名前でリポジトリを建てる。`tab-widths` の引数は**到達する状態**(枚数ではない)で、報告行の `widths=` を形で読む。`tab-mark` の引数は手を置くタブ(既定 1)— 詳細は `WindowTabActs.qml` の `tabWidthActTimer` / `tabMarkActTimer`、仕込みは `verify/repos.rs`、判定は `verify/verbs/window.rs`)
- `tab-name [<タブ>]`(**同名リポジトリのタブ名と hover**。xtask が同じ根の下に 5 つ建てる — 詳細は `WindowTabActs.qml` の `tabNameActTimer`、仕込みは `verify/repos.rs`、判定は `verify/verbs/window.rs`)
- `tab-drag [<from>:<to>]` / `tab-hold [<タブ>]`(**タブの並べ替えは対でしか意味を成さない**。xtask が 4 つ建てる。`tab-drag` は両向き撃つ(既定 `3:1` が左・`0:3` が右)— 詳細は `WindowTabActs.qml` の `tabDragTimer` / `tabHoldTimer`、判定は `verify/verbs/window.rs`)
- **作業ツリー行への 2 つの着地は `wip-landing --preset carried` / `wip-landing-stopped --preset carried-clashing`**(着地が乗るのはこの窓の行を持つ pass だけ、を既定選択と操作停止後の 2 経路で。pass の順は run が決めるので、preset を替えると watchdog まで無言 — 詳細は `WindowBadgeActs.qml` の `wipLandingTimer` / `AutoActHistoryVerbs.qml` の `stoppedLandingTimer`、判定は `verify/verbs/window.rs`)
- `tab-carry --preset basic --preset stashes`(**タブ切替が何を連れて行き何を捨てるか**。タブが 2 つ要る — 詳細は `WindowTabActs.qml` の `tabCarryTimer`、判定は `verify/verbs/window.rs`)
- `tab-edge [<タブ>]`(**帯が横スクロールしている時の並べ替え**。xtask が 12 枚建てて窓を床に落とす。**判定は報告行** — 詳細は `WindowTabActs.qml` の `tabEdgeTimer`、判定は `verify/verbs/window.rs`)
- `tab-pin [<タブ>]` / `tab-pin-go [<タブ>]`(**今居るタブの張り付きタブ**と、それを押して帯が流れる側。staging は `tab-edge` と同じ 12 枚 + 床、引数は前に出すタブ(既定 0 = 張り付きタブは左の縁)— 詳細は `WindowTabPinActs.qml`、判定は `verify/verbs/window.rs`)
- `tab-open-go [<パス>]`(**開いた時に帯がその席へ流れる**。staging は `tab-pin` と同じで前は 0 番固定。引数を省くと xtask が 13 個目を建てて渡す — 詳細は `WindowTabPinActs.qml` の `tabOpenGoTimer`、判定は `verify/verbs/window.rs`)

**判定する所は報告行に隣り合わせで並べる** — `must_say` は 1 本の文字列の**部分一致**なので、間に別の読みが挟まると同じ run が落ちる。

**新しい window 側の動詞を足したら `AutoActCompletion.defersCompletion` の一覧にも足す**(implement.md §壊れない動詞の実装と反復)。**報告の前の最後の待ちに `"page": PageSettled.windowSettled(window)` を入れる** — 完了のターンに page が settled でなければ `complete … page=arriving:` で FAIL(共通の保持は報告の後なので、報告の値は保持では直らない)。**入れ忘れは「たまに落ちる」として出る** — **失敗した run のログは `auto_act ran=` と `auto_act complete=` が同じマイクロ秒に並び、報告行が 1 本も無い**。緑が続いても入れ忘れていないことにはならない。

- `corner wip|<行>`(**右下の git バージョンが隅を譲るか**。**省略はエラー**。**`--preset basic` と `--preset long` を対で読む**(片側だけでは「常に出る」「常に出ない」と見分けが付かない)— 詳細は `AutoActDetailsVerbs.qml` の `cornerTimer`、判定は `verify/verbs/panes.rs`)
- `open-again [<パス>]`(**既に開いているリポジトリを開き直す形**。引数なし(`--preset basic --preset stashes`)と git の綴りを渡す形を対で読み、本題は後者 — 詳細は `WindowTabActs.qml` の `openAgainTimer`)
- `identity [<name>|<email>]` / `identity-half` / `identity-tip`(identity の画面と、半端 save の印。既定の値は `Ada Lovelace|ada@example.com`。**書き込みが demo リポジトリの外 = 人の git 設定へ出る**のはこの系列だけ(隔離は全動詞共通 — linux.md §Linux(コンテナ)での動確「どの run も自分の gitconfig で立つ」)— 詳細は `WindowDialogActs.qml` の `identityTimer` / `identityTipTimer`、仕込みは `verify/shim.rs`、判定は `verify/verbs/identity.rs`)
- `solo`(**2 つ目のプロセスが出す窓**。xtask が `--config-dir` の `lock` を握ってから起動する — 詳細は `WindowTabActs.qml` の `soloActTimer`、判定は `verify/verbs/panes.rs`)
- `band`(**タブ行がタイトルバーを兼ねる帯の形**。掴めるかは絵に出ないので報告行で読む。`--system-title-bar` を付けると兼ねない側(mac / Linux)の形 — 詳細は `WindowBandActs.qml` の `bandActTimer`)
- `app-menu` / `app-menu-reclick`(**☰ の開け閉めの対**。引数なし — 詳細は `WindowBandActs.qml` の `appMenuActTimer`、判定は `verify/verbs/window.rs`)
- `window-fill`(**最大化した窓の中身が四隅まで届いているか**。**絵では読めない** — 詳細は `WindowFrameActs.qml` の `fillActTimer`、判定は `verify/verbs/window.rs`)
- `window-floor [fold|log|wip|rail]`(**窓がこれ以上小さくならないこと**の 5 形(規約 §窓の床)。`wip` は `--preset rebase-conflict` — 詳細は `WindowFrameActs.qml` の `floorActTimer`、判定は `verify/verbs/window.rs`)

**操作パネルの 4 つのボタンが縮む形は `band-actions-none` / `band-actions` / `band-actions-fold` / `band-actions-alert` の 4 つ**(規約 §操作パネル)。**引数は「形」で渡す** — どの pixel でどの形になるかは入っているフォントの問いなので、幅はパネル自身の計算が名乗る(`band-actions` だけは数値でその幅も指せる)。**`--preset longnames`** を当てる(名前が短いと床でも語が残り、縮んだ形に届かない)。`-alert` はさらに `--allow-write-failure` — 詳細は `WindowBandActs.qml`、判定の理由は `verify/verbs/window.rs`。

**最低バージョンを割った git(`old-git` / `old-git-card` / `old-git-fold`)と LFS の無い git(`no-lfs` / `no-lfs-card`、`--preset lfs`)は本物を建てる** — xtask が自分自身の exe を git の名前で子の PATH の先頭に置き、`--version` / `git lfs` にだけ答えて残りは本物の git へ素通しする(アプリは普通に動いたまま)。**動詞名で自動**、他の動詞には `--old-git <版>` / `--no-lfs` で着せる。`old-git` / `no-lfs` の引数は幅か `floor` — 詳細は `WindowBadgeActs.qml` と `verify/git_shim.rs` / `verify/run.rs`。

**設定 / 画面状態の永続化は 2 回の実行で読む**: `state`(引数 `change` で畳み・右ペイン幅・グラフのチップ列 / レーン列の幅・コマンドログ・auto fetch 間隔・初期ロード数を動かしてから申告する / `minimize` で最大化 → 申告 → 最小化 → 申告)。同じ `--config-dir` を渡した 2 回目に `--restore` を足し、報告行が両方で一致すれば往復している — 詳細は `WindowFrameActs.qml`。

**開いたまま止める撮影用の引数・preset**(読み方・待ち・罠は各ドライバのコメント): `settings-tools` / `settings-tools-loading`(設定ダイアログの Merge editor 候補一覧を実際に開いて止める。前者は settled 側、後者は実際の loading=true edge を latch する側。引数なし — `WindowSettingsActs.qml`)/ `file-menu` / `file-menu-untracked` / `file-menu-staged` / `file-menu-conflict`(引数はパス。conflict は種別の文言を `conflict_kind` として報告する。`--preset conflict-kinds` に 4 種が揃っている — `AutoActFileRowVerbs.qml`)/ `ref-list-card` と `row-card`(引数は行番号。同じ行・同じ preset で対で読む — `AutoActGraphRowVerbs.qml`)/ `signature-tip`(引数は行番号。`E` は `--preset errsig` の行 0 だけが持ち、gpg バイナリが要る)/ `stash-tip`(引数は行番号か `older` = HEAD の行の 1 つ下。`older` は stash を持たない preset と組む(`--preset co-authors`))/ `path-tip`(引数は `wip`(既定)か行番号、または `carried:<worktree 名>`(`--preset carried` と組む))/ `author-card` / `author-card-open`(引数は行番号。`--preset authorship` が 4 状態を行 0〜3 に並べる — 以上 4 つは `AutoActTipVerbs.qml`)/ `details-fit`(引数は行。省くとページが自分で開いた行のまま測る。壁は `--preset edges` の行 7)/ `details-grow` / `details-grow-squeeze` / `wip-grow` / `wip-grow-squeeze`(引数は詳細ペインが行番号・commit 欄は打ち込む本文(省略時は xtask が長い本文を入れる)。**PNG は必ず目視する** — 以上 2 組は `AutoActDetailsVerbs.qml`)/ `replace-remote-box`(引数 `<remote>/<old>:<入れておく名前>`。既にリモートに在る名前を渡せば拒否された枠が撮れる — `AutoActNavBoxVerbs.qml`)

**アバターは `avatar-*`**(**引数は取り込む画像のパス**。保管庫は実行ごとに空なので、見るものはまず入れる。**`avatar-assign` とカードの 4 つ(`avatar-settings` / `avatar-row-lit` / `avatar-remove` / `avatar-combo`)は引数を省くと xtask が画像を作って渡す**。**パスを手で渡す run は `gate --verb` に乗らない**(パスは OS ごとに別物)。**引数を取らない `avatar-tip` / `avatar-rest` / `avatar-hover` はそのまま乗る** — 詳細は `AutoActFindVerbs.qml` / `WindowSettingsActs.qml` と `verify/repos.rs`)。

## 通信中(リング)と起動直後の狙い方

**通信中は実 edge の latch で撮る**(型は implement.md §壊れない動詞の実装と反復)。`file://` の subprocess は 50〜150ms で終わり、
並行 build 等でその幅は桁ごと動く。既存の決定的な中間状態:

| 撮りたい状態 | 動詞 | 因果的な証拠 |
|---|---|---|
| force push 通信中(枠あり) | `force-push-hold --preset diverged` | `busyOp == "push"` の edge か、同じ notify が運んだ答えの列の `push`(1 回の drain で始まって終わった時)を latch。`push_hold mode=diverged busy=true` |
| fetch 通信中(枠なし) | `fetch-busy` | `busyOp == "fetch"` の edge か、同じ notify が運んだ答えの列の `fetch`(1 回の drain で始まって終わった時)を latch。`fetch_busy busy=true fails=0 turned=false` |
| merge tool 候補 loading | `settings-tools-loading` | `mergeToolsLoading == true` を latch。`settled=false loading=true open=true` |

fetch の黄 / 赤、通常 push、起動直後の loading を撮る必要があるなら、同じ型の専用 latch
動詞を先に追加する。`--watchdog-ms` を短くしても画像は保存されず FAIL になるため代用に
ならない。`Motion.stilled` はリング回転を止めるだけで、busy を
作った証拠にはならない。

**起動直後の 0 / false は答えの前の姿**(rules/app-ui.md §UI 自動化)。ゲート(git 不在 / 二重起動)は
元から出力状態なので、その `visible` を待ち `gate.visible ? gate : mainUi` の出ている側を掴む。

**ヘッドレスで色を確かめる時はデモリモートの URL を疑う** — 届かないリモートを使う
検証(`fetch-fail` / `fetch-resume` / 黄の `push` はどれも
`--preset unreachable` + `--allow-write-failure` で塞ぐ)
で通信が成功してしまうと失敗の記録が
消え、黄も赤も出ない(逆に `fetch-recover` は**回復側が届く origin に乗っている** —
塞いだままだと成功が来ず、退く絵が撮れない)。実験で `remote set-url` を触ったら戻すこと。**URL は demo repo の
`.git/config` を直接書き換えるのが早い**(`git -C` が通らない worktree セッションでも
届く)。`--preset diverged` は `push -f` の形をそのまま出すので、**枠のある状態と
`!` の同居**はこれで撮る。

## 面の掃き

**掃きの報告文は 1 本**(`SweepPad.sweepAir`)— `all=` / `caret=` / `hand=` / `reach=` / `miss=` / `text=` は部品側が書く。動詞ごとの主張だけが `hand=` の後ろに挟まる(`card_sweep` の `open=` が並ぶ席はそこ)。**`all=` が判定・`reach=` は診断** — 面の余白の箇所数はフォントと機械で変わる。**`hand=` を必ず並べる**(run は関数を叩くので `MouseArea` を消しても掃きは緑になる)。

- `gate-sweep`(**起動ゲートの掃き** = `solo` と同じ仕込み。引数なし。**git が無い側のゲートには動詞が無い** — 詳細は `WindowDialogActs.qml`)
- `open-dialog-sweep`(**開けなかったフォルダのダイアログの掃き** = `open-not-a-repo` と同じ仕込み(引数なしで xtask がフォルダを作る)— `WindowDialogActs.qml`)
- `open-fail-sweep`(**同じ失敗のタブ側画面の掃き** = `open-fail-tab` と同じ仕込み — `WindowDialogActs.qml`)
- `ask-sweep`(**質問バーの掃き** = `move-ask` と同じ仕込み。引数はリモート ref(既定 `origin/main`)、**`--preset diverged` が要る** — `AutoActSwitchVerbs.qml`)
- `settings-sweep`(**設定画面の章の列の掃き**。引数は分類 = `app`(既定)/ `git`。**`Bash` から `settings-sweep git` は撃てない** — 単独の `git` が引数に立つ形は worktree の hook が deny するので、PowerShell から撃つ — `WindowSettingsActs.qml`)
- `diff-band-sweep`(**DIFF 帯のパスの掃き**。引数は `diff-file` と同じ `<パス>` か `<bucket>:<パス>`(**必須**)。撮るのは `--preset longpaths` の `unstaged:crates/platitude-core/src/session/integration/support/fixtures/refs_join_snapshot.rs` — `AutoActDiffVerbs.qml`)
- `plan-amend-kept`(**プラン以前から枠に在った素の amend 文**が、プランを開いて `Discard` した後も残ること。引数なし・`--preset plan` — `AutoActPlanVerbs.qml`)
