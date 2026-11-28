import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude
import platitude.ui
import platitude.ui.words

// The graph row's real `CommitRowMenu`, standing on the answers its one door reads (`CommitMenuState`): which rows
// are there, what each says, which kind the move is aimed with, and what a press asks for. What a state allows is
// the menu type's (`menu::lines`) and tested there.
//
// No picture tells an offered row from one that is not once the list closes over them, nor carries the kind a
// press was sent with.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the menu runs nothing itself.
    property string asked: ""
    /// Core's answer for a stand (`GitFacts.menuOn` — `encode::MenuAnswer`): `lines` the rows there are, each to why it
    /// greys (core's words, "" while live) in drawing order, and `fields` over the rest of the record — what a press
    /// sends.
    function menuOf(lines, fields) {
        const rows = Object.keys(lines || {})
        const all = { "rows": rows, "whys": rows.map(row => lines[row]), "switchAsks": false, "upstreamWhere": "",
                      "pullRuns": "", "worktreeAdd": "" }
        for (const key in (fields || {}))
            all[key] = fields[key]
        return GitFacts.seated(all)
    }
    /// The first level's rows on a commit row aimed at a branch the tree is not on, live, as core's type answers them
    /// (`CommitMenuState.lines`): the commit's, `switch` and `pull` — and the TAG card's making row, which the first
    /// level lends it.
    readonly property var commitLines: ({ "branch-here": "", "reach": "", "cherry-pick": "", "revert": "",
                                          "squash": "", "drop": "", "rebase-interactive": "", "merge": "",
                                          "rebase": "", "reset-soft": "", "reset-mixed": "", "reset-hard": "",
                                          "pull": "", "tag-here": "" })
    /// What the menu's rows read of that answer (`MenuLines`), written by `aimAt`.
    readonly property MenuLines lines: MenuLines {}
    /// The rows a branch nothing holds lends the cards and the first level, tracking its reading: its delete table and
    /// `Set upstream…`, live.
    readonly property var branchLines: ({ "delete-branch": "", "delete-remote-branch": "", "delete-both": "",
                                          "set-upstream": "" })
    /// What the cards stand on (`RefMenuFacts.stand`) for a row drawing a branch nothing holds, tracking
    /// `origin/feature/topic-a`; `fields` replaces any part of it — the card's record, core's answer, the lead.
    function stand(fields) {
        const all = {
            "lead": { "kind": "branch", "full": "feature/topic-a", "local": "feature/topic-a", "leads": "",
                      "path": "", "called": "", "home": false, "cardHere": false, "placing": "free" },
            "menu": root.menuOf(root.branchLines, { "upstreamWhere": "remote" }),
            "card": { "remoteCounterpart": "origin/feature/topic-a", "remoteCounterpartOid": "abc123",
                      "upstream": "origin/feature/topic-a" },
            "oid": "abc123",
            "push": undefined,
            "merged": "yes",
            "tagAt": ""
        }
        for (const key in (fields || {}))
            all[key] = fields[key]
        return all
    }
    /// A stand whose WORKTREE card makes what `fields` says (`RefWorktreeMenu.standOn`), and whose other cards hold
    /// nothing: `here` (a `Create worktree here…` row), `checkout` (a `worktree add` row, how it takes the branch out)
    /// and `out` (why that greys — with none, `taken` where something is in the way) as core answers them; `branch`
    /// (the local branch the row lands on), `start` (the remote branch a tracking one is made off) and `holderLeaf`
    /// (the worktree holding it) as the row's lead says them, with `path` / `place` / `taken` as the place it would
    /// go — none without a `checkout`.
    function making(fields) {
        const all = Object.assign({ "here": false, "checkout": "", "out": "", "branch": "", "start": "",
                                    "holderLeaf": "", "path": "", "place": "", "taken": "" },
                                  fields)
        return root.stand({
            "lead": { "kind": all.start !== "" ? "remote" : "branch",
                      "full": all.start !== "" ? all.start : all.branch, "local": all.branch, "leads": "",
                      "path": "", "called": all.holderLeaf, "home": false, "cardHere": false, "placing": "free",
                      "place": all.checkout === "" ? undefined
                             : { "path": all.path, "name": all.place, "taken": all.taken } },
            "menu": root.menuOf(Object.assign(all.here ? { "worktree-here": "" } : {},
                                              all.checkout === "" ? {}
                                              : { "worktree-add": all.out !== "" ? all.out
                                                                  : all.taken !== "" ? "taken" : "" }),
                                { "worktreeAdd": all.checkout }),
            "card": { "remoteCounterpart": "", "remoteCounterpartOid": "", "upstream": "" },
            "merged": ""
        })
    }

    CommitRowMenu {
        id: menu
        branch: "main"
        oid: "abc123"
        stashRef: ""
        published: false
        rebasePublished: false
        lines: root.lines
        tipHeldElsewhere: false
        switchAsks: false
        heldLeaf: ""
        pullRuns: "here"
        pullUpstream: "origin/feature/topic-a"
        hardResetTakes: 0
        hardResetNotCopied: false
        onSwitchRequested: (kind, name) => root.asked = "switch " + kind + " " + name
        onCherryPickRequested: oidHex => root.asked = "cherry-pick " + oidHex
        onRevertRequested: oidHex => root.asked = "revert " + oidHex
        onApplyStashRequested: selector => root.asked = "apply " + selector
        onPopStashRequested: selector => root.asked = "pop " + selector
        onMergeRequested: ref => root.asked = "merge " + ref
        onRebaseRequested: ref => root.asked = "rebase " + ref
        onPullRequested: branch => root.asked = branch === "" ? "pull" : "pull " + branch
        onResetRequested: mode => root.asked = "reset " + mode
        onPushTagRequested: (remote, tag, expect) => root.asked = "push " + remote + " " + tag + " " + expect
        onDeleteRemoteTagRequested: (remote, tag, rowGoes, expect) =>
            root.asked = "delete-remote-tag " + remote + " " + tag + " " + expect
        onDeleteRemoteRequested: (remoteRef, expect) => root.asked = "delete-remote " + remoteRef + " " + expect
        onWorktreeHereRequested: oidHex => root.asked = "worktree-here " + oidHex
        onCarriedOpenRequested: path => root.asked = "carried-open " + path
        onCarriedApplyRequested: path => root.asked = "carried-apply " + path
        onWorktreeAddRequested: (mode, branch, start, path, name) =>
            root.asked = "worktree-add " + mode + " " + branch + " " + start + " " + path + " " + name
    }

    /// How often the menu stood again, and as what (`MenuLines.standAgain`): the page answers it.
    SignalSpy {
        id: stood
        target: root.lines
        signalName: "standAgain"
    }

    /// The row wearing that command, read the way the menu reads itself (`AppMenu.offeredRows`), so a row nothing
    /// declares comes back undefined rather than quietly passing.
    function row(code) {
        for (let i = 0; i < menu.menu.count; i++) {
            const item = menu.menu.itemAt(i)
            if (item && item.code === code && item.codeColSeat !== undefined)
                return item
        }
        return undefined
    }

    /// A level's own rows on show — not the cards' title rows, which open their cards whatever the rows inside say —
    /// and how many of them grey.
    function rowsOf(level) {
        let rows = 0
        let greyed = 0
        for (let i = 0; i < level.count; i++) {
            const item = level.itemAt(i)
            if (!item || item.codeColSeat === undefined || !item.present || item.subMenu)
                continue
            rows++
            if (item.blocked)
                greyed++
        }
        return { "rows": rows, "greyed": greyed }
    }

    /// The first level's title row a card hangs off.
    function titleOf(sub) {
        for (let i = 0; i < menu.menu.count; i++) {
            const item = menu.menu.itemAt(i)
            if (item && item.subMenu === sub)
                return item
        }
        return undefined
    }

    /// What the first level shows, in order: each row's command, its words where it has none, a card's title.
    function shownOrder() {
        const shown = []
        for (let i = 0; i < menu.menu.count; i++) {
            const item = menu.menu.itemAt(i)
            if (!item || item.codeColSeat === undefined || !item.present)
                continue
            shown.push(item.subMenu ? item.subMenu.title : item.code !== "" ? item.code : item.text)
        }
        return shown
    }

    /// `fields` names only what this case is about; its `lines` changes core's answer (`commitLines`) row by row — a
    /// word greys the row for that reason, `null` takes it away.
    function aimAt(fields) {
        const all = { "targetKind": "branch", "targetName": "feature/topic-a", "switchAsks": false, "heldLeaf": "",
                      "pullRuns": "here", "pullUpstream": "origin/feature/topic-a", "published": false,
                      "rebasePublished": false, "hardResetTakes": 0, "hardResetNotCopied": false }
        const rows = Object.assign({}, root.commitLines)
        for (const key in fields) {
            if (key !== "lines")
                all[key] = fields[key]
        }
        for (const row in (fields.lines || {})) {
            if (fields.lines[row] === null)
                delete rows[row]
            else
                rows[row] = fields.lines[row]
        }
        for (const key in all)
            menu[key] = all[key]
        root.firstRows = rows
        root.drawn()
        root.asked = ""
    }
    /// The first level's own rows (`aimAt`) and the cards' and the rows they lend it (the facts `offer` and
    /// `standCards` stand the cards on): core answers both in one answer, which the menu is drawn by (`MenuLines`).
    property var firstRows: ({})
    property var cardRows: ({})
    function drawn() {
        root.lines.answer = root.menuOf(Object.assign({}, root.cardRows, root.firstRows))
    }
    function offer(facts) {
        menu.standFresh("commit")
        root.cardRows = root.rowsIn(facts.menu)
        root.drawn()
        menu.offerCommit(facts)
    }
    function standCards(facts, branchAt, again) {
        root.cardRows = root.rowsIn(facts.menu)
        root.drawn()
        menu.standCards(facts, branchAt, again)
    }
    /// An answer's rows, each to why it greys — what `menuOf` takes.
    function rowsIn(answer) {
        const rows = {}
        for (const line of answer.lines)
            rows[line.word] = line.why
        return rows
    }

    TestCase {
        name: "CommitRowMenu"
        when: windowShown

        // Every case starts on the commit's level, fresh, as `CommitMenuState.openRowMenu` stands one.
        function init() {
            menu.standFresh("commit")
        }
        function cleanup() {
            menu.menu.dismiss()
            menu.runningWhy = ""
            menu.heldReason = ""
            root.lines.waiting = false
            root.lines.release()
            stood.clear()
        }
        /// A command out and its end as the page says them (`RepoPage.onRunningWhyChanged`): the rows' reason and the
        /// tree it was asked on (`MenuLines.waiting`).
        function startCommand() {
            menu.runningWhy = Words.otherCommandRunning
            root.lines.waiting = true
        }
        function endCommand() {
            menu.runningWhy = ""
            root.lines.waiting = false
        }

        /// The move row is there exactly when core says so (`menu::Row::Reach`); a move that cannot be
        /// made has no row, not a grey one.
        function test_the_move_row_follows_the_word_it_was_handed_data() {
            return [
                { tag: "a branch the tree is not on", fields: {}, offered: true, asks: false },
                { tag: "the branch the tree is on", fields: { lines: { "reach": null } }, offered: false, asks: false },
                {
                    tag: "one with an operation standing",
                    fields: { switchAsks: true },
                    offered: true,
                    asks: true,
                },
                {
                    tag: "one another worktree holds",
                    fields: { heldLeaf: "topic" },
                    offered: true,
                    asks: false,
                },
            ]
        }

        function test_the_move_row_follows_the_word_it_was_handed(data) {
            root.aimAt(data.fields)
            compare(menu.switchItem.offered, data.offered)
            compare(menu.switchItem.asks, data.asks, "the mark says a question is coming before it is asked")
            compare(menu.switchItem.blockedReason, "", "a move that cannot be made is not a greyed row")
        }

        /// The press opens that worktree and asks nothing, so the words are all that is read before it
        /// (offers::SwitchAction::OpenHolder). The mark is the WORKTREES one that worktree wears everywhere.
        function test_a_held_branch_names_the_worktree_instead_of_a_command() {
            root.aimAt({ heldLeaf: "topic" })
            compare(menu.switchItem.code, "", "there is no git command for opening a worktree")
            compare(menu.switchItem.text, "Open")
            compare(menu.switchItem.codeName, "topic")
            compare(menu.switchItem.nameMark, "tree")
            verify(menu.switchItem.codesName)
            compare(menu.switchItem.tip.words, "Open topic", "a cut folder comes back whole on the hover")
            compare(menu.switchItem.tipMarks, [["Open ".length, "tree", undefined, "topic".length]],
                    "behind the mark it wears on the row")
            root.aimAt({})
            compare(menu.switchItem.code, "switch", "an ordinary move is the command it runs")
            verify(!menu.switchItem.codesName)
            compare(menu.switchItem.tipMarkWord, "")
        }

        /// Which kind the press is sent as is this file's alone, and the two roads part on it
        /// (`offers::switch_action`).
        function test_the_move_is_aimed_with_the_kind_the_chip_drew() {
            root.aimAt({})
            menu.switchItem.triggered()
            compare(root.asked, "switch branch feature/topic-a")

            root.aimAt({ targetKind: "remote", targetName: "origin/feature/topic-a" })
            menu.switchItem.triggered()
            compare(root.asked, "switch remote origin/feature/topic-a")
        }

        /// What this file decides about `pull`: where git would turn the press down the row stays and says why
        /// (デザイン規約 §メニュー), and a press that moves a branch aside carries the chip's name.
        function test_the_pull_row_is_there_on_the_word_and_grey_on_the_answer() {
            root.aimAt({})
            verify(root.row("pull").offered)
            compare(root.row("pull").blockedReason, "")
            compare(root.row("pull").text, "into main", "the row names the branch that moves")
            compare(root.row("pull").refTint, Theme.textLink, "in the current branch's colour")
            root.row("pull").triggered()
            compare(root.asked, "pull", "the worktree's own: git resolves the branch and its far side")

            root.aimAt({ lines: { "pull": "diverged" } })
            verify(root.row("pull").offered, "the row stays: it is the reader's question, not a missing one")
            compare(root.row("pull").blockedReason, Words.pullDiverged)
            root.aimAt({ lines: { "pull": "standing" } })
            verify(root.row("pull").offered, "a stopped operation greys the row, it does not take it away")
            compare(root.row("pull").blockedReason, "Finish the stopped operation first")

            root.aimAt({ pullRuns: "aside" })
            compare(root.row("pull").text, "into feature/topic-a", "aside, the chip's own branch is the one that moves")
            compare(root.row("pull").refTint, Theme.textPrimary, "in a local branch's colour")
            root.row("pull").triggered()
            compare(root.asked, "pull feature/topic-a", "a branch the worktree is not on is named to the press")

            root.aimAt({ pullRuns: "aside", lines: { "pull": "own" } })
            compare(root.row("pull").blockedReason,
                    "Cannot fast-forward — it has commits not on origin/feature/topic-a", "naming the upstream")
            root.aimAt({ pullRuns: "aside", heldLeaf: "topic", lines: { "pull": "held" } })
            compare(root.row("pull").blockedReason, Words.checkedOutElsewhere("topic"))
            compare(root.row("pull").reasonMarkWord, "topic", "the holder's folder wears the tree mark in the tip")

            root.aimAt({ pullRuns: "", lines: { "pull": null } })
            verify(!root.row("pull").offered, "a row with no far side to go to is no row")
        }

        /// Under a stopped operation every row that starts or rewrites history keeps its seat and says what comes
        /// first, on core's word (`menu::Why::StoppedOperation`) — the reset card's each, its title greying with them
        /// and still opening to show them — and a warning goes with the press it warned of. The move stays live: it
        /// asks first.
        function test_a_standing_operation_greys_every_row_that_starts_or_rewrites_history() {
            const standing = {}
            for (const row of ["branch-here", "cherry-pick", "revert", "squash", "drop", "rebase-interactive", "merge",
                               "rebase", "reset-soft", "reset-mixed", "reset-hard", "tag-here"])
                standing[row] = "standing"
            root.aimAt({ published: true, rebasePublished: true, hardResetTakes: 2, lines: standing })
            root.offer(root.stand())
            const resetTitle = root.titleOf(menu.resetSubmenu)
            verify(resetTitle.greyed, "the reset card's title greys with every row inside")
            verify(!resetTitle.blocked, "with no reason of its own: it still opens the card")
            verify(menu.menu.openSub(menu.resetSubmenu))
            for (const item of [menu.branchHereItem, root.row("cherry-pick"), root.row("revert"), root.row("squash"),
                                root.row("drop"), root.row("rebase --interactive"), root.row("merge"),
                                root.row("rebase"), menu.tagHereItem, menu.hardResetRow]) {
                verify(item.offered, item.code + " " + item.text + " keeps its seat")
                compare(item.blockedWhy, Words.finishStoppedFirst, item.code + " " + item.text)
                compare(item.tag, "", "a greyed row warns of nothing")
            }
            compare(menu.switchItem.blockedWhy, "")

            root.aimAt({ published: true, rebasePublished: true, hardResetTakes: 2 })
            compare(root.row("merge").blockedWhy, "")
            compare(menu.hardResetRow.blockedWhy, "")
            verify(!resetTitle.greyed, "the title comes back with the rows")
            compare(root.row("rebase").tag, Words.rewritesPushed)
            compare(menu.hardResetRow.tag, "uncommitted too")
        }

        /// A row greyed by what the menu holds back, which core's answer leaves live, warns of nothing either: the
        /// warning goes with the press it warns of, whatever greys the row (`AppMenuItem.greyed`), and comes back with
        /// it.
        function test_a_row_the_menu_holds_back_warns_of_nothing() {
            root.aimAt({ published: true, rebasePublished: true, hardResetTakes: 2 })
            root.offer(root.stand())
            verify(menu.menu.openSub(menu.resetSubmenu))
            const warned = [root.row("squash"), root.row("drop"), root.row("rebase --interactive"),
                            root.row("rebase"), menu.hardResetRow]
            for (const item of warned)
                verify(item.tag !== "", item.code + " warns while it can be pressed")
            menu.heldReason = Words.otherCommandRunning
            for (const item of warned) {
                verify(item.greyed, item.code + " greys")
                compare(item.tag, "", item.code + " warns of nothing")
            }
            menu.heldReason = ""
            for (const item of warned)
                verify(item.tag !== "", item.code + " warns again")
        }

        /// On HEAD's own commit `merge` and `rebase` bring nothing in and the reset card takes the branch nowhere: each
        /// greys saying its own, on core's word (`menu::Why::SameCommit` / `AlreadyThere`), the card's title greying
        /// with its rows and still opening to show them.
        function test_heads_own_commit_greys_merge_rebase_and_reset_each_for_its_own() {
            const nothingIn = "Nothing to bring in — this is the commit you are on"
            root.aimAt({ lines: { "merge": "same-commit", "rebase": "same-commit", "reset-soft": "already-there",
                                  "reset-mixed": "already-there", "reset-hard": "already-there" } })
            root.offer(root.stand())
            compare(root.row("merge").blockedWhy, nothingIn)
            compare(root.row("rebase").blockedWhy, nothingIn)
            verify(root.titleOf(menu.resetSubmenu).greyed, "the reset card's title greys with every row inside")
            verify(menu.menu.openSub(menu.resetSubmenu))
            compare(menu.hardResetRow.blockedWhy, "Nothing to move — this is the commit you are on")
        }

        /// The branch HEAD is on with no commits yet: each row that needs one there greys naming it, on core's word
        /// (`menu::Why::HeadUnborn`), the name in the reason wearing a branch's mark as the row's own words wear it.
        function test_a_branch_with_no_commits_yet_is_named_in_the_reason_with_its_mark() {
            const unborn = { "revert": "head-unborn", "squash": "head-unborn", "drop": "head-unborn",
                             "rebase-interactive": "head-unborn", "rebase": "head-unborn" }
            root.aimAt({ lines: unborn })
            for (const code of ["revert", "squash", "drop", "rebase --interactive", "rebase"]) {
                const item = root.row(code)
                compare(item.blockedWhy, "main has no commits yet", code)
                compare(item.tipMarkWord, "main", code + ": the name in the reason wears its mark")
                compare(item.tipMarkKind, "branch", code)
                compare(Words.tipNameSeat(item.blockedWhy, item.tipMarkWord), 0,
                        code + ": the tip seats it where the reason opens with it")
            }
            root.aimAt({})
            compare(root.row("revert").tipMarkWord, "", "a live row has no reason to seat a mark in")
        }

        /// The press carries the chip's name, not the id: a merge of `main` says so in the commit it writes.
        function test_the_rows_that_join_two_lines_name_both_ends() {
            root.aimAt({})
            compare(root.row("merge").text, "into main")
            compare(root.row("rebase").text, "main onto it")
            root.row("merge").triggered()
            compare(root.asked, "merge feature/topic-a")

            root.aimAt({})
            root.row("rebase").triggered()
            compare(root.asked, "rebase feature/topic-a")

            // A row drawing no name falls back to its commit.
            root.aimAt({ targetKind: "", targetName: "" })
            root.row("merge").triggered()
            compare(root.asked, "merge abc123")
        }

        /// The rewrite rows wear the tag and stay a plain click (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」),
        /// each on the commits it rewrites: from here up, the pressed commit and those above it; onto it, the ones the
        /// rebase replays (`<row>..HEAD`), which the pressed commit is not among.
        function test_a_pushed_commit_is_told_about_and_not_held() {
            root.aimAt({})
            compare(root.row("rebase").tag, "")
            compare(root.row("rebase --interactive").tag, "")

            root.aimAt({ published: true })
            compare(root.row("rebase --interactive").tag, Words.rewritesPushed)
            compare(root.row("rebase").tag, "", "a pushed commit to land on rewrites nothing pushed")

            root.aimAt({ rebasePublished: true })
            compare(root.row("rebase").tag, Words.rewritesPushed, "what it replays is pushed")
            compare(root.row("rebase --interactive").tag, "", "the pressed commit is not")
            verify(root.row("rebase").offered, "the row goes on being a row")
            compare(root.row("rebase").blockedReason, "")
        }

        /// The held rows wear `warning` — what they throw away comes back from the discard record — and `--hard`
        /// takes `danger` only where a file it takes is one the record cannot copy (デザイン規約 §回数押し の色の表).
        function test_a_held_row_warns_and_only_an_uncopied_reset_is_danger() {
            root.aimAt({ hardResetTakes: 2 })
            verify(Qt.colorEqual(menu.hardResetRow.holdTone, Theme.warning), "copied whole")
            verify(Qt.colorEqual(menu.dropItem.holdTone, Theme.warning), "a dropped commit")
            verify(Qt.colorEqual(menu.stashDropItem.holdTone, Theme.warning), "a dropped stash")

            root.aimAt({ hardResetTakes: 2, hardResetNotCopied: true })
            verify(Qt.colorEqual(menu.hardResetRow.holdTone, Theme.danger), "a file the copy cannot take")
        }

        /// A menu raised under a git command this window started keeps the rows it has with nothing running, every one
        /// greyed with what it waits for, and gives them back the moment the command ends — the same menu, still up,
        /// saying so for the page to stand the rows on what the command left (デザイン規約 §メニュー).
        function test_a_menu_raised_under_a_running_command_waits_for_it_with_every_row() {
            root.aimAt({})
            menu.runningWhy = ""
            root.offer(root.stand())
            const idle = root.rowsOf(menu.menu)
            menu.menu.dismiss()
            verify(idle.rows > 0)
            compare(idle.greyed, 0, "nothing runs, nothing greys")

            startCommand()
            root.offer(root.stand())
            verify(menu.menu.visible, "the menu comes up")
            const under = root.rowsOf(menu.menu)
            compare(under.rows, idle.rows, "with the rows it has with nothing running")
            compare(under.greyed, under.rows, "every one of them waiting")
            compare(menu.switchItem.blockedWhy, Words.otherCommandRunning, "and saying what for")
            verify(menu.menu.openSub(menu.branchCard))
            const card = root.rowsOf(menu.branchCard)
            verify(card.rows > 0)
            compare(card.greyed, card.rows, "a card opened under it waits too")

            endCommand()
            compare(root.rowsOf(menu.menu).greyed, 0, "back the moment it ends")
            compare(root.rowsOf(menu.branchCard).greyed, 0)
            verify(menu.menu.visible, "with the menu still up")
            compare(stood.count, 1, "which stands it again once")
            compare(stood.signalArguments[0][0], "wait", "as the wait it was")
            menu.menu.dismiss()
        }

        /// A card opened under a command its menu was raised before waits for it, so the command's end stands the menu
        /// as a wait — once, the card's wait and the menu's end being the one end (`MenuLines.commandEnded`).
        function test_a_card_opened_under_a_command_makes_its_end_a_wait() {
            root.aimAt({})
            root.offer(root.stand())
            startCommand()
            compare(root.rowsOf(menu.menu).greyed, 0, "the menu up before it waits for nothing")
            verify(menu.menu.openSub(menu.branchCard))
            const card = root.rowsOf(menu.branchCard)
            compare(card.greyed, card.rows, "the card opened under it waits")
            endCommand()
            compare(stood.count, 1, "the end stands the menu once")
            compare(stood.signalArguments[0][0], "wait", "as a wait")
            menu.menu.dismiss()
        }

        /// Stood again on a newer reading with the menu up (`CommitMenuState.standAgain`), every level keeps its shape:
        /// a row the reading takes away greys where it is, in a card as on the first level, and none comes in.
        function test_a_menu_stood_again_keeps_its_shape_at_every_level() {
            root.aimAt({ pullRuns: "", lines: { "pull": null } })
            root.offer(root.stand())
            verify(menu.menu.openSub(menu.branchCard))
            const first = root.rowsOf(menu.menu).rows
            const card = root.rowsOf(menu.branchCard).rows
            verify(!menu.pullItem.present)

            root.lines.again("wait")
            root.aimAt({ lines: { "reach": null } })
            const facts = root.stand()
            facts.menu = root.menuOf({ "delete-branch": "", "delete-remote-branch": "", "delete-both": "" },
                                     { "upstreamWhere": "remote" })
            root.standCards(facts)
            verify(menu.switchItem.withdrawn, "a row the reading takes away greys where it is")
            compare(menu.switchItem.blockedWhy, menu.switchItem.withdrawnWhy)
            verify(menu.branchCard.upstreamItem.withdrawn, "in a card as on the first level")
            verify(!menu.pullItem.present, "a row the reading brings waits for the menu to be raised again")
            compare(root.rowsOf(menu.menu).rows, first)
            compare(root.rowsOf(menu.branchCard).rows, card)
        }

        /// A card whose rows the newer reading takes away greys them where they are, its title with them — the greyed
        /// title still opens the card —, and they take no press.
        function test_a_card_stood_away_greys_every_row_in_it() {
            root.aimAt({})
            root.offer(root.stand())
            verify(menu.menu.openSub(menu.resetSubmenu))
            root.lines.again("wait")
            root.aimAt({ lines: { "reset-soft": null, "reset-mixed": null, "reset-hard": null } })
            verify(menu.hardResetRow.withdrawn, "the card's rows are no longer there")
            compare(menu.hardResetRow.blockedWhy, menu.hardResetRow.withdrawnWhy)
            verify(root.titleOf(menu.resetSubmenu).greyed, "the title greys with them")
            verify(!menu.hardResetRow.completeHold(), "and its rows take no press")
            root.aimAt({})
            verify(!menu.hardResetRow.withdrawn, "back on offer, back live")
        }

        /// A stash's row has what brings the stash back — `apply` keeps the entry, `pop` drops it once its changes are
        /// in — and its drop, as its row on the left, each greyed on core's word; a press hands up the selector the
        /// row answers to.
        function test_a_stash_row_brings_it_back_or_drops_it() {
            menu.stashRef = "stash@{1}"
            menu.standFresh("stash")
            root.lines.answer = root.menuOf({ "apply-stash": "", "pop-stash": "", "drop-stash": "" })
            menu.offerStash()
            compare(root.rowsOf(menu.stashLevel).rows, 3, "apply, pop and the drop")
            const coded = code => {
                for (let i = 0; i < menu.stashLevel.count; i++) {
                    const item = menu.stashLevel.itemAt(i)
                    if (item && item.code === code && item.present)
                        return item
                }
                return null
            }
            coded("apply").triggered()
            compare(root.asked, "apply stash@{1}")
            coded("pop").triggered()
            compare(root.asked, "pop stash@{1}")
            verify(menu.stashDropItem.offered)
            compare(menu.stashDropItem.blockedReason, "")
            root.lines.answer = root.menuOf({ "apply-stash": "opening", "pop-stash": "opening",
                                              "drop-stash": "opening" })
            for (const code of ["apply", "pop"])
                compare(coded(code).blockedReason, Words.menuWhy("opening"), code)
            compare(menu.stashDropItem.blockedReason, Words.menuWhy("opening"))
            menu.stashLevel.dismiss()
            menu.stashRef = ""
        }

        /// Another worktree's uncommitted row has a menu of its own — its door named by the folder, and the apply, both
        /// handing up the path the row names — and it waits as the others do: raised under a command, every row greys
        /// and the end is said; stood again on a conflict, the apply greys where it is; and a withdrawal takes this
        /// menu down, not the commit's (`CommitMenuState.standAgain`).
        function test_the_carried_row_menu_waits_and_withdraws_as_the_others() {
            menu.carriedPath = "C:/w/topic"
            menu.carriedName = "topic"
            menu.standFresh("carried")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "" })
            startCommand()
            menu.offerCarried()
            verify(menu.carriedShowing)
            const under = root.rowsOf(menu.carriedRowMenu)
            compare(under.rows, 2, "the door and the apply")
            compare(under.greyed, 2, "every one waiting")
            endCommand()
            compare(root.rowsOf(menu.carriedRowMenu).greyed, 0, "back the moment it ends")
            compare(stood.count, 1, "which stands it again once")
            compare(stood.signalArguments[0][0], "wait")
            compare(menu.carriedOpenItem.codeName, "topic")

            root.lines.again("wait")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "conflicts-here" })
            verify(menu.carriedApplyItem.present, "an apply a conflict came under keeps its seat")
            compare(menu.carriedApplyItem.blockedReason, "Settle this worktree's conflicts first")
            menu.withdraw()
            verify(!menu.carriedShowing, "the withdrawal takes this menu down")
            menu.standFresh("carried")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "" })
            menu.offerCarried()
            menu.carriedApplyItem.triggered()
            compare(root.asked, "carried-apply C:/w/topic")
            menu.offerCarried()
            menu.carriedOpenItem.triggered()
            compare(root.asked, "carried-open C:/w/topic")
            menu.carriedRowMenu.dismiss()
        }

        /// A conflict on either side greys the apply where it is and says which side to settle — the other worktree by
        /// the name its door wears (デザイン規約 §別の worktree を読む).
        function test_a_conflict_on_either_side_greys_the_apply() {
            menu.carriedPath = "C:/w/topic"
            menu.carriedName = "topic"
            menu.standFresh("carried")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "conflicts-there" })
            menu.offerCarried()
            verify(menu.carriedApplyItem.offered)
            compare(menu.carriedApplyItem.blockedReason, "Settle its conflicts first — topic")
            compare(menu.carriedApplyItem.tipMarkWord, "topic")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "conflicts-here" })
            compare(menu.carriedApplyItem.blockedReason, "Settle this worktree's conflicts first")
            compare(menu.carriedApplyItem.tipMarkWord, "")
            root.lines.answer = root.menuOf({ "reach": "", "apply-changes": "" })
            compare(menu.carriedApplyItem.blockedReason, "")
            menu.carriedRowMenu.dismiss()
        }

        /// Only a command running as the menu opened is waited for: one started under a menu already up — the plain
        /// branch delete stands for git's answer with the menu up — greys nothing.
        function test_a_menu_already_up_greys_nothing_for_a_command_started_under_it() {
            root.aimAt({})
            menu.runningWhy = ""
            root.offer(root.stand())
            verify(menu.menu.openSub(menu.branchCard))
            menu.runningWhy = Words.otherCommandRunning
            compare(root.rowsOf(menu.menu).greyed, 0)
            compare(root.rowsOf(menu.branchCard).greyed, 0)
            menu.runningWhy = ""
            menu.menu.dismiss()
        }

        /// The rows used often widen in reach down the first level and close with the named branch's delete table —
        /// `push` beside `pull`, the table under them —, above the cards that hold what is used rarely
        /// (デザイン規約 §メニュー).
        function test_the_delete_table_closes_the_rows_used_often_above_the_cards() {
            root.aimAt({ pullRuns: "aside" })
            const facts = root.stand()
            facts.menu = root.menuOf(Object.assign({ "push": "" }, root.branchLines), { "upstreamWhere": "remote" })
            facts.push = { "state": "ready", "sendsPlain": true, "asks": false, "overwrites": false,
                           "target": "origin/feature/topic-a", "lease": "fed789",
                           "upstream": "origin/feature/topic-a", "ahead": 1, "behind": 0 }
            root.offer(facts)
            const shown = root.shownOrder()
            const at = word => shown.indexOf(word)
            verify(at("merge") >= 0 && at("merge") < at("pull"), "below the rows that move the tree's branch: " + shown)
            compare(at("push"), at("pull") + 1, "push beside pull")
            compare(shown.slice(at("push") + 1, at("push") + 4), ["branch --delete", "push --delete", "Delete both"],
                    "then the delete table")
            compare(shown[at("Delete both") + 1], "BRANCH", "and the cards under it")
            menu.menu.dismiss()
        }

        /// The rows that only add a commit hand up the row they stand on, each there as core answers it
        /// (`menu::Part::Copied` / `Undone`).
        function test_the_rows_that_only_add_a_commit_take_the_row_they_stand_on() {
            root.aimAt({})
            root.row("cherry-pick").triggered()
            compare(root.asked, "cherry-pick abc123")

            root.aimAt({})
            root.row("revert").triggered()
            compare(root.asked, "revert abc123")

            root.aimAt({ lines: { "cherry-pick": null, "revert": null } })
            verify(!root.row("cherry-pick").offered)
            verify(!root.row("revert").offered)

            root.aimAt({ lines: { "revert": null } })
            verify(root.row("cherry-pick").offered)
            verify(!root.row("revert").offered)
        }

        /// The other card comes up holding only what can still be made here.
        function test_each_card_is_aimed_only_at_a_name_of_its_own_kind_data() {
            const branchy = root.stand({
                "menu": root.menuOf(root.branchLines, { "upstreamWhere": "remote" }) })
            // A tag standing here only, as `tst_tagcard.qml` spells core's answer and the tag's record.
            const tagged = root.stand({
                "lead": { "kind": "tag", "full": "v1.0", "local": "", "leads": "", "path": "", "called": "",
                          "home": false, "cardHere": false, "placing": "free" },
                "card": { "pushRemote": "origin", "lease": "", "reach": "", "heldBack": "", "carriers": "",
                          "rowGoes": false },
                "menu": root.menuOf({ "tag-here": "", "push-tag": "", "delete-tag": "" }),
                "merged": "",
                "tagAt": "abc123" })
            return [
                {
                    tag: "the chip drew a branch",
                    aim: { targetKind: "branch", targetName: "feature/topic-a" },
                    facts: branchy,
                    named: "feature/topic-a",
                    tagNamed: "",
                },
                {
                    tag: "the chip drew a tag",
                    aim: { targetKind: "tag", targetName: "v1.0" },
                    facts: tagged,
                    named: "",
                    tagNamed: "v1.0",
                },
            ]
        }

        function test_each_card_is_aimed_only_at_a_name_of_its_own_kind(data) {
            root.aimAt(data.aim)
            root.offer(data.facts)

            compare(menu.branchCard.refId, data.named, "the branch card is aimed at a branch and nothing else")
            compare(menu.branchCard.applies, data.named !== "")
            compare(menu.deleteItem.offered, data.named !== "",
                    "and the delete table it lends the first level follows the words it was handed")

            compare(menu.tagCard.refId, data.tagNamed, "and the tag card at a tag and nothing else")
            verify(menu.tagCard.applies, "the tag card stays either way — a tag can still be made here")
            compare(menu.tagCard.tagHereItem.offered, true, "which is the row that says so")
            compare(menu.tagCard.pushTagItem.offered, data.tagNamed !== "",
                    "the rows that answer for an existing tag need one to name")
            compare(menu.tagCard.deleteTagItem.offered, data.tagNamed !== "")
            menu.menu.dismiss()
        }

        /// The WORKTREE card's two making rows (デザイン規約 §worktree を作る): the card stands with
        /// `Create worktree here…` on a row that names no worktree, `worktree add` names the folder it would make and
        /// greys with the reason where something is in the way there, and both hand up what the menu froze — a remote
        /// branch sent with the remote one it is made off.
        function test_the_worktree_card_makes_worktrees_and_names_where_they_go() {
            root.aimAt({})
            root.offer(root.making({ "here": true, "checkout": "branch", "branch": "feature/topic-a",
                                           "path": "C:/work/repo.worktrees/feature-topic-a",
                                           "place": "feature-topic-a" }))
            const card = menu.worktreeCard
            verify(card.offeredRows > 0, "the card stands on a row that names no worktree")
            verify(!card.removeWorktreeItem.offered, "with nothing to take away")
            verify(card.worktreeHereItem.offered)
            compare(card.worktreeHereItem.text, "Create worktree here…")
            card.worktreeHereItem.triggered()
            compare(root.asked, "worktree-here " + menu.oid)

            const add = card.worktreeAddItem
            verify(add.offered)
            compare(add.code, "worktree add")
            compare(add.nameMark, "tree")
            compare(add.codeName, "feature-topic-a")
            compare(add.blockedReason, "")
            // What the hover says once the folder is cut: the whole line, chip and folder, the folder behind its mark.
            compare(add.tip.words, "worktree add feature-topic-a")
            compare(add.tipMarks, [["worktree add ".length, "tree", undefined, "feature-topic-a".length]])
            add.triggered()
            compare(root.asked,
                    "worktree-add branch feature/topic-a  C:/work/repo.worktrees/feature-topic-a feature-topic-a")

            root.aimAt({ targetKind: "remote", targetName: "origin/feature/remote-only" })
            root.offer(root.making({
                "here": true, "checkout": "track", "branch": "feature/remote-only",
                "start": "origin/feature/remote-only", "path": "C:/work/repo.worktrees/feature-remote-only",
                "place": "feature-remote-only" }))
            card.worktreeAddItem.triggered()
            compare(root.asked, "worktree-add track feature/remote-only origin/feature/remote-only "
                    + "C:/work/repo.worktrees/feature-remote-only feature-remote-only")

            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "feature/blocked",
                "path": "C:/work/repo.worktrees/feature-blocked", "place": "feature-blocked", "taken": "folder" }))
            compare(card.worktreeAddItem.blockedReason, "The folder is not empty — feature-blocked")
            compare(card.worktreeAddItem.tipMarkWord, "feature-blocked", "the tip's tree mark stands before the folder")

            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "fix/listed",
                "path": "C:/work/repo.worktrees/fix-listed", "place": "fix-listed", "taken": "listed" }))
            compare(card.worktreeAddItem.blockedReason, "Taken by another worktree — fix-listed")
            compare(card.worktreeAddItem.tipMarkWord, "fix-listed")

            // The branch already out — here, or in the worktree named — greys the row on that, whatever the place.
            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "main",
                "path": "C:/work/repo.worktrees/main", "place": "main", "out": "current" }))
            verify(card.worktreeAddItem.offered, "the branch the tab stands on keeps the row")
            compare(card.worktreeAddItem.blockedReason, "Switch away first — this is the branch you are on")
            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "feature/topic-a",
                "path": "C:/work/repo.worktrees/feature-topic-a", "place": "feature-topic-a", "out": "held",
                "holderLeaf": "topic" }))
            compare(card.worktreeAddItem.blockedReason, Words.checkedOutElsewhere("topic"))
            compare(card.worktreeAddItem.tipMarkWord, "topic", "the holder's folder wears its mark in the tip")
            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "feature/clash",
                "path": "C:/work/repo.worktrees/feature-clash", "place": "feature-clash", "out": "rebasing" }))
            compare(card.worktreeAddItem.blockedReason, "Being rebased in this worktree",
                    "a stop here holds the branch with the tree detached")
            root.offer(root.making({
                "here": true, "checkout": "branch", "branch": "feature/clash",
                "path": "C:/work/repo.worktrees/feature-clash", "place": "feature-clash", "out": "bisecting" }))
            compare(card.worktreeAddItem.blockedReason, "Being bisected in this worktree", "the operation core names")

            // Where no new worktree can go, or where is not read yet, both making rows grey saying which, with no
            // folder to name (`menu::Placing`).
            const nowhere = [["bare", "bare", "Cannot create a worktree — the repository is bare"],
                             ["drive-root", "drive-root",
                              "Cannot create a worktree — the repository is at the root of the drive"],
                             ["worktrees-unread", "unread", "Wait for the worktrees to be read"]]
            for (const [why, placing, words] of nowhere) {
                root.offer(root.stand({
                    "lead": { "kind": "branch", "full": "feature/topic-a", "local": "feature/topic-a", "leads": "",
                              "path": "", "called": "", "home": false, "cardHere": false, "placing": placing },
                    "menu": root.menuOf({ "worktree-here": why, "worktree-add": why }, { "worktreeAdd": "branch" }),
                    "card": { "remoteCounterpart": "", "remoteCounterpartOid": "", "upstream": "" },
                    "merged": ""
                }))
                compare(card.worktreeHereItem.blockedReason, words, why)
                compare(card.worktreeAddItem.blockedReason, words, why)
                compare(card.worktreeAddItem.codeName, "", why + ": no folder to name")
            }

            root.offer(root.making({}))
            verify(!card.worktreeAddItem.offered, "a row whose name has no branch to take out has no row")
            compare(card.offeredRows, 0, "and a row with nothing to make or take away has no card")
            menu.menu.dismiss()
        }

        /// The cards' leases come up through this menu as the oids they are — a lease passed on as a flag would
        /// reach git as `true`.
        function test_the_carried_cards_hand_their_leases_up_whole() {
            const drifted = root.stand({
                "card": { "pushRemote": "origin", "lease": "deadbee", "reach": "origin", "heldBack": "drift",
                          "carriers": "origin", "rowGoes": false },
                "menu": root.menuOf({ "tag-here": "", "push-tag": "", "delete-tag": "",
                                      "delete-remote-tag": "tag-drifted", "delete-tag-both": "tag-drifted" }),
                "tagAt": "abc123" })
            root.aimAt({ targetKind: "tag", targetName: "v1.0" })
            root.offer(drifted)
            menu.tagCard.pushTagItem.held()
            compare(root.asked, "push origin v1.0 deadbee")

            const both = root.stand({
                "card": { "pushRemote": "origin", "lease": "", "reach": "origin", "heldBack": "",
                          "carriers": "origin", "rowGoes": false },
                "menu": root.menuOf({ "tag-here": "", "push-tag": "", "delete-tag": "", "delete-remote-tag": "" }),
                "tagAt": "abc123" })
            root.aimAt({ targetKind: "tag", targetName: "v1.0" })
            root.offer(both)
            menu.tagCard.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote-tag origin v1.0 " + menu.oid)

            const reading = root.stand({
                "card": { "remoteCounterpart": "", "remoteCounterpartOid": "", "upstream": "" },
                "menu": root.menuOf({ "delete-remote-branch": "" }),
                "merged": "" })
            root.aimAt({ targetKind: "remote", targetName: "origin/main" })
            root.offer(reading)
            menu.deleteRemoteItem.held()
            compare(root.asked, "delete-remote origin/main " + menu.oid)
        }
    }
}
