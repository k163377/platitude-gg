import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The graph row's real `CommitRowMenu`, standing on the answers its one door reads (`CommitMenuState`): which rows
// are there, what each says, which kind the move is aimed with, and what a press asks for. What a state allows is
// `offers::ref_menu` / `offers::commit_menu`'s and tested there.
//
// No picture tells an offered row from one that is not once the list closes over them, nor carries the kind a
// press was sent with.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the menu runs nothing itself.
    property string asked: ""

    CommitRowMenu {
        id: menu
        branch: "main"
        oid: "abc123"
        stashRef: ""
        published: false
        canSequence: true
        canIntegrate: true
        canEditHistory: true
        canMoveBranch: true
        canBranchHere: true
        stashCanWrite: false
        tipHeldElsewhere: false
        canSwitch: true
        switchAsks: false
        heldLeaf: ""
        canPull: true
        pullBlocked: false
        hardResetTakes: 0
        onSwitchRequested: (kind, name) => root.asked = "switch " + kind + " " + name
        onCherryPickRequested: oidHex => root.asked = "cherry-pick " + oidHex
        onRevertRequested: oidHex => root.asked = "revert " + oidHex
        onMergeRequested: ref => root.asked = "merge " + ref
        onRebaseRequested: ref => root.asked = "rebase " + ref
        onPullRequested: root.asked = "pull"
        onResetRequested: mode => root.asked = "reset " + mode
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

    /// `fields` names only what this case is about.
    function aimAt(fields) {
        const all = { "targetKind": "branch", "targetName": "feature/topic-a", "canSwitch": true,
                      "switchAsks": false, "heldLeaf": "", "canPull": true, "pullBlocked": false,
                      "published": false, "canIntegrate": true, "canSequence": true, "hardResetTakes": 0 }
        for (const key in fields)
            all[key] = fields[key]
        for (const key in all)
            menu[key] = all[key]
        root.asked = ""
    }

    TestCase {
        name: "CommitRowMenu"
        when: windowShown

        /// The move row is there exactly when core says so (offers::ref_menu `switch_to`); a move that cannot be
        /// made has no row, not a grey one.
        function test_the_move_row_follows_the_word_it_was_handed_data() {
            return [
                { tag: "a branch the tree is not on", fields: { canSwitch: true }, offered: true, asks: false },
                { tag: "the branch the tree is on", fields: { canSwitch: false }, offered: false, asks: false },
                {
                    tag: "one with an operation standing",
                    fields: { canSwitch: true, switchAsks: true },
                    offered: true,
                    asks: true,
                },
                {
                    tag: "one another copy holds",
                    fields: { canSwitch: true, heldLeaf: "topic" },
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

        /// The press opens that copy and asks nothing, so the words are all that is read before it
        /// (offers::SwitchAction::OpenHolder). The mark is the WORKTREES one that copy wears everywhere.
        function test_a_held_branch_names_the_copy_instead_of_a_command() {
            root.aimAt({ canSwitch: true, heldLeaf: "topic" })
            compare(menu.switchItem.code, "", "there is no git command for opening a working copy")
            compare(menu.switchItem.text, "Open")
            compare(menu.switchItem.markName, "topic")
            compare(menu.switchItem.nameMark, "tree")
            verify(menu.switchItem.namesMark)
            root.aimAt({ canSwitch: true })
            compare(menu.switchItem.code, "switch", "an ordinary move is the command it runs")
            verify(!menu.switchItem.namesMark)
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

        /// Greying is the only thing this file decides about `pull`: git would turn a diverged press down, so the
        /// row stays and says why (デザイン規約 §メニュー).
        function test_the_pull_row_is_there_on_the_word_and_grey_on_the_answer() {
            root.aimAt({})
            verify(root.row("pull").offered)
            compare(root.row("pull").blockedReason, "")

            root.aimAt({ pullBlocked: true })
            verify(root.row("pull").offered, "the row stays: it is the reader's question, not a missing one")
            compare(root.row("pull").blockedReason, Words.pullDiverged)

            root.aimAt({ canPull: false })
            verify(!root.row("pull").offered, "a row with no far side to go to is no row")
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

        /// The rewrite rows wear the tag and stay a plain click (デザイン規約 §長押し「ローカルの履歴書き換えはクリック」).
        function test_a_pushed_commit_is_told_about_and_not_held() {
            root.aimAt({})
            compare(root.row("rebase").note, "")

            root.aimAt({ published: true })
            compare(root.row("rebase").note, Words.rewritesPushed)
            verify(root.row("rebase").offered, "the row goes on being a row")
            compare(root.row("rebase").blockedReason, "")
        }

        /// They ask less of the repository than the rest (offers::commit_menu `sequence`).
        function test_the_rows_that_only_add_a_commit_take_the_row_they_stand_on() {
            root.aimAt({})
            root.row("cherry-pick").triggered()
            compare(root.asked, "cherry-pick abc123")

            root.aimAt({})
            root.row("revert").triggered()
            compare(root.asked, "revert abc123")

            root.aimAt({ canSequence: false })
            verify(!root.row("cherry-pick").offered)
            verify(!root.row("revert").offered)
        }

        /// The other card comes up holding only what can still be made here.
        function test_each_card_is_aimed_only_at_a_name_of_its_own_kind_data() {
            const branchy = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "origin/feature/topic-a",
                              "remoteDrifted": false, "open": true, "merged": "yes",
                              "offers": ["switch", "branch-here", "integrate", "delete", "set-upstream"] }
            const empty = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteDrifted": false,
                            "open": false, "merged": "", "offers": [] }
            // A tag standing here only, in core's words as `tst_tagcard.qml` spells them.
            const tagged = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false,
                             "offers": ["branch-here", "integrate", "delete", "push-tag"] }
            const notag = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] }
            return [
                {
                    tag: "the chip drew a branch",
                    aim: { targetKind: "branch", targetName: "feature/topic-a" },
                    facts: { "branch": branchy, "tag": notag },
                    named: "feature/topic-a",
                    tagNamed: "",
                },
                {
                    tag: "the chip drew a tag",
                    aim: { targetKind: "tag", targetName: "v1.0" },
                    facts: { "branch": empty, "tag": tagged },
                    named: "",
                    tagNamed: "v1.0",
                },
            ]
        }

        function test_each_card_is_aimed_only_at_a_name_of_its_own_kind(data) {
            root.aimAt(data.aim)
            menu.offerCommit(data.facts)

            compare(menu.branchCard.refId, data.named, "the branch card is aimed at a branch and nothing else")
            compare(menu.branchCard.applies, data.named !== "")
            compare(menu.branchCard.deleteItem.offered, data.named !== "",
                    "its rows follow the words it was handed")

            compare(menu.tagCard.refId, data.tagNamed, "and the tag card at a tag and nothing else")
            verify(menu.tagCard.applies, "the tag card stays either way — a tag can still be made here")
            compare(menu.tagCard.tagHereItem.offered, true, "which is the row that says so")
            compare(menu.tagCard.pushTagItem.offered, data.tagNamed !== "",
                    "the rows that answer for an existing tag need one to name")
            compare(menu.tagCard.deleteTagItem.offered, data.tagNamed !== "")
            menu.menu.dismiss()
        }
    }
}
