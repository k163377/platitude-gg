# PG_AUTO_ACT 動詞表(正本)

[SKILL.md](SKILL.md) の参照資料 — **1 動詞 1 行**(近縁動詞は束ねる・段落側だけの動詞は §索引)、使う動詞名で Grep して該当行と続く節の該当段落だけを読み(全読みしない)、追記も 1 動詞 1 行(段落で書く仕込みは下の節へ)。

## 動詞一覧

書き込み操作の headless 検証は **`PG_AUTO_ACT` = 動詞 / `PG_AUTO_ACT_ARG`**:

- `commit`(引数はメッセージ。**省略時は `chore: commit from the headless run`** — 空のまま撃つと git が拒み、コミットの積まれていない絵が撮れる)
- `amend`(同じく引数はメッセージだが、**省略時は `--no-edit`** = HEAD の文言をそのまま持ち上げるので既定でも書き込みは成立する)
- `amend-reset-author` / `stash` / `stash-staged` / `stash-file` / `switch` / `switch-remote` / `squash` / `reword` / `edit-message` / `edit-message-leave` / `edit-message-discard`
- `edit-message-focus`(description 枠に caret を入れる = 打っている間の色。**何も打たない**ので **body を持つコミットが要る** — `--preset co-authors` の行 0 = HEAD。**休息側は同じ preset・同じ行の `co-authors 0`** で、対で読む。報告行は `message_focus pane= focused= color=` で、読むのは **`color=`**(`textPrimary` `#e2e8f0` / `textSecondary` `#94a3b8`)。`forceActiveFocus()` は offscreen でも効く = フォーカスは hover と違って注入できる)
- `cherry-pick`(引数は ref 名 / 完全な oid / `row:<n>`。グラフ行にある commit なら先にそこへスクロールして選ぶ。下の `tip_landed` を読む)
- `stage-hunk` / `stage-line` / `push` / `force-push` / `force-push-hold`
- `push-retry`(**ボタンが黄 + `!` になる形は push を届かないリモートへ撃つ** — `fetch-fail` と同じ仕込み。push-retry は `--preset diverged` で実際の非早送り拒否を受けてから force で着地させ、報告行 `push_retry refused= branch=` が印の立った側を言う。**印が消えた側は最後の絵で見る**)
- `reset-soft` / `reset-mixed` / `reset-hard`(引数は完全な oid か `row:<n>`。**省略時は HEAD の 1 つ下の行** — 先端への reset は何も動かず絵にならない)
- `reset-hard-confirm`(hard はサブメニューの長押し行 — -confirm はメニューと行を出した所で止まり、reset-hard が長押しを完走させて実行。引数の解決は reset-soft と同じ)
- `commit-menu` / `reset-menu` / `fetch` / `settings`
- `open-picker`(**ダイアログはプラットフォームの窓なのでどちらの PNG にも写らない** — 判定は報告行 `picker folder=` = 開いているリポジトリの親フォルダの `file:` URL)
- `open-not-a-repo` / `open-bare` / `open-not-a-repo-retry` / `open-not-a-repo-cancel` / `open-fail-tab` / `open-fail-tab-bare`
- `open-fail-tab-log`(**開けないフォルダの 6 通り**。規約 §リポジトリを開く = 置き場所は「どの道で来たか」・文言は種別。**引数を渡さなければ xtask がそのフォルダを作る** — 素のディレクトリか `git init --bare` した先で、**リポジトリになれないものが主題なので `--preset` で名指しできない**。前半 4 つがピッカー経由 = ダイアログ(報告行 `open_failed kind= dialog= tabs= active= near=`。**`tabs=` が「開けなかったフォルダはタブにならない」の答え**で、`-retry` は既存の `picker folder=` が**失敗フォルダの親**を名乗るか・`-cancel` は `tabs=` と `active=` が入る前と同じかを見る)、後半 3 つがそれ以外の道 = タブの中の画面(報告行 `open_fail_tab tabs= state= kind= commands=`。`-log` は `>_` を押して**その画面からログが届く**ことを見る = パネルは空でよい、届くことが答え)。**`dialog=` / `commands=` はどちらもその窓自身の可視性**。**3 つ目の種別(`other` = git が答えられなかった)はアプリから作れない** — 起こせるのは実質 10 秒のタイムアウトだけなので、撮る時は `repo::open` が `TimedOut` を返す**使い捨てパッチ**を当てる(コミットしない))
- `preview` / `preview-unstaged` / `preview-staged` / `dbl-local` / `dbl-remote` / `move-branch` / `name-branch` / `commands` / `commands-fail`
- `commands-clear`(**ヘッダの `Clear` が空にするのは行だけではない** — 帯の `>_` は行と**ヘッダの 1 行**の両方で赤くなるので、行だけ消すと空のパネルの上に赤が残る。`commands-fail` と同じ拒否から始め、着いてから `Clear` を押す。**`--allow-write-failure` が要る**。報告行 `commands_clear was= wrong= mark= open=` は**同じ 1 つの印を押す前と後に読む** — `was=true` が無いと、拒否が着かなかった run の休息状態が修正に化ける(`must_say` は両方)。`mark=` は印自身の色で、**パネルは開いたままなので `textPrimary`(`#e2e8f0`)が正**・`open=true` がその席)
- `fetch-fail`(引数は連続で失敗させる回数 — 1〜2 で警告の形、3 で auto fetch が止まった形。**リモートが届かないリポジトリが要る**: demo repo の origin を存在しないパスへ向けておく)
- `fetch-resume`(3 回失敗させてから、止まったボタンの長押しを完走させる)
- `fetch-recover`(失敗 → 成功の遷移: fetch の失敗がヘッダの 1 行と赤い `>_` を立て、**後続の fetch 成功がその両方を自動で下ろす**側を見る — 意匠の正本は rules-refs/app-ui.md の同名の項。**仕込み不要 = 既定 preset で自己完結** — 失敗側は**存在しないリモート名**への fetch(`git fetch --prune pg-no-such-remote` が exit 128)、回復側は届く file:// origin への fetch。**`--allow-write-failure` が要る**(前半の拒否がこの動詞の見せ物)。報告行 `fetch_recover was= hadline= wrong= line= failures= open=` は**成功の前(`was=` / `hadline=`)と後(残り)を対で読む** — 絵は退いた後しか持てない。ログ本体の失敗行は残るのが正 = 履歴は読者のもの)
- `nav-dbl`(引数 `<section>:<name>`)
- `nav-filter`(引数は打ち込む文字列。左メニュー上端の帯に**入力欄そのものへ**書く = 5 セクションが一斉に絞り込まれる。**絞り込んだ行はツリーを外れて全名で並ぶ**ので、フォルダ行が消えて `origin/main` のような名前がそのまま出るのが正。報告行 `nav_filter typed= branches= remotes= tags= stashes=` は各セクションの `shown()/total`(0 件のセクションが在るのが普通 — 見出しは総数のまま残る))
- `nav-fold`(引数 `no-tags` でグラフからタグを外した状態にしてから畳む = レールの旗の消えた側が撮れる)
- `nav-peek`(引数はセクション名 `branch` / `remote` / `worktree` / `stash` / `tag`)
- `nav-peek-away` / `nav-peek-into` / `nav-peek-out`
- `nav-peek-shut`(引数は同じセクション名。開いてから**ポインタをセルの外へ退かせる** / **セルを出てリストの中へ入る** / **そこからリストの外(diff / グラフ側)へ抜ける** / **そのセルをクリックする**。`peek=` が空に戻るのが正なのは away / out / shut で、**`-into` だけは開いたままが正**(この 2 つは対で読む — 「リストの中に居る」が言えないと、閉じてよい退き方と閉じてはいけない退き方が同じ状態に潰れる)。`nav-peek-shut` は `collapsed=true` = **クリックで一覧が戻らない**ことも同時に見る。**中身が 0 のセクションは `nav-peek` に `--preset empty` を当てて読む** — 開かないのが正で、`peek=` は空のまま。`--preset empty` で 1 以上あるのは WORKTREES だけなので、開く側はそれで見る)
- `nav-close`(引数は同じセクション名。**畳まず**にそのセクションの見出しを閉じる = 見出しが上げるシグナルをそのまま叩く。報告行 `nav_section closed= header= ground= pane=` で、`header=` が閉じた見出しの上端・`ground=` が最後のセクションの下に残る地の上端。**セクションは上に詰まったままが正**で、閉じた見出しが `pane − rowHeight` に居たら地を持っているのがセクションの側 = 不合格(**ただし本当に溢れている時は正しく最下段に来る** — refs の多いリポジトリでは `ground=pane` = 地が 0 で、それは詰まっている証拠。`--preset basic` なら余りがあるので `header` は 300 未満))
- `nav-peek-rename`
- `nav-unfold`(左メニューを畳む / 畳んだレールの 1 セクションを開く / 畳んで戻す。**hover もクリックも注入できないのでセクションを名指しする**。開いたセクションはポップアップなので overlay.png に写る。報告行は `nav_rail collapsed= width= peek= top= cell= end= pane= editing= diff=` で、**`top=` と `cell=` の一致が「セルの上端から開いた」**・**`end=` ≤ `pane=` が「下へ伸びて下端で止まった」**(規約 §左メニューを畳む)。**この 4 つは `peek=` が名乗っている時だけ読む** — 閉じた後の値は畳んだ Popup のもの。**ペインより行数が多いセクションでしか差が出ない**ので、`--preset manytags`(タグ 2000 本)を当てる。素の `basic` では全セクションが content 高さに収まり、どんな置き方でも同じ絵になる)
- `diff-fold` / `diff-unfold` / `diff-fold-by-hand` / `diff-fold-by-rename`
- `diff-keep-folded`(引数はパス。diff が左メニューを畳む / `✕` で閉じて戻す / **一覧を戻すと diff が閉じる**のを帯のブロックと改名の入力欄の 2 経路で / 先に手で畳んであれば畳んだまま。同じ `nav_rail` 行の `collapsed=` と `diff=` で判定する)
- `rename-branch` / `rename-tag` / `rename-stash`
- `branch-at-tag`(いずれも引数は新しい名前)
- `delete-branch`(**3 つとも引数はブランチ名** — 省かれると `git branch --delete -- ''` を撃つだけで何も撮れない。メニューを開いたまま `--delete`。マージ済みなら消えてメニューが閉じ、拒まれたら行が `branch -D` + `not merged` に化ける。**拒まれる側は `--preset basic` の `feature/topic-a`**)
- `delete-branch-refused`(その化けた行を出したまま止める。報告行 `ref_menu delete=<チップ> <名前> note=`)
- `delete-branch-go`(化けた行の長押しまで走らせて `-D`)
- `delete-blocked-tip`(引数なし。現在ブランチのチップメニューを開き、**押せない削除行のツールチップを強制表示**する(`AppMenuItem.tipForced` = 実 hover と同じ `ToolTip.visible` へ書く)。報告行 `delete_blocked code= tip= reason=` の **`tip=true` が must_say 相当** — **報告のタイマは `tipDelayMs` より後**(800ms。早く読むと絵には tip が出ているのに報告が false になる)
- `menu-highlight`(引数はブランチ名・省略で現在ブランチ。**ポインタの代わりにキーボードの道でハイライトを作る**(`refMenu.currentIndex = 1` = merge の行)ので、重ね色のハイライトを絵で見られる唯一の経路。hover は注入できない)
- `delete-branch-early`(引数はブランチ名。メニューを開いて**待つだけ・書き込み無し** — 開いた時の `checkBranchDelete` が着いた後の削除行を報告する。未マージ(`feature/topic-a`)なら `code=branch -D held=true note=not merged`、マージ済みの引数なら `--delete` のまま `held=false`。報告行 `delete_early asked= merged= code= held= note=`)
- `chip-menu`(引数はブランチ名。**チップ右クリック相当** — `openRecordMenu` 経由で ref メニューを開く。報告行 `chip_menu ref= commit= delete=` の `ref=true commit=false` が正。`delete=` は 200ms 時点のスナップショットで、未マージブランチでは早着した `-D` にも素の `--delete` にもなり得る — どちらも正)
- `chip-menu-current`(引数なし。現在ブランチのチップ — **削除の表が常に開く**(`ref=true commit=false` が正)。ローカルは無効表示。リモートの読み(upstream か同名リモート)があればリモートの 2 行も並び、押せるのは `push --delete` だけ・両方は無効(`--preset basic` の main はこちら)。読みが無ければローカル無効 1 行だけ。コミット行メニューへのフォールスルーは HEAD マーカー等、名前の無いチップだけに残る)
- `delete-tag` / `delete-stash`(`delete-remote` / `-go` は下のリモートブランチの並びの行が正 — 重複掲載しない)
- `delete-stash-row`(**質問は無く**メニューが開いたまま止まり、`-go`(delete-stash-row は引数 `go`)が行の長押しを走らせる)
- `stash-apply-row`
- `stash-pop-row`(グラフ行の Apply / Pop)
- `delete-force` / `delete-tag-go` / `delete-stash-go` / `discard-file` / `delete-file`
- `discard-staged`(引数はパス。同じ 1 行を unstaged / 未追跡 / staged の行から入る。メニューを出したまま止まり、行の文言を報告する)
- `discard-file-go` / `delete-file-go`
- `discard-staged-go`(行の長押しを走らせる)
- `diff-file` / `line-tools` / `hunk-tools` / `pick-lines` / `stage-lines`(**この 5 つと `stage-hunk` / `stage-line` / `keep-place` / `discard-hunk` 系は行が届いてから撃つ** — `RepoPage.stageRowTimer` が 50ms 毎に訊いて着き次第実行し、5 秒で諦める。**全員が `diff_row act= ready= rows= line= waited=` を報告し、`ready=false` は FAIL**(`ready` は 1 つ目の hunk に変更行が在ること。`diff-file` だけは「読み終わり + 行 / 画像 / binary のどれか」)。`waited=` は訊き続けた長さ(tick 数 × 50ms)で、preset は 50・platitude-gg 自身は 250〜300 — 固定 800ms のどちら側に落ちる読みだったかはこれで言える。**改名の source は渡さなくてよい** — ページが `NavSectionModel.origOf(path)` で行と同じ値を埋める。渡さない diff を git は「無から現れたファイル」として読むので、この経路が無いと**改名の diff は全行追加の絵にしかならない**)
- `keep-place`(**引数は `<パス>` か `<bucket>:<パス>`** — bucket は `unstaged`(既定)/ `staged` / `untracked` / `conflicts`。diff を開く / 行がポインタの下で出す `+` を出す / **1 つ目の hunk 見出しの 2 語をポインタの下の色に戻し、その hunk 全体を照らす** / **1 つ目の hunk の変更行を 2 本選ぶ**(`picked_lines any= got= want=` を報告。見出しが `Stage 2 lines` になる。**`must_say` は `any=true` = 1 本も選べなかった run だけを落とす** — `--preset dirty` の `b.txt` は変更行が 1 本しか無いので `got=1` = 見出しはそこで「hunk」を名乗る。**2 本の絵が要る時は変更行を 2 本持つファイルを渡す**(このリポジトリ自身の dirty なソースが手近))/ 選んでから書き込む / **contentY 400 まで読み進めてから partial stage し、戻った位置を `diff_place` で報告する**(20 hunk 級の diff が要る = 300 行超のファイルに 20 か所の変更)。hover は注入できないので行を名指しする。休息時の語は `textSecondary` なので、色の付いた形はこれらの動詞でしか撮れない。**新規ファイルは分ける道具を出さない**(規約 §diff の中のステージ)ので、`untracked:` で開くと `+` も hunk 見出しの 2 語も出ない —— それが正)
- `colour-place`(引数はパス。**色は行の後から届く**(`SessionEvent::DiffColoured`)ので、その入れ替えが読んでいた位置を奪わないかを見る唯一の動詞 — 行が来たら contentY 400 まで読み進め、`DiffModel.coloured` が立ってから報告する。報告行 `colour_place coloured= at= rows= waited=` の **`coloured=true at=400` が `must_say`**。**ハイライトされる言語で行数の多い diff が要る**(`--preset` には無い): `demo-repo basic --at <短い temp>` して 3000 行級の `.rs` をコミット → 全行書き換え → `--repo` で渡す。`.txt` で同じことをすると色が来ないので `coloured=false` = 不合格になる — それが「色の付く側」の対照)
- `eol-hover`(引数はパス。WIP のファイル行の改行コード警告 `!` を名指しし、カードを開く — hover は注入できないので実 hover と同じ 1 つのプロパティへ書く。報告行 `eol_hover path= card= text=` の **`card=` はカード自身の `opened`**。overlay.png に写る)
- `eol-commit`(引数 `amend` 可。警告が index に乗った木で本文だけ入れ、コミットボタンにポインタを置く — 枠 + `!` + カードの絵で、何もコミットしない。報告行 `eol_commit staged= warned= card=`。**この 2 つは `--preset eol`** = 4 ケース + 印の付かない普通の変更 1 つ)
- `discard-hunk`(hunk 見出しの長押しボタンを出したまま止まる)
- `discard-hunk-go`(そのボタンを完走させる)
- `discard-many`
- `discard-many-go`(先頭行 + 引数のパスを Ctrl クリックで選んでから同じ 1 行)
- `stage-many`
- `stage-many-go`(同じ 2 行を選んでから**先頭行の `+` にポインタを置く** — 一緒に動く行の印も出る。`-go` が押す。hover は注入できないので `showStageTools` で名指しする)
- `wip`(作業ツリーの一覧)
- `wip-tally`(同じ一覧を開き、**グラフの未コミット行の集計**を報告する。報告行 `wip_tally added= modified= deleted= renamed= copied= conflicted= rows=` の **`rows=` は一覧の行数**で、**種別の和がそれと一致するのが答え**(`status::Kinds` は行を数えるため)。数字自体は PNG に写るので `must_say` は無い。**4 種類が同時に出る木はどの preset にも無い**ので、撮る時は `demo-repo basic --at <path>` で建ててから `git rm` / `git mv` / `git add` を撃ち `--repo` で渡す。conflict の側は `--preset conflict`)
- `wip-message`
- `wip-message-focus`(commit 欄の description。**対で読む** — 引数(省略可)の body を打つ所までは同じで、`-focus` だけが caret を入れる。commit 欄は空で始まるので、打たずに撮ると placeholder しか写らない。同じ `message_focus` 行を報告する)
- `find` / `find-next`
- `find-prev`(Ctrl+F の検索。キーもキー入力も注入できないのでページの `startFind()` を叩き、引数があればその文字列を箱に入れる = 打鍵と同じ経路で照合が走る。`-next` / `-prev` はさらに Enter / Shift+Enter 相当を 1 回。報告行は 2 本 — `find open= query= matches= at= row= selected= width= cap= clears=` と、300ms 後の `find_settled shift=`。**`width` と `cap` が幅の規則の両側** — `cap` は subject の 1 文字目までの距離で、`width` がそこから `spaceXs + spaceLg` 内側に収まっているのが正(規約 §コミットを探す)。**長いクエリを当てないと幅は最小のまま**なので、幅を見る時は 70 文字級を渡す。**一致がある長いクエリ**を撮るには長い subject を持つリポジトリが要る(`demo-repo basic --at <path>` して長い subject のコミットを 2 つ積み `--repo` で渡す)。`-prev` は端で回るので、先頭の一致から撃つと最後の一致が答える(**一致が 1 件だけの時に `-next` を撃つとその行へ戻る** = 回っている証拠。-1 になったら不合格)。**`clears=` は「グラフが下へ退くか」の判定・`shift=` は実際に退いた量**で、**対で読む** — 退きはアニメーションなので、動詞と同じ呼び出しスタックでは必ず 0。**退く側は `--preset co-authors` に `signed`**(木がきれいで行 0 が一致 → `clears=true` / `shift=32`)、**退かない側は同じ preset の `readme`**(行 7 が一致)と **`--preset basic` の `feat`**(木が汚れていて行 0 が WIP = 決して一致しない)。**0 件のクエリは全行が暗く、カウンタの席が消えて箱が右端まで伸びる**)
- `op-exit`(止まった操作の出口カード。`--preset rebase-conflict` / `rebase-staged` / `rebase-empty` / `cherry-pick-conflict` / `conflict` で全状態が撮れる)
- `op-exit-go`(引数は `--skip` 等のフラグ。その行の長押しを完走させる)
- `take-side-ours`
- `take-side-theirs`(引数はパス。conflict 行のメニューから片側を採る)
- `open-mergetool`(引数はパス。同じメニューから外部ツールへ渡し、`merge_tool <名前>` を報告する。**待ちの表示を撮るには、閉じないツールを設定したリポジトリが要る** — demo repo の `.git/config` に `merge.guitool = <名前>` + `[mergetool "<名前>"] cmd = sleep 30` / `trustExitCode = true` を書いて `--repo` で渡す。未設定なら行が `Choose merge editor…` になり何も起動しない)
- `merge-branch`
- `rebase-onto`(引数はブランチ名。merge は下の `tip_landed` を読む)
- `revert-commit`(引数は**完全な oid** か **`row:<n>`**、省略で HEAD。**その行へスクロールして選んでから**メニューを開いて撃つ = 人がする所作そのもの。下の `tip_landed` を読む。**視界の側は背の高い履歴でしか試験にならない** — ペインに収まる木では最初から `onscreen=true` なので、`demo-repo co-authors --at <dir>` の木に 40 件ほどコミットを積んで `--repo` で渡し、深い行(`row:44` 等)を撃つ。**コンテナへ渡す木は `git config commit.gpgsign false` を先に撃つ**(co-authors の preset は署名鍵を Windows の絶対パスで指しており、`/work` からは読めず `git revert` が exit 128 で落ちる)。**失敗した run の木は捨てるか `reset --hard` する** — 止まった revert が staged を残し、次の run が `your local changes would be overwritten` で落ちる)
- `integrate-menu`(引数はブランチ名。ref メニューを立てたまま止める)
- `drop-commit`
- `drop-commit-go`(引数は**完全な oid** か **`row:<n>`**(グラフの行番号 — 使い捨ての demo repo の oid はヘッドレスからは綴れないので、先端以外を落とす時はこちら。**汚れた木では行 0 が WIP** なので 1 つ下がる)、省略で HEAD のもの。`plan_edit` は symbolic name を取らない。**行は長押しにもクリックにもなる**ので報告行 `drop_row drop this commit oid= hold= reached=` で形を読み、`-go` は出ている方の所作を取る。**両側を撮るなら `--preset diverged` = 長押し / `--preset basic` = クリック**(後者は stash と `v0.3-local` タグが先端に乗っている)。タグだけで決まる形を見たいなら diverged の repo に `git tag` を打って `--repo` で渡す。**未コミット変更のある木では core が stash 経由で回り直す**(規約 §未コミット変更がある状態で履歴を書き換える)ので、`--preset drop-collides` = 戻す時に conflict した着地・`--preset drop-stops` + `row:2` = replay 自体が止まった着地(**`--allow-write-failure` が要る** — 止まったことを git が報告するのがこの動詞の見せ物)。**回り道そのものを絵にするならコマンドログを開けておく** = `state change` を撃った `--config-dir` を次の実行へ渡す(`rebase --interactive` → `stash push` → `rebase --interactive` → `stash pop --index` が並ぶ。**成功した回り道ではログは自分から開かない**))
- `publish` / `publish-taken` / `publish-remotes` / `publish-add` / `publish-go`
- `publish-new-go`(**`--preset unpublished`**。upstream の無いブランチで `push` を押した所で止まる / 名前を `taken`(引数で上書き可)にして**向こうに在る名前の形**にする。**同じ動詞が 3 つの答えを撮る** — 既定の `taken` は早送りできない側(**`push -f` の長押し**)、`carried` は早送りできる側(**素のクリック**)、`outsider` は一度も fetch していない名前(**語も所作も据え置きで枠 + `!`**)。preset がその 3 本を向こうに用意しており、**どれになったかは報告行 `publish settled far= code= hold= alert= lease= theirs=` で読む**(絵は色と印しか語らず、lease が張られたかは写らない) / 行き先のリストを開く(overlay.png)/ `Add remote…` のダイアログを出す(引数 `<name>|<url>`。overlay.png)/ 答える / ダイアログを確定してから答える。報告行は `publish state= remote= branch= dialog= name=` と `publish answering far= unsure= answerable=`(`far` = 向こうが持っていたもの。もう一方の行の `state` はこちらの push 状態で、別の問いに答えている)。**届かないリモートの絵は `publish-new-go` に `file:///nowhere/…` を渡し `--quit-ms 3600`**(ダイアログ確定 1800ms → 答え 3000ms の間)。**リモート 0 本の形は同じ動詞に `--preset noremote`** — 押しただけでダイアログが自ら立つので `publish` の報告行が `dialog=true name=origin` を言い、リモートがある側(`--preset unpublished`)は同じ動詞で `dialog=false` = **同じ 1 つの動詞が両側を答える**(開くかどうかを決めているのはリモートの本数だから)。**`publish-add` の名前欄は空**(prefill は 0 本の時だけ)。**確定した瞬間の絵**(行き先に新しいリモートが入り、ピルの `push` が生きたバー)は `publish-new-go` + `--quit-ms 3600` で、**到達できる push 先が要る** = `git init --bare` した先を `file:///…` で渡し、`--repo` には `demo-repo noremote --at` で作った木を渡す(既定の使い捨て repo は毎回別のパスなので、bare を隣に置けない))
- `rename-remote`(引数 `<remote>/<old>:<new>`。REMOTES の行は畳まれているので ref を名指しする。質問の手前で止まる)
- `rename-remote-go`(長押しを最後まで進めて実行)
- `rename-local-upstream`(引数は新しい名前。手元の改名 → 続く質問まで)
- `delete-remote`
- `delete-remote-go`(引数 `<remote>/<branch>`。同じく ref を名指しする。**質問は無く**畳みが開いてメニューが立ったまま止まり、-go が行の長押しを完走させる)
- `graph-divider`(グラフ列の仕切り。ポインタを仕切りの上に置いて答えを読む(hover は注入できないので `restDividerPointer` = 実 hover と同じ 1 つのプロパティへ書く)。**床が 1 レーンより狭くなって以降、レーン 1 本でも狭める側は動く** — `--preset noremote`(1 本)も `--preset basic`(2 本)も `line=true refuses=false` が正で、`max=` − `min=` が伸びしろ。**この動詞での禁止の輪(`refuses=true`)は窓が上限を床まで潰した時だけ**で、既定の窓では出ない — **上限に当たったドラッグの輪は `graph-divider-max`** が撮る(ホバーでは出ないのが意匠。規約 §グラフ列は最も広い所のレーンまで)。報告行 `graph_divider shown= line= refuses= lanes= max= min=` の `line=` / `refuses=` は**縦線と輪それぞれの `visible`**、`shown=` は `MouseArea` が居ること(**OS のカーソル自体は PNG に写らない** — カーソルとの重なりだけは実窓で見る))
- `divider-refuse`(**窓の中の掴める境界 9 通りの「これ以上無い」**。引数が場所と向き: `graph-max` / `graph-min` / `label-min` / `label-max`(グラフの 2 本の仕切り、`--preset basic`)/ `sidebar-min` / `details-min` / `log-min`(3 本の SplitView バー、同 preset)/ `desc-max` / `desc-min`(description 枠の掴み。**`--preset long` の行 0** = 枠より長い本文が要る。`details-grow` と同じ 800ms 待ちに乗るので `--quit-ms 3000` で足りる)。`must_say` は全ケース共通で `divider_refuse refuses=true line=true` — `line=` は**手が掴んでいる側の境界が出たまま**(グラフは縦線、SplitView はバー、掴みは `descGrips`)で、引っ込むのは「どちらにも動けない」時の答えなので別物。報告行 `divider_refuse refuses= line= case= …`。**輪は PNG に写る**ので絵で読む — 境界は止まった所、輪はポインタが行った先に**離れて**写るのが正。**hover では出ないのが意匠**なので `graph-divider` の絵には写らない)
- `graph-min`(グラフ列を床まで縮める — 仕切りのクランプと同じ `setGraphColumns` へ 0 を渡し、止まった幅を読む。床は**subject のティックがレーン 0 の co-author バッジにギリ重ならない幅**(`GraphPane.graphColWMin` = ceil(バッジ右端) − `spaceSm` — 規約 §グラフ列は最も広い所のレーンまで)。報告行 `graph_min w= min=` は `must_say` が `w=21 min=21` を要求(トークン算術の答えそのもの — トークンを動かしたら verify.rs も焼き直す)。**`--preset co-authors` で撮る**(バッジ付きノードが並ぶ)— 絵は**ティックとバッジの間が 1px 弱で、バッジの輪郭が切れていない**ことを拡大目視する)
- `graph-step`(引数は符号付きの歩数 = 上下矢印で履歴を辿る。**キーは注入できない**ので、キーハンドラと同じ `GraphPane.stepRow()` へ入り、その前に**行クリックと同じ `takeKeyboard()`** でキーボードを取る — グラフを一度も押していない窓は矢印を聞かない(規約 §矢印で履歴を辿る)。報告行 `graph_step from= row= steps= landing= held= back= refused= onscreen= focused= selected=` の **`landing=` が視界の落ち方**(`in` = 動かなかった / `edge` = 端へ 1 行寄せた / `center` = 出発点が画面に無かったので真ん中へ)、**`selected=` は右のペインが追いついたか**(= 落ち着き待ちの着地が起きたか。報告は 400ms 後 > `keyStepSettleMs`)、`refused=` は断られた歩数。`--preset basic` は 14 行 = 既定の窓に収まるので `landing=in`(見えている行へは視界を動かさない)。**歩数を 14 以上にすると端で止まって `refused>0` になる**のが正しい姿で、`must_say` は `refused=0` を要求するので歩数は履歴の中に収める)
- `graph-step-edge`
- `graph-step-far`(**視界が動く 2 つ。`window-floor` と同じ 320x240 の種を撒く** — どの preset も既定の窓に収まる本数しか持たず、折り返しの無いグラフには視界の規則が無い。`-edge` は下端を越えて 10 行歩き `landing=edge`(1 行ずつ寄せた端)、`-far` は先に視界を末尾へ送ってから 1 歩で `landing=center`。**絵では区別できない** — 1 行ずつ着いた端と飛んで着いた端は同じ写真になる)
- `graph-step-diff`(引数は**選択中のコミットが触ったパス**。CHANGES から diff を開くのと同じ `toggleDiff("commit", …)` を通してから 1 歩。**`focused=false` が仕掛けで `diff=true` が見せ物** — Qt は画面から退けたペインにフォーカスを残し、キーもそこへ届き続けるので、放っておくと**裏でグラフの選択が動き、選択が動けば diff が閉じて画面が引き戻る**。キーボードを取るのは diff を開く**前**(順が逆だと、もう画面に無いペインを押しているだけで何も証明しない)。`--preset basic` の HEAD は `docs/guide.md` を触っている)
- `graph-step-named`
- `graph-step-dirty`(**断る 2 つ。`back=true` = 行が動いていないことが見せ物**で、`refused=` と対で読む(歩かなければ同じ絵になる)。`-named` は行に名前箱を開いてから 1 歩 — **単行の入力欄は Up/Down を消費せず親のリストまで上がってくる**ので、これが無いと名前を打っている最中に選択が動く。`-dirty` は詳細ペインのメッセージを書きかけにしてから 2 歩 = **1 歩目は通って `guardEdits` に質問を上げさせ(`held=true`)、2 歩目が断られる**。3 つ目の断る理由(質問バーが立っている)は同じ式で断られ、そちらはピルがキーボードを持っているので動詞を持たない)
- `diff-step`
- `diff-step-edge`(引数は**パス**、`--preset dirty` の `untracked.txt` が唯一スクロールに足りる diff(8 行)。**diff 自身の上下矢印 = 動くのは選択ではなく視界で 1 打 1 行**(規約 §diff を上下に送る)。キーハンドラと同じ `DiffPane.stepRows()` へ入る。**キーボードを取る動作を挟まないのが仕掛け** — グラフと違って diff は**画面に出た時に自分で取る**ので、`focused=true` は「到達しただけで取れた」ことの主張(false なら実窓でも矢印が死んでいる)。**`graph-step-edge` と同じ 320x240 の種を撒く** — どの demo の diff も既定の窓に収まり、スクロールしない面はどの規則も答えられない。`diff-step` は **1 行**送って底の手前(`moved=true atEnd=false stopped=false`)= 「1 打 = 1 行」そのもの、`-edge` は 20 行 = 底を越えて要求し、届かなかった分が `stopped=true` になって**端で止まる**(回らない)。**この種でも可動域は 2 行しかない**(実測 = 8 行の diff と 320x240 の窓)ので、`diff-step` の歩数を増やすと底に着いて `atEnd=false` が落ちる。報告行 `diff_step from= rows= steps= moved= atEnd= stopped= focused=`。**絵はこの動詞で最も弱い証人** — 2 行送った diff と一度も送っていない diff は同じ写真なので、判定は行を読む。上端の止まりは同じ `clampY` の同じ式なので動詞を持たない)
- `graph-bar`
- `graph-bar-away`(レーンの横スクロールバー。**対で読む** — 出る側だけでは何も証明しない。報告行 `graph_bar shown= overflow=`)
- `middle-scroll`(引数 `message` で subject 列から中クリックする。既定はレーン列。報告行 `middle_scroll lanes= x= max=` の `x` が 0 か max か = 横へ流れたかどうか)。**この 3 つはレーンが列に溢れている必要があり、どのデモリポジトリも既定の列幅では溢れない**ので、フックが先にレーン列を 2 本ぶんまで詰める(人が仕切りでやることと同じ)。**`--preset stashes` が 3 レーン**で摘みが 6 割になり絵として読める
- `middle-close`(引数はタブの index、省略で先頭。タブを中クリックで閉じる。**タブが 2 つ以上要る**ので `--preset` を 2 回渡す(`--preset basic --preset stashes` = 2 タブ。`--repo` も同じく繰り返せる)—— 1 タブでは「タブが閉じた」しか言えず「隣は残った」が言えない。報告行 `middle_close tabs= active= open=` の **`open=` は残っているリポジトリのパス**で、**タイトルでは閉じた側を言えない**(デモの作業ツリーはどれも `repo` という名前)。**active より左を閉じたら `active` が 1 つ左へ寄るのが正**)。クリックと同じ経路を通る。

### 索引(段落側に仕込みがある動詞)

以下は上の並びに出ない動詞。**引数・preset・報告行・仕込みは §動詞ごとの読み方・仕込み を動詞名で Grep** して読む(ここは逆引き用の 1 行だけ):

- `tab-widths`(タブ N 個の幅の譲り合い。引数は個数 1..=16)
- `tab-mark`(タブの hover と `✕` の出方)
- `corner`(右下の git バージョン表示の譲り方。報告行 `git_corner`)
- `open-again`(開いているリポジトリをもう一度開く — 訊かれず既存タブへ)
- `identity`(identity 未設定の質問ダイアログ)
- `identity-half`(半分だけ保存された identity から続ける)
- `identity-tip`(半端 save の後の帯の状態カード。overlay.png 側。must_say)
- `solo`(2 個目の起動が断られる報告。must_say `solo blocked=true`)
- `band`(タブ行がタイトルバーを兼ねる帯 — 数字で読む。報告行 `band`)
- `window-fill`(最大化が四隅まで届く報告。must_say `fills=true`)
- `window-floor`(窓の床 = これ以上小さくできない大きさ。must_say)
- `details-fit`(詳細ペインの列が収まる報告。must_say `details_fit fits=true`)
- `badges`(帯の状態が 3 つ揃った形 = `…` 1 つに畳まれている。identity は skip して背後に残す。must_say)
- `badges-hover`(その `…` を hover で開いたカード。must_say)
- `old-git`(最低バージョンを割った git で普通に動いている窓 = 帯の 4 つ目のバッジ。must_say)
- `old-git-card`(その badge を hover で開いたカードの行。overlay.png 側。must_say)
- `old-git-fold`(黄だけが立った窓を床まで縮めた `…` = 印が `warning` で塗られている報告。must_say)
- `state`(設定・画面状態の永続化。`--config-dir` を渡した 2 回目に `--restore`)
- `settings-tools`(設定ダイアログの Merge editor 候補一覧を開いたまま)
- `stash-dialog`(stash のオプションカード)
- `file-menu` / `file-menu-untracked` / `file-menu-staged` / `file-menu-conflict`(ファイル行の右クリック、バケツ別)
- `amend-author`(amend の著者引き継ぎ表示)
- `stash-menu`(stash 行の右クリック)
- `name-box`(グラフ行の名前入力欄を開いたまま止める。引数は行番号)
- `ref-list` / `ref-list-card`(チップ列の展開と、行カードとの対読み)
- `fetch-ref-list`(fetch を撃ってから行のチップを展開 — タグの雲)
- `signature`(詳細ペインの署名 1 語。`--preset signed`)
- `signature-tip`(署名のツールチップ。**must_say `code=E tip=true` = `--preset errsig` 以外では FAIL**)
- `stash-tip`(stash 行を選んだ時の summary 箱のツールチップ)
- `path-tip`(省略されたパスのフルパスのツールチップ。`-tree` 付きはツリーのフォルダ連鎖行。`--preset longpaths`)
- `author-card` / `author-card-open`(author 名の平常時とカード。`--preset authorship`)
- `co-authors` / `co-authors-open`(日付行の credit 表示とカード。`--preset co-authors`)
- `row-card`(グラフ行の hover カード)
- `details-grow` / `details-grow-squeeze` / `wip-grow` / `wip-grow-squeeze`(description 枠の grip。must_say `description_grow keeps=true`)
- `nav-rename`(左メニューの改名入力欄)
- `rename-remote-box`(リモートブランチ改名の入力欄)
- `avatar-rest` / `avatar-hover` / `avatar-assign` / `avatar-badge` / `avatar-settings` / `avatar-row-lit` / `avatar-remove` / `avatar-combo`(アバター一式: 静止 / ペンのバッジ / 与える / バッジ / 設定一覧 / 行の hover / Remove の赤 / 候補の combo)

## 動詞ごとの読み方・仕込み

**先端に答えが立つ 3 つ(`merge-branch` / `cherry-pick` / `revert-commit`)は 1 本の報告行を共有する** — `tip_landed follows= onscreen= op= head= selected= row=` で、**`follows=` / `onscreen=` の対が合格条件**(`Outcome::must_say`)。選択が新しい先端のコミットへ移り、その行が画面に在ることを言う(規約 §履歴を合流させる)。**merge / cherry-pick は `--preset diverged` の `origin/main`** — 木がきれいで、取り込むものが向こうにあり、着地が非早送りの本物のコミットになる(**`--preset basic` は木が汚れていて `git cherry-pick` が `your local changes would be overwritten` で落ちる**)。**選択は撃つ前に先端から動かす**(グラフ行から入る 2 つは `jumpToRow` してから撃つ)— 動かさないと「元から先端に居た」だけで `follows=true` になる。`merge-branch` は ref 行から入るので選択は起動時の先端のまま = **書き込みが先端を動かすことがそのまま試験になる**。**取り込み済みの merge も先端に着くのが正**(`merge-branch v0.1 --preset basic` = `Already up to date` でも `follows=true`)。

**タブの幅と `✕` は `tab-widths` / `tab-mark`** — どちらも**xtask が長さの違う名前でリポジトリを建てる**(デモの作業ツリーは全部 `repo` なので、同名の並びでは「どのタブが譲ったか」を言えない。最後の 1 つだけ `basic` = 絵の中でページを持つのは前に出るそれ)。`tab-widths` の引数はタブ数(1..=16・既定 8)で、報告行 `tab_widths tabs= run= cap= floor= max= content= view= scrolls= widths=` の **`widths=` が本体**(実測 Windows: **8 タブ** = 長い 2 枚だけ cap 167 で揃い短い側は自然幅 / **12 タブ** = cap が下限の 1 つ上 / **16 タブ** = `cap=floor` で `scrolls=true`)。**何タブで縮み始めるかは OS で違う**ので、両 OS で同じ数字を期待しない — Linux は merged chrome を持たない = 掴み代 88 も窓ボタンも要らないぶん run が広く(実測 1133 対 896)、8 タブでは 1 枚も縮まず 16 タブでもまだ収まる。**読むのは数字の一致ではなく形**(短い側が据え置き / 譲った側が同じ幅 / `content` ≤ `view`)。`tab-mark` の引数は手を置くタブ(既定 1)で、報告行 `tab_marks tabs= current= pointed= marks=` の `marks=` は**印自身の opacity**(前に居るタブと手の下のタブだけ 1)。**`✕` の hover 半分はこの動詞にしか無い**(hover は注入できないので `TopBar.pointAtTab` が実 hover と同じ `pointed` へ書く)。

**右下の git バージョンが隅を譲るかは `corner`** — 引数はペイン(`wip` = 作業ツリー / 行番号 = そのコミットの詳細。省略は `wip`)。**`--preset basic`(一覧が隅まで届かない)と `--preset long`(一覧がペインを超える)を対で読む** — 片側だけでは「常に出る」「常に出ない」と見分けが付かない。**両ペインぶん要る**(WIP と詳細は別々に自分の一覧を測る)ので計 4 run。報告行 `git_corner pane= shown= room= needs=` の **`shown=` はラベル自身の可視性**(頼んだ側 = 空き寸法を報告するとバインディングが切れていても緑になる)。**PNG でも読める** — 隅の淡い文字が在るか無いかがそのまま答え。実測 Windows: `wip basic room=499` / `1 basic room=575` = 出る、`long` はどちらも `room=0` = 退く(`needs=21` = ラベルの箱そのもの)。

**既に開いているリポジトリを開き直す形は `open-again`**(引数はパス、省略で先頭タブ自身の綴り。報告行 `open_again tabs= active= asked= open=`)。**2 通りを対で読む**: `--preset basic --preset stashes` に**引数なし**で「移る」側(起動直後は `active=1`、頼んだ後に `tabs=2 active=0`)、**git の綴りを引数で渡す**方が「綴りが違っても同じフォルダ」側(`--repo` にはシェルの綴り = `C:\Users\WRONGW~1\…` を渡し、引数に `git worktree list` が出す `C:/Users/wrongwrong/…` を渡す → `tabs=1`)。**後者だけが本題** — 素の文字列比較でも前者は通る。WORKTREES の行から入る形は `nav-dbl worktree:<git の綴り>` で、判定は `opened repository tab` のログが 1 本きりであること。**復元側は仕込みが要る**(`--config-dir` に重複入りの `[tabs] paths` を書いた `state.toml` を置き `state --restore`)— `restored tab dropped: already open` が落ちた件数を、`state tabs= active=` が残った形を言う。

**identity の画面は `identity` / `identity-half` の対**(引数は打ち込む値 `<name>|<email>`、既定 `Ada Lovelace|ada@example.com`)。**この 2 つだけ書き込みが demo リポジトリの外 = 人の git 設定へ出る**ので、xtask が `GIT_CONFIG_GLOBAL`(+ `GIT_CONFIG_NOSYSTEM=1`)と**作業ディレクトリごと**隔離した上で起動する(隔離しないとこの機械の identity を読み書きする)。`identity` = 画面を埋めるだけ(全フラグが下りているのが正)/ `identity-half` = **多値の `user.email` を仕込んだ設定へ保存する** = git が `cannot overwrite multiple values` で exit 5 に落とすので **name だけ着地**(ロック競合と同じ形を決定的に作る唯一の手)。報告行 `identity state= dialog= nameSaved= emailSaved= unsaved= badge= said=` で、**両方 `must_say` を持つ** — 「save が通らずに残っているダイアログ」と「まだ誰も答えていないダイアログ」は同じ絵なので、写真では答えられない。**`dialog=` が本体**。**半端 save の印は `identity-tip`** — 同じ隔離・同じ半端 save の後にダイアログを閉じ(印は半端の間しか立たない)、`badges-hover` と同じ帯の状態カードを開く(`TopBar.statePointedAt` = 実 hover と同じ 1 つのプロパティ)。報告行 `identity_tip unsaved= badge= tip= rows=` の **`tip=` はカードの `opened`**(unsaved / badge / tip の 3 つが must_say)。カードは overlay.png の側に写る。

**2 つ目のプロセスが出す窓は `solo`** — **xtask 自身が `--config-dir` の `lock` を握ってから**アプリを起動するので、起動するのは本物の 2 つ目(状態を真似るフラグは無い)。報告行 `solo blocked= held= gate= main=` で、**`blocked=true` を言わなかった run は FAIL**(仕込みが効かないと絵は普通の窓になり、普通の窓は普通に撮れてしまう)。`--config-dir` を渡さない既定でよい。

**タブ行がタイトルバーを兼ねる帯の形は `band`**(報告行 `band merged= plain= grabRun= buttonsX= width= tabsW= rightMargin=`。offscreen は窓ボタンを OS が描かない = 絵では欠けが見えないため、数字で読む。**`tabsW=` が NaN / 0 はタブ列が丸ごと消えている**(幅式の項が壊れた時の形 — id が同名プロパティをシャドウすると NaN になる)。`PG_PLAIN_CHROME=1` を立てて撃つと兼ねない側の形 = mac / Linux のレイアウトがこの機械からも出る — `merged=false grabRun=0` が正)。

**最大化した窓の中身が四隅まで届いているかは `window-fill`**(最大化 → 申告 → 撮影のために windowed へ戻す。報告行 `window_fill fills= maximized= at= size= window=` で、**合格条件は `fills=true`**(`Outcome::must_say`)。読むのは**中身の scene 座標と窓の寸法**で、頼んだ margin ではなく着地の方。**両 OS が同じ答えを返す**(窓は frameless = 窓とクライアントが同一で、埋める相手は `merged` に関係ない)。**絵では読めない** — 最大化した窓は画面いっぱいなので、端が足りないことを見せる相手のデスクトップが隣に無い。**offscreen の最大化は 800×800** — 正は `at=0,0 size=800x800`(FAIL でも `screenshot saved=true` は出る)。**窓の縁が画面の外へ出るかはヘッドレスでは判定できない**(実モニタが無い)ので、縁の規則は実窓の画素実測が正本 = rules-refs/app-ui.md)

**窓がこれ以上小さくならないことは `window-floor` の 4 形**(引数なし / `fold` / `log` / `wip`。規約 §窓の床)。**引数なしは xtask が `--config-dir` に 320×240 の `state.toml` を仕込んでから起動する** — 床より小さい形はアプリ自身には作れない(アプリは自分が取れない形を書き留めないため)ので、「床が無かった頃のファイル」は外から置くしかない。`fold` は畳んだ床へ窓を降ろしてから一覧を戻し、`log` は panes だけの床へ降ろしてからログを開く = **どちらも「立っている窓の下で床が上がる」**側を見る。報告行 `window_floor fits= floorW= floorH= w= h= from= folded= log=` で、**合格条件は `fits=true`**(`Outcome::must_say`。床に立った窓と床を割った窓は**同じ絵に収まる** — 割った側は写真の外にペインがあるだけなので、写真では答えられない)。**`from=` が「一度は降ろした」の証拠**で、これが無いと `fold` は最初から広かった run と区別が付かない。`wip` は右ペインを WIP に切り替えてから床へ降ろし、**`wipScrolls=true`**(= 出口カードは下にあるが**届く**)を見る — `--preset rebase-conflict` を当てる。実測(Windows / offscreen): `floorW=824 floorH=245`、畳むと 688、ログを開くと `floorH=369`。**詳細ペインの側は `details-fit` の `overH=` で読む**(判定には入らない相乗り。壁は `--preset edges` の行 7 を**床の窓**で開く = `--config-dir` に床より小さい `state.toml` を仕込むと起動が床まで持ち上げる。実測: 包む前 `overH=160` / 包んだ後 `overH=0`)。

**帯の状態が 3 つ同時に立つ形は `badges` / `badges-hover` の対**(前者の引数は**窓の幅** — 省略で既定の 1440 / `floor` で床 / 数値でその幅。後者が hover でカードを開く)。**群は 3 段で譲る**(規約 §ウィンドウの縁)ので、**3 形とも撮る**。**同じ幅で同じ形になるとは限らない**のが要点 — タブ列と群が帯の不足を分け合うので、**タブの本数と名前の長さで境目が動く**(OS でも動く: Linux は帯が窓ボタンも掴み代も持たないぶん粘る)。**形は絵ではなく報告行で読む**(`words=` / `mark=` / `cap=`)。実測 2026-08-11(Windows / 長い名前 3 タブ): 1200 = 群 cap 80 / 1050 = 55 / 980 = 45(語の最後の一段)/ 930 = `…` / 880 = タブも下限。**`--preset` 1 つ**(タブ 1 枚)なら群だけが縮み、床 821 でも語が残る。**印だけの形は床より狭い所にしかない**ことがあるので、この動詞は指定された幅を床でクランプしない(`fits=false` はそこでは正 = 判定にも入れていない)。**どちらも `--preset cherry-pick-conflict` が要る** — 進行中の操作と conflict が同時に要り、素の preset ではどちらも立たないので `must_say` に届かない(引数なしで撃つと `op=false conflicts=false` で FAIL する = それが正しい振る舞い)。3 つ目の identity は xtask が**空の gitconfig を隔離して**渡し、`PG_AUTO_IDENTITY=skip` がダイアログを `Not now` で閉じる = 状態だけが残る(identity 系と同じ隔離なので、この機械の設定は読まない)。報告行 `badges fits= op= conflicts= identity= words= mark= cap= groupW= card= rows= cardSize= bandW= floorW= w= tabsW= grabRun=` を両方が出し、**`badges` は `op=true conflicts=true identity=true`・`badges-hover` は `card=true rows=op,conflicts,identity` が合格条件**(`Outcome::must_say`)。**どの形になったかは判定に入れない** — それは渡した幅が決めることで、引数が名乗っている。**`words=` / `mark=` は行と印それぞれの側・`cap=` は縮んだ幅**(-1 = どれも縮んでいない)で、**`rows=` はカードが実際に並べた行** — 1 行のカードと 3 行のカードは帯へ寄せて切り出すと同じ絵になる。**カードは overlay.png に写る**(Popup)。**`identity-tip` も同じカードを読む**(`tip=` はカードの `opened`)。**畳んだ後は帯 646 で床は動かない**(実測 2026-08-11)が、**両 OS 撮るのは変わらない**(フォントが違えばカードの中の 3 行も別々に組み上がる)。

**最低バージョンを割った git は `old-git` / `old-git-card` の対** — **状態を真似るフラグは無く、本物の古い git を建てる**(`solo` が本物のロックを握るのと同じ形)。xtask が**自分自身の exe を shot dir の `gitshim/git`(Windows は `git.exe`)へコピーして子の PATH の先頭に置き**、`PG_SHIM_GIT_VERSION` / `PG_SHIM_REAL_GIT` を渡す。そのコピーは `--version` にだけ古い版を答え、**残りは全部本物の git へ素通しする**(`verify::git_shim`)ので、**アプリは普通に動いたまま**バッジだけが立つ = リポジトリの入っている絵が撮れる。版は 2 動詞が自分で持つ(`2.42.0` — 最低バージョンは上がる一方なので、この数字は将来も割ったままでいる)。**他の動詞にも `--old-git <版>` で着せられる**(`badges` に足せば 4 つ目が並ぶ)。**rustc も .bat も要らない**のが要点で、コピー元は今走っているそのバイナリ。報告行 `old-git badge= card= rows= words= mark= tint= cap= version= min= w=` の **`badge=` は帯自身の読み**(条件ではない)で、**`old-git` は `old-git badge=true`・`old-git-card` は `old-git badge=true card=true` が合格条件**(`Outcome::must_say`)— **仕込みが PATH に届かなかった run は普通の窓を撮り、普通の窓は普通に撮れてしまう**。**`rows=` は判定に入れない** — 他の 3 行はその機械の事情で立つ(identity を設定していないコンテナでは `SET IDENTITY` も並ぶ)。**カードは overlay.png の側**。**3 つ目の `old-git-fold` は畳んだ姿**(`badges` と同じ引数を `old-git` も取るが、こちらは床を自分で持つ)で、合格条件は **`mark=true tint=warning`** — **黄だけが立っている窓で `…` が赤くなっていないこと**が見物(`tint=` は帯が実際に塗った色を `danger` / `warning` の名前で言う = 規約 §状態「色は最も重い状態が決める」が塗りまで届いたか。**hex では書かない** — 判定にトークンの値の写しを持ち込まない)。**赤い方は `badges floor --preset cherry-pick-conflict`** が撮る(同じ `tint=` が `badges` の報告行にも在る)。**畳んだ絵は色以外に何も言わない**ので、対で読まないと「黄になった」も「赤のままだった」も同じ 3 点に見える。**git が無い側(全画面のゲート)には動詞が無い** — PATH から git を外した起動が要るので、xtask の `path_with_qt` と噛み合わない(撮る時は exe を直に叩き、PATH に Qt の bin だけを置く)。

**設定 / 画面状態の永続化は 2 回の実行で読む**: `state`(引数 `change` で畳み・右ペイン幅・グラフのチップ列 / レーン列の幅・コマンドログ・auto fetch 間隔を動かしてから申告する)。**1 回の実行では何も証明できない** — 同じ `--config-dir` を渡した 2 回目に `--restore` を足すと、`PG_AUTO_OPEN` を渡さない = アプリが自分で憶えているタブを開く経路に入る。報告行 `state tabs= active= opened= collapsed= sidebar= details= graphLabels= graphLanes= commands= maximized= windowW= windowH= windowMax= autoFetch=` が両方で一致すれば往復している。**`window*` の 3 つだけはファイルの側**(`AppBackend.startWindow*()` の読み直し)で、他は窓とページの側 — **最小化した窓は申告しないので、窓の言い分とファイルの言い分が割れる場面がある**のがその理由。**引数 `minimize` がその場面**: 最大化 → 申告 → 最小化 → 申告 と走り、`windowMax=true` かつ `windowW/H` が最大化前のままなのが正(下りている窓は**窓の姿でも最大化の姿でもない数字**を名乗る = rules-refs/app-ui.md)。**最大化の申告を先に 1 度挟むのが要**で、無いと 2 度目の申告に守るものが無く、配線が外れていても緑になる。撮影のために最後は窓を上げ直す(下りた窓から `grabToImage` は何も返さない)。**`--config-dir` を渡さない実行は毎回まっさらな一時ディレクトリ**(shot dir の下)で走るので、動確が開発者の実設定を読むことも汚すこともない。**`PG_CONFIG_DIR` 以外の `PG_*` が立っていれば exe 自身もファイル無しに倒す**ので、xtask を通さず直接叩いても実設定は無事(実測: `settings store settings=None state=None`)。**タブの遅延ロードは `opened repository tab` の行数で読む** — 3 タブ復元して 1 本だけなのが正

**開いたまま止める撮影用**は settings-tools(設定ダイアログを開いて Merge editor の候補一覧を開いたまま止める。`merge_editor wanted= shown= configured=` を報告する。**候補は 2 波で来る** — 自前定義は即・同梱分は `--tool-help` が Windows で約 8 秒なので、**リングが回っている絵は既定の `--quit-ms` を 3000 程度に詰めて**、**出揃った絵は 14000 に伸ばして**撮る。demo repo の `.git/config` に `[mergetool "名前"] cmd = …` を 2 つほど書いておくと 1 波目に中身が入る)/ stash-dialog(引数 staged-only でそのチェックをクリック済みの状態)/ file-menu / file-menu-untracked / file-menu-staged / file-menu-conflict(引数はパス。バケツごとに出る行が変わるので 4 つ。conflict は種別の文言を `conflict_kind` として報告する — hover は注入できないため。`--preset conflict-kinds` に 4 種が揃っている)/ amend-author / settings / commit-menu / reset-menu / stash-menu / name-box(引数は行番号)/ ref-list / ref-list-card(引数は行番号。チップを展開したまま止める / **同じ行で「行のカードを出す → チップへ移る → 行がもう一度頼む」を通す** = 報告された所作そのもの。後者は `row-card` と**対で読む** — 同じ行・同じ preset で `row_card open=` が割れる(`ref-list-card` は `open=false list=true subject=true` / `row-card` は `open=true list=false`)のが「一覧が出ている間はカードは出ない」の答えで、**片側だけでは何も証明しない**。**`subject=true` が「カードは一度出た」を言う**ので、出したものを引っ込める側と後から断る側の両方が同じ 1 行で読める。`--preset tags` の行 0 が 3 段のチップを持つ)/ fetch-ref-list(引数は行番号。fetch を撃ってからその行のチップを展開する — タグの雲は fetch 後にしか出ない。`--preset tags` に 4 状態が揃っている)/ signature(引数は行番号。その行を選び、gpg / ssh-keygen が返すまで待って印を報告する。`--preset signed` に verified / signed / 無署名の 3 行がある)/ signature-tip(引数は行番号。同じ選択の後、印の ToolTip を実 hover と同じ attached visible で立て、判定の理由の 1 文を出したまま撮る。**`E` は `--preset errsig` の行 0 だけが持つ** — 埋め込みの OpenPGP 署名済みコミットで、鍵はどのキーリングにも無い。ssh 署名は設定をどう壊しても `N` / `U` / `B` にしかならず(実測)、**検証には gpg バイナリが要る**(無いと同じコミットが `N` になる。Windows は Git 同梱・Linux はイメージが積む)。報告行 `signature_tip code= tip=` の **`tip=` は ToolTip 自身の visible**(出力側)。**ツールチップは overlay.png の側に写る**)/ stash-tip(引数は行番号。stash の行を選ぶと詳細ペインの summary が読み取り専用になり、その箱の ToolTip が「改名は左の一覧で」を出す。`--preset stashes` は行 0〜2 が stash。報告行 `stash_tip blocked= tip=`。overlay.png の側に写る)/ path-tip(引数は `wip`(既定)か行番号 — `corner` と同じ形で**ペインを選ぶ**。`-tree` を足した `wip-tree` / `<行>-tree` はツリーのまま撮り、行 0 = **ペインが省略したフォルダ連鎖の行**が的になる(フォルダは省略だけが引き金 — 親フォルダ行が既に綴っている接頭辞は理由にならない)。素の形は paths view へ切り替え、先頭の行を `pointedTipRow` で名指しして、**省略された行のフルパスのツールチップ**を立てたまま撮る。**`--preset longpaths`** = どのペイン幅でも省略される長さのパスを 1 つ、コミットと作業ツリーの両方に持つ(行 0 = WIP・行 1 = そのコミット — details 側は `path-tip 1`)。報告行 `path_tip pane= tree= tip= text=` の **`tip=` / `text=` は共有インスタンス自身の visible と文言**(出力側)。`must_say` は `tree=false tip=true`(`-tree` 付きは `tree=true tip=true`)— paths view では表示名 = フルパスなので、**tip が立ったこと自体が「省略が引き金になった」の証明**になる。ツールチップは overlay.png の側に写る)/ author-card / author-card-open(引数は行番号。その行を選んで author 名の平常時を撮る / さらに名前のカードを開く。**hover は注入できない**ので `-open` は `DetailsPane.showAuthor` = 実 hover と同じ 1 つのプロパティへ書く。報告行は `author_card open= author= committer= other= later=` で、**`open=` はカード自身の可視性**。**2 つは対で読む**(閉じたままが正の側だけでは何も証明しない)。**`--preset authorship` が 4 状態を行 0〜3 に並べる**: 行 0 = 別人が 3 日後に置いた(`other=true later=true`)/ 行 1 = 別人が同じ瞬間に置いた(forge の squash 相当。`other=true later=false`)/ 行 2 = 同じ人が 2 日後に置いた(amend / rebase 相当。`other=false later=true`)/ 行 3・4 = 普通のコミット。**日時をずらすのは `git commit --date`**(`--date` が demo repo の `GIT_AUTHOR_DATE` に勝ち、committer 側は行進する stamp のまま = 差が正確に出る)) / co-authors / co-authors-open(引数は行番号。その行を選んで日付行の credit 表示を撮る / さらにカードを開く。**hover は注入できない**ので `-open` は `DetailsPane.showCoAuthors` = 実 hover と同じ 1 つのプロパティへ書く。報告行は `co_authors count= first= open=` で、**`open=` はカード自身の可視性**(入力側を報告するとバインディングが切れていても緑になる)。**2 つは対で読む** — 閉じたままが正の側だけでは何も証明しない。/ row-card(引数は行番号。その行の hover カードを開く。**hover は注入できない**ので、行の遅延タイマが上げるのと同じ `rowHoverRequested(row, true)` を叩く(**ページの関数を直接呼ばない** — 出す / 出さないの判定はページ側に在るので、迂回すると出さないはずの場面まで開いて緑になる)。報告行は `row_card open= credit= cut= list= subject= body=` で、**`open=` は Popup 自身の `opened`**、**`credit=` は credit 行に実際に配られた幅**(0 = 誰も credit していないコミット)、**`cut=` は名前が溢れて省略された**(規約 §co-author の表示 の幅の規則はこの 2 つでしか読めない — 省略記号は PNG でしか見えず、カードの幅は PNG からは測れない)、`list=` はチップの一覧 = `ref-list-card` が読む側。`--preset co-authors` の行 4 は body の無いコミットなので `body=false`、行 7 は誰も credit していないので `credit=0` が正)。`--preset co-authors` は**人数と署名の全組み合わせ**: 行 0 = 1 人 + `G` / 行 1 = 1 人 + **`B`**(署名後に tree を差し替えてある — `%G?` が `G` → `B` になるのを実測して作った唯一の経路)/ 行 2 = 1 人 + **`U`**(保証されていない鍵)/ 行 3 = 1 人 + `G` + **25 字の author 名**(名前が縮んで印が残るのを見る席)/ 行 4 = 3 人 + `G` / 行 5 = 3 人 / 行 6 = 1 人 / 行 7 = 0 人。**署名の 3 段はこの preset だけが全部持つ**(`--preset signed` は co-author を持たない))/ details-fit(引数は行。**詳細ペインの列がペインに収まっているか**。`Layout.fillWidth` を書いていない子は Fixed = 縮まない床なので、譲らない行が 1 つあると列全体がその床の幅で並び、`fillWidth` の枠と一覧がその幅を受け取って**窓の右端でグリフを半分に切って**描く。報告行 `details_fit fits= over= pane=` の **`fits=true` は合格条件**(`Outcome::must_say`)— **溢れた絵は溢れていない絵と同じ枠に収まる**ので、写真では答えられない。壁は **`--preset edges` の行 7**(2000 字の subject + 120 バイトの co-author 名))/ details-grow / details-grow-squeeze / wip-grow / wip-grow-squeeze(**description 枠の右下の掴みを、ペインが出せる分より深く引く**。素の 2 つはそこで止め、`-squeeze` はその後コマンドログを上げて**ペインの方を縮める** = 手が広げた分を返す側。**引数は詳細ペインが行番号・commit 欄は打ち込む本文**(省略時は xtask が長い本文を入れる — 空の欄には開けるものが無いため)。**報告行は 4 つとも 1 本**の `description_grow keeps= pane= grip= box= wants= cap= rows=` で、**`keeps=true` が合格条件**(`Outcome::must_say`。溢れた列は著者行 / commit ボタンを窓のフッタの上に描くが、その絵は溢れていない絵と同じ枠に収まる = `details-fit` と同じ盲点)。**`grip=` は must_say に入れない** — 掴みが出ない run が対の片側だから(**`--preset edges` の行 7 = 出る側 / 行 4 = 本文が無いので出ない側**)。`rows=2` が下限そのもの。**PNG は必ず目視する** — 数字が全部正しいまま**枠の中の本文だけが元の高さで途切れる**形があり(TextEdit のビューポート再描画。rules-refs/app-ui.md)、それは絵にしか出ない)/ nav-rename / rename-remote-box(引数 `<remote>/<old>:<入れておく名前>`。畳みを開いて入力欄を出す — 既にリモートに在る名前を渡せば拒否された枠が撮れる)

**アバターは `avatar-*`**(**引数は取り込む画像の `file:` URL** — 保管庫は実行ごとに空なので、見るものはまず入れる): `avatar-rest`(顔が identicon のまま)/ `avatar-hover`(ペンのバッジを出す — hover は注入できないので `DetailsPane.avatarPointedAt` を立てる)/ `avatar-assign`(選んだコミットの著者へ引数の画像を取り込み、**報告行 `avatar email= details= rows= error=`**。`rows=` は **Rust 側が数える**グラフ行の枚数 = 顔がグラフまで届いたかを言う。QML から数えると**画面に出ていても 0 になる**)/ `avatar-badge`(バッジを押して設定カードを開く = 入力欄にその人が入り `Choose avatar…` に焦点)/ `avatar-settings` / `avatar-row-lit`(一覧の 1 行目にポインタを置いた形 = `Remove` が明るい側。置かない側が `avatar-settings`)/ `avatar-remove`(その行の長押しを走らせる)/ `avatar-combo`(候補の一覧を開く)。**往復は 2 回の実行で読む** — `--config-dir` を共有し、1 回目に `avatar-assign`、2 回目は**何も割り当てない `avatar-settings`** で顔が出ていれば `settings.toml` の `[[avatar]]` と `avatars/` から戻っている。**画像の実体は中身のハッシュ名**(`avatar` モジュール)なので、ファイル名は毎回同じ = 絵は再現する。


## 通信中(リング)と起動直後の狙い方

**通信中(リング)は `--quit-ms` で狙う**: 撮影は `quit-ms - 800`、動詞は 1200ms の
固定タイマで走る(`Main.qml` の `shotTimer` / `RepoPage.qml` の `autoActTimer`。どちらも
ほぼ同じ瞬間から数え始める)ので、**その差が撮影窓**。既定の 10000 では通信はとうに
終わっている。リングは `PG_SHOT_DIR` があると止まる(`ActionButton.still`)ので毎回
同じ絵になる。実測で当たった値(2026-08-07):

| 撮りたい状態 | 動詞 | `--quit-ms` |
|---|---|---|
| fetch 通信中(通常) | `fetch` | 2100 |
| fetch 通信中(1〜2 回失敗の黄) | `fetch-resume` | 2450 |
| fetch 通信中(停止後の赤) | `fetch-fail 30` | 2600 |
| push 通信中 | `push` | 2100 |
| force push 通信中 | `force-push-hold` | 3400 |

`file://` の fetch は 50〜150ms しか走らないので、**窓は数十 ms**。外したら 10〜20ms
刻みで振る(**当たり外れは PNG を見るまで判らない** — stderr の `screenshot saved` の
時刻は grab のコールバックが走った時刻で、掴んだ瞬間より後)。**黄は `fetch-fail 2` では
撮れない**(失敗が記録されてから次の fetch が busy になるまでの間に必ず 1 フレーム
入る)。`fetch-resume` なら 3 回失敗させた後の resume の fetch が「失敗数 > 0 かつ
停止解除済み」= 黄のまま走るので、そこを狙う。**赤は逆に何回でも撃てる**(3 回で停止
した後も `fetch-fail N` は N まで走り続け、その間ずっと赤 + busy)。

**起動直後の状態も同じ窓で狙える**: 撮影は `--quit-ms 801〜3499` で「quit-ms − 800」ms
に発火する(801 = 起動後 1ms。**800 以下は 3500ms 固定に化ける**)。グラフ初回ロードの
中央リング(`loading && rowTotal === 0`)は demo repo でも first chunk が ~74ms あるので
`--quit-ms 850` で自然に撮れる(2026-08-08 実測。サイドバーの件数がまだ 0 なのが
ロード途中の証拠)。**ゲート(git 不在 / もう 1 つ起動していた)も撮れる** — 撮影は
出ている方を掴む(`gate.visible ? gate : mainUi`)。

**ヘッドレスで色を確かめる時はデモリモートの URL を疑う** — 届かないリモートを使う
検証(`fetch-fail` / `fetch-resume` / 黄の `push`)で通信が成功してしまうと失敗の記録が
消え、黄も赤も出ない(逆に `fetch-recover` は**回復側が届く origin に乗っている** —
塞いだままだと成功が来ず、退く絵が撮れない)。実験で `remote set-url` を触ったら戻すこと。**URL は demo repo の
`.git/config` を直接書き換えるのが早い**(`git -C` が通らない worktree セッションでも
届く)。`--preset diverged` は `push -f` の形をそのまま出すので、**枠のある状態と
`!` の同居**はこれで撮る。
