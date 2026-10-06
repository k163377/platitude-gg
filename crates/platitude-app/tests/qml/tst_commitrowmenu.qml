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
    /// The WORKTREE card's making rows where a case is not about them (`CommitMenuState.askCopyRows`).
    readonly property var makingNothing: ({ "oid": "", "here": false, "checkout": "", "branch": "", "start": "",
                                             "path": "", "place": "", "taken": "" })

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
        hardResetNotCopied: false
        onSwitchRequested: (kind, name) => root.asked = "switch " + kind + " " + name
        onCherryPickRequested: oidHex => root.asked = "cherry-pick " + oidHex
        onRevertRequested: oidHex => root.asked = "revert " + oidHex
        onMergeRequested: ref => root.asked = "merge " + ref
        onRebaseRequested: ref => root.asked = "rebase " + ref
        onPullRequested: root.asked = "pull"
        onResetRequested: mode => root.asked = "reset " + mode
        onPushTagRequested: (remote, tag, lease) => root.asked = "push " + remote + " " + tag + " " + lease
        onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
            root.asked = "delete-remote-tag " + remote + " " + tag + " " + expect
        onDeleteRemoteRequested: (remoteRef, expect) => root.asked = "delete-remote " + remoteRef + " " + expect
        onCopyHereRequested: oidHex => root.asked = "copy-here " + oidHex
        onCopyAddRequested: (mode, branch, start, path, name) =>
            root.asked = "copy-add " + mode + " " + branch + " " + start + " " + path + " " + name
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
                      "published": false, "canIntegrate": true, "canSequence": true, "hardResetTakes": 0,
                      "hardResetNotCopied": false }
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
            compare(menu.switchItem.code, "", "there is no git command for opening a worktree")
            compare(menu.switchItem.text, "Open")
            compare(menu.switchItem.markName, "topic")
            compare(menu.switchItem.nameMark, "tree")
            verify(menu.switchItem.namesMark)
            compare(menu.switchItem.ToolTip.text, "Open topic", "a cut folder comes back whole on the hover")
            compare(menu.switchItem.tipMarkWord, "topic", "behind the mark it wears on the row")
            root.aimAt({ canSwitch: true })
            compare(menu.switchItem.code, "switch", "an ordinary move is the command it runs")
            verify(!menu.switchItem.namesMark)
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

        /// The held rows wear `warning` — what they throw away comes back from the discard record — and `--hard`
        /// takes `danger` only where a file it takes is one the record cannot copy (デザイン規約 §長押し の色の表).
        function test_a_held_row_warns_and_only_an_uncopied_reset_is_danger() {
            root.aimAt({ hardResetTakes: 2 })
            verify(Qt.colorEqual(menu.hardResetRow.holdTone, Theme.warning), "copied whole")
            verify(Qt.colorEqual(menu.dropItem.holdTone, Theme.warning), "a dropped commit")
            verify(Qt.colorEqual(menu.stashDropItem.holdTone, Theme.warning), "a dropped stash")

            root.aimAt({ hardResetTakes: 2, hardResetNotCopied: true })
            verify(Qt.colorEqual(menu.hardResetRow.holdTone, Theme.danger), "a file the copy cannot take")
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
                              "remoteCounterpartOid": "abc123", "remoteDrifted": false, "open": true, "merged": "yes",
                              "offers": ["switch", "branch-here", "integrate", "delete", "set-upstream"] }
            const empty = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteCounterpartOid": "",
                            "remoteDrifted": false, "open": false, "merged": "", "offers": [] }
            // A tag standing here only, in core's words as `tst_tagcard.qml` spells them.
            const tagged = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "tagReach": "",
                             "tagHeldBack": "", "tagCarriers": "", "tagHere": true,
                             "offers": ["branch-here", "integrate", "delete", "push-tag"] }
            const notag = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] }
            return [
                {
                    tag: "the chip drew a branch",
                    aim: { targetKind: "branch", targetName: "feature/topic-a" },
                    facts: { "branch": branchy, "tag": notag, "making": root.makingNothing },
                    named: "feature/topic-a",
                    tagNamed: "",
                },
                {
                    tag: "the chip drew a tag",
                    aim: { targetKind: "tag", targetName: "v1.0" },
                    facts: { "branch": empty, "tag": tagged, "making": root.makingNothing },
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

        /// The WORKTREE card's two making rows (デザイン規約 §作業コピーを作る): the card stands with
        /// `Create worktree here…` on a row that names no copy, `worktree add` names the folder it would make and greys
        /// with the reason where something is in the way there, and both hand up what the menu froze — a remote branch
        /// sent with the remote one it is made off.
        function test_the_worktree_card_makes_copies_and_names_where_they_go() {
            const empty = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteCounterpartOid": "",
                            "remoteDrifted": false, "open": false, "merged": "", "offers": [] }
            const notag = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] }
            const making = { "oid": menu.oid, "here": true, "checkout": "branch", "branch": "feature/topic-a",
                             "start": "", "path": "C:/work/repo.worktrees/feature-topic-a",
                             "place": "feature-topic-a", "taken": "" }
            root.aimAt({})
            menu.offerCommit({ "branch": empty, "tag": notag, "busy": 0, "making": making })
            const card = menu.copyCard
            verify(card.applies, "the card stands on a row that names no copy")
            verify(!card.removeCopyItem.offered, "with nothing to take away")
            verify(card.copyHereItem.offered)
            compare(card.copyHereItem.text, "Create worktree here…")
            card.copyHereItem.triggered()
            compare(root.asked, "copy-here " + menu.oid)

            const add = card.copyAddItem
            verify(add.offered)
            compare(add.code, "worktree add")
            compare(add.nameMark, "tree")
            compare(add.markName, "feature-topic-a")
            compare(add.blockedReason, "")
            // What the hover says once the folder is cut: the whole line, chip and folder, the folder behind its mark.
            compare(add.ToolTip.text, "worktree add feature-topic-a")
            compare(add.tipMarkWord, "feature-topic-a")
            add.triggered()
            compare(root.asked,
                    "copy-add branch feature/topic-a  C:/work/repo.worktrees/feature-topic-a feature-topic-a")

            root.aimAt({ targetKind: "remote", targetName: "origin/feature/remote-only" })
            menu.offerCommit({ "branch": empty, "tag": notag, "busy": 0, "making": {
                "oid": menu.oid, "here": true, "checkout": "track", "branch": "feature/remote-only",
                "start": "origin/feature/remote-only", "path": "C:/work/repo.worktrees/feature-remote-only",
                "place": "feature-remote-only", "taken": "" } })
            card.copyAddItem.triggered()
            compare(root.asked, "copy-add track feature/remote-only origin/feature/remote-only "
                    + "C:/work/repo.worktrees/feature-remote-only feature-remote-only")

            menu.offerCommit({ "branch": empty, "tag": notag, "busy": 0, "making": {
                "oid": menu.oid, "here": true, "checkout": "branch", "branch": "feature/blocked", "start": "",
                "path": "C:/work/repo.worktrees/feature-blocked", "place": "feature-blocked", "taken": "folder" } })
            compare(card.copyAddItem.blockedReason, "The folder is not empty — feature-blocked")
            compare(card.copyAddItem.tipMarkWord, "feature-blocked", "the tip's tree mark stands before the folder")

            menu.offerCommit({ "branch": empty, "tag": notag, "busy": 0, "making": {
                "oid": menu.oid, "here": true, "checkout": "branch", "branch": "fix/listed", "start": "",
                "path": "C:/work/repo.worktrees/fix-listed", "place": "fix-listed", "taken": "listed" } })
            compare(card.copyAddItem.blockedReason, "Taken by another worktree — fix-listed")
            compare(card.copyAddItem.tipMarkWord, "fix-listed")

            menu.offerCommit({ "branch": empty, "tag": notag, "busy": 0, "making": root.makingNothing })
            verify(!card.copyAddItem.offered, "a branch out somewhere has no row, not a grey one")
            verify(!card.applies, "and a row with nothing to make or take away has no card")
            menu.menu.dismiss()
        }

        /// The cards' leases come up through this menu as the oids they are — a lease passed on as a flag would
        /// reach git as `true`.
        function test_the_carried_cards_hand_their_leases_up_whole() {
            const empty = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "", "remoteCounterpartOid": "",
                            "remoteDrifted": false, "open": false, "merged": "", "offers": [] }
            const drifted = { "pushRemote": "origin", "tagDriftOid": "deadbee", "tagOnlyThere": false,
                              "tagReach": "origin", "tagHeldBack": "drift", "tagCarriers": "origin", "tagHere": true,
                              "offers": ["branch-here", "integrate", "delete", "push-tag"] }
            root.aimAt({ targetKind: "tag", targetName: "v1.0" })
            menu.offerCommit({ "branch": empty, "tag": drifted, "making": root.makingNothing })
            menu.tagCard.pushTagItem.held()
            compare(root.asked, "push origin v1.0 deadbee")

            const both = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "tagReach": "origin",
                           "tagHeldBack": "", "tagCarriers": "origin", "tagHere": true,
                           "offers": ["branch-here", "integrate", "delete", "push-tag", "delete-remote-tag"] }
            root.aimAt({ targetKind: "tag", targetName: "v1.0" })
            menu.offerCommit({ "branch": empty, "tag": both, "making": root.makingNothing })
            menu.tagCard.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote-tag origin v1.0 " + menu.oid)

            const reading = { "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "",
                              "remoteCounterpartOid": "", "remoteDrifted": false, "open": true, "merged": "",
                              "offers": ["delete"] }
            const notag = { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] }
            root.aimAt({ targetKind: "remote", targetName: "origin/main" })
            menu.offerCommit({ "branch": reading, "tag": notag, "making": root.makingNothing })
            menu.branchCard.deleteItem.held()
            compare(root.asked, "delete-remote origin/main " + menu.oid)
        }
    }
}
