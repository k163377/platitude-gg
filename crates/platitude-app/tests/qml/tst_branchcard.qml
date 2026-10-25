import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import "painted.js" as Painted

// The real branch card (`RefBranchMenu`) standing on the answers a menu would hand it (`RefRowMenu.branchFacts`), and
// the four rows it lends the level above — the push beside `pull` and the delete table (`BranchPushRow` /
// `BranchDeleteRow` / `BranchRemoteDeleteRow` / `BranchBothDeleteRow`), laid out as both entrances lay them: which rows
// each keeps, what each says, the gesture and colour each takes, and what a press asks for. Which rows a state may
// offer is `offers::ref_menu`'s and tested there.
//
// The greyed rows' sentences are judged here: a wrong reason frames like a right one.
Item {
    id: root
    // Wide enough for a card that bid a long name whole: the window would cut it otherwise.
    width: 640
    height: 480

    /// What a press asked for, last one wins: the card and its rows run nothing themselves.
    property string asked: ""

    /// Word lists as `offers::RefMenuOffers::words` answers them — data, not a rule.
    readonly property var free: ["switch", "branch-here", "integrate", "delete", "delete-remote", "set-upstream",
                                 "unset-upstream"]
    /// The branch the worktree is on: no move, no delete, and the rows say so.
    readonly property var current: ["branch-here", "integrate", "set-upstream", "unset-upstream", "current"]
    /// One another worktree holds: `switch` stays and leads there without asking, the delete goes.
    readonly property var held: ["switch", "branch-here", "integrate", "set-upstream", "unset-upstream"]
    /// A reading standing on another commit: the local delete stays, the one that reaches over there goes.
    readonly property var drifted: ["switch", "branch-here", "integrate", "delete", "set-upstream", "unset-upstream"]

    /// The level above the card, holding the rows the card lends it and the card itself, as `RefRowMenu` and
    /// `CommitRowMenu` hold them.
    AppMenu {
        id: level
        BranchPushRow {
            id: pushRow
            card: branchCard
        }
        BranchDeleteRow {
            id: deleteRow
            card: branchCard
        }
        BranchRemoteDeleteRow {
            id: remoteDeleteRow
            card: branchCard
        }
        BranchBothDeleteRow {
            id: bothDeleteRow
            card: branchCard
        }
        RefBranchMenu {
            id: branchCard
            onDeleteRequested: (kind, id, name, oidHex) => root.asked = "delete " + kind + " " + id
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                root.asked = "upstream " + branch + " " + counterpart + " " + unsettable
            onUnsetUpstreamRequested: branch => root.asked = "unset " + branch
            onCheckDeleteRequested: branch => root.asked = "check " + branch
            onForceDeleteRequested: branch => root.asked = "force " + branch
            onDeleteRemoteRequested: (remoteRef, expect) => root.asked = "delete-remote " + remoteRef + " " + expect
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                root.asked = "delete-both " + branch + " " + remoteRef + " " + forced + " " + expect
            onPushRequested: (branch, force, lease) => root.asked = "push " + branch + " " + force + " " + lease
            onPublishRequested: (branch, upstream) => root.asked = "publish " + branch + " " + upstream
        }
    }

    /// `merged` is what the drawn rows already say (`yes` / `no`, empty when only git can answer). The row stands on
    /// `abc123` and its reading on `def456`, so a press says which commit it leased. `push` is the push row's answer
    /// (`NavSectionModel.branchPush`), none by default.
    function standOn(fields) {
        const all = { "kind": "branch", "name": "feature/topic-a", "full": "feature/topic-a",
                      "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "origin/feature/topic-a",
                      "upstream": "origin/feature/topic-a", "upstreamWhere": "remote",
                      "remoteCounterpartOid": "def456", "remoteDrifted": false, "open": true, "merged": "yes",
                      "offers": root.free, "push": undefined }
        for (const key in fields)
            all[key] = fields[key]
        root.asked = ""
        branchCard.standOn(all.kind, all.name, all.full, "abc123", {
            "heldByWorktree": all.heldByWorktree,
            "holderLeaf": all.holderLeaf,
            "remoteCounterpart": all.remoteCounterpart,
            "upstream": all.upstream,
            "upstreamWhere": all.upstreamWhere,
            "remoteCounterpartOid": all.remoteCounterpartOid,
            "remoteDrifted": all.remoteDrifted,
            "open": all.open,
            "merged": all.merged,
            "offers": all.offers,
            "push": all.push
        })
    }
    /// A push row's answer in `state`, going to `origin/feature/topic-a` and leased to `fed789` where it counts.
    function pushing(state) {
        return { "state": state, "target": "origin/feature/topic-a", "lease": "fed789",
                 "upstream": "origin/feature/topic-a" }
    }
    /// …and where the push would stay in this repository (`stays-here` — measured against another branch here and
    /// marked to no remote, or marked to `.`): that alone, every other field empty.
    readonly property var pushingHere: ({ "state": "stays-here", "target": "", "lease": "", "upstream": "" })

    TestCase {
        name: "BranchCard"
        when: windowShown

        /// A menu a test opened goes whatever the test said, so the next one finds its rows at rest.
        function cleanup() {
            if (!level.visible)
                return
            level.dismiss()
            tryVerify(() => !level.visible)
        }

        /// A branch's three delete forms are a fixed table whose rows stay and grey (デザイン規約 §メニュー).
        function test_why_each_row_of_a_branchs_table_is_out_data() {
            return [
                {
                    tag: "the branch the tree is on",
                    fields: { offers: root.current },
                    reason: "Switch away first — this is the branch you are on",
                },
                {
                    tag: "held by another worktree",
                    fields: { offers: root.held, heldByWorktree: "C:/worktrees/topic", holderLeaf: "topic" },
                    reason: "Checked out in another worktree — topic",
                    // The one reason naming something outside this repository, so the tip gets a word for the tree
                    // mark (デザイン規約 §ref の種別).
                    marks: "topic",
                },
            ]
        }

        function test_why_each_row_of_a_branchs_table_is_out(data) {
            root.standOn(data.fields)
            verify(deleteRow.offered, "the table keeps its seats")
            compare(deleteRow.blockedReason, data.reason)
            compare(bothDeleteRow.blockedReason, data.reason, "and the pair names the half that runs first")
            const marks = data.marks === undefined ? "" : data.marks
            compare(deleteRow.tipMarkWord, marks, "the word the tip stands a mark against")
            compare(bothDeleteRow.tipMarkWord, marks)
        }

        function test_a_drifted_reading_is_out_while_the_local_delete_is_not() {
            root.standOn({ offers: root.drifted, remoteDrifted: true })
            compare(deleteRow.blockedReason, "", "the branch here goes")
            compare(remoteDeleteRow.blockedReason, "Differs from origin/feature/topic-a")
            compare(bothDeleteRow.blockedReason, "Differs from origin/feature/topic-a")
        }

        /// The colour a row's tag is painted in.
        function tagInk(row) {
            return Painted.inkOf(row, row.note)
        }

        /// A tag is one of the row's words: on the row out of reach it greys with them, and stays the warning on the
        /// one that can still be pressed (デザイン規約 §無効).
        function test_a_row_out_of_reach_greys_its_tag() {
            root.standOn({ offers: root.drifted, remoteDrifted: true, merged: "no" })
            verify(bothDeleteRow.blocked)
            compare(bothDeleteRow.note, "not merged")
            compare(tagInk(bothDeleteRow), Theme.textMuted)
            verify(!deleteRow.blocked)
            compare(deleteRow.note, "not merged")
            compare(tagInk(deleteRow), Theme.warning)
        }

        /// Under the presses the tag lifts with the words over the fill: in the warning it would sink into it
        /// (デザイン規約 §回数押しの色).
        function test_a_row_being_pressed_lifts_its_tag() {
            root.standOn({ merged: "no" })
            compare(deleteRow.note, "not merged")
            verify(level.offerHere(), "the menu opens")
            tryVerify(() => level.opened)
            // The menu lays its rows out after it stands: press once they are, not on the bare frame.
            verify(waitForItemPolished(level.contentItem), "the rows are laid out")
            mouseClick(deleteRow, 30, deleteRow.height / 2)
            tryVerify(() => deleteRow.holding)
            compare(tagInk(deleteRow), Theme.textOnAccent)
        }

        function test_a_branch_nothing_holds_back_offers_all_three() {
            root.standOn({})
            for (const row of [deleteRow, remoteDeleteRow, bothDeleteRow]) {
                verify(row.offered)
                compare(row.blockedReason, "")
            }
            compare(deleteRow.code, "branch --delete")
            compare(remoteDeleteRow.code, "push --delete")
            compare(deleteRow.holdPresses, Metrics.holdPressesPlain,
                    "the everyday delete is the plain delete's two presses; git is what may still refuse it")
            compare(deleteRow.holdTone, Theme.textPrimary, "in the plain ink")
            verify(remoteDeleteRow.holdPresses > 0, "reaching past this machine is held")
            compare(remoteDeleteRow.holdTone, Theme.warning)
            compare(bothDeleteRow.holdTone, Theme.warning, "while the local half is not a force delete")
        }

        /// It writes configuration, which the current branch and a held one still take (offers::ref_menu).
        function test_the_upstream_row_follows_its_own_word() {
            root.standOn({})
            verify(branchCard.upstreamItem.offered)
            root.standOn({ offers: root.current })
            verify(branchCard.upstreamItem.offered, "the branch the tree is on writes settings like any other")
        }

        /// Taking the upstream off (デザイン規約 §ブランチが測られる相手を決める): seated by core's word, in words that pair
        /// with `Set upstream…`, naming what goes, the plain delete's two presses in the plain ink.
        function test_the_unset_row_names_the_upstream_it_takes_off() {
            root.standOn({})
            const row = branchCard.unsetUpstreamItem
            verify(row.offered)
            compare(row.code, "", "words, no chip")
            compare(row.text, "Unset upstream origin/feature/topic-a")
            compare(row.refName, "origin/feature/topic-a")
            compare(row.holdPresses, Metrics.holdPressesPlain)
            compare(row.holdTone, Theme.textPrimary)
            // A name is plain, a remote's as any other's: the code ground sets it apart (§ref の種別「文の中の名前」).
            verify(Qt.colorEqual(row.refColor, Theme.textPrimary), "a remote branch: " + row.refColor)
            root.standOn({ remoteCounterpart: "", upstream: "release", upstreamWhere: "local" })
            verify(Qt.colorEqual(row.refColor, Theme.textPrimary), "a branch here: " + row.refColor)
            // …but for the branch stood on, in its own colour wherever it is named.
            root.standOn({ remoteCounterpart: "", upstream: "main", upstreamWhere: "current" })
            verify(Qt.colorEqual(row.refColor, Theme.textLink), "the branch the tree is on: " + row.refColor)

            // A gone upstream names no reading, and is taken off all the same.
            root.standOn({ remoteCounterpart: "", remoteCounterpartOid: "", upstream: "origin/gone" })
            verify(row.offered)
            compare(row.refName, "origin/gone")

            root.standOn({ offers: root.current })
            verify(row.offered, "the branch the tree is on takes it like any other setting")

            root.standOn({ remoteCounterpart: "", upstream: "", offers: ["switch", "delete", "set-upstream"] })
            verify(!row.offered, "nothing to take off")
        }

        /// The card holds the upstream rows alone, so nothing else widens it: the words round the name bid whole and
        /// the name at most `labelColW` (`AppMenuItem.growsForText`) — a short one stands whole, a long one gives way
        /// into the hover.
        function test_a_card_of_upstream_rows_alone_cuts_only_a_long_name_data() {
            return [
                { tag: "a short upstream", upstream: "origin/topic", cut: false },
                { tag: "one past the names' bid", upstream: "origin/" + "a".repeat(40), cut: true },
            ]
        }

        function test_a_card_of_upstream_rows_alone_cuts_only_a_long_name(data) {
            root.standOn({ remoteCounterpart: data.upstream, upstream: data.upstream })
            verify(level.offerHere(), "the menu opens")
            tryVerify(() => level.opened)
            verify(level.openSub(branchCard), "the card opens")
            tryVerify(() => branchCard.opened)
            verify(waitForItemPolished(branchCard.contentItem), "the rows are laid out")
            const row = branchCard.unsetUpstreamItem
            compare(row.refName, data.upstream)
            compare(row.refCut, data.cut, "in a card " + branchCard.width + " wide")
        }

        /// The question an emptied name turns into is only offered where there is something to take off.
        function test_the_upstream_question_is_told_whether_an_emptied_name_unsets() {
            root.standOn({ remoteCounterpart: "", remoteCounterpartOid: "", upstream: "origin/gone" })
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a  true", "opens at the default, and the gone one can go")

            root.standOn({ remoteCounterpart: "", upstream: "", offers: ["switch", "delete", "set-upstream"] })
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a  false", "nothing for it to take off")
        }

        /// A remote-tracking row keeps the assembled rule; its own delete is the held one reaching past this machine.
        function test_a_remote_row_keeps_only_the_rows_it_can_name() {
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: ["switch", "branch-here", "integrate", "delete"] })
            verify(deleteRow.offered)
            compare(deleteRow.code, "push --delete")
            verify(deleteRow.holdPresses > 0)
            compare(deleteRow.holdTone, Theme.warning)
            verify(!remoteDeleteRow.offered, "a reading has no reading of its own")
            verify(!bothDeleteRow.offered)
            verify(!branchCard.upstreamItem.offered, "and no setting of its own either")
            verify(!branchCard.unsetUpstreamItem.offered)
            verify(!branchCard.applies, "so the card, aimed at it, takes its title row away")
        }

        /// The toolbar's forms for a branch the worktree is not on (デザイン規約 §リモートへ送る): a click where git takes
        /// it, the held leased overwrite where only that reaches, a question where nothing says where it goes.
        function test_the_push_row_takes_the_toolbars_form_data() {
            return [
                { tag: "ready", state: "ready", code: "push", presses: 0, words: "to origin/feature/topic-a" },
                { tag: "elsewhere", state: "elsewhere", code: "push", presses: 0,
                  words: "to origin/feature/topic-a" },
                { tag: "behind", state: "behind", code: "push --force", presses: Metrics.holdPressesWarned,
                  words: "to origin/feature/topic-a" },
                { tag: "diverged", state: "diverged", code: "push --force", presses: Metrics.holdPressesWarned,
                  words: "to origin/feature/topic-a" },
                { tag: "never sent", state: "publish", code: "push", presses: 0, words: "to…" },
            ]
        }

        function test_the_push_row_takes_the_toolbars_form(data) {
            root.standOn({ push: root.pushing(data.state) })
            verify(pushRow.offered)
            compare(pushRow.code, data.code)
            compare(pushRow.holdPresses, data.presses)
            compare(pushRow.text, data.words)
            compare(pushRow.holdTone, Theme.warning, "an overwrite reaches past this machine")
        }

        /// Nothing to send, nothing to name, or a door of its own: no seat (デザイン規約 §メニュー).
        function test_the_push_row_has_no_seat_where_it_could_do_nothing_here_data() {
            return [
                { tag: "the remote has it all", fields: { push: root.pushing("clean") } },
                { tag: "no answer read", fields: {} },
                { tag: "the branch the tree is on: the toolbar's button",
                  fields: { offers: root.current, push: root.pushing("ready") } },
                { tag: "the branch the tree is on, against one here: still the toolbar's",
                  fields: { offers: root.current, push: root.pushingHere } },
                { tag: "a closed tab", fields: { open: false, push: root.pushing("ready") } },
                { tag: "a remote-tracking row sends nothing",
                  fields: { kind: "remote", name: "origin/main", full: "origin/main", remoteCounterpart: "",
                            offers: ["delete"], push: root.pushing("ready") } },
            ]
        }

        function test_the_push_row_has_no_seat_where_it_could_do_nothing_here(data) {
            root.standOn(data.fields)
            verify(!pushRow.offered)
        }

        /// No remote to send to, so the row stands out in the first push's words and says why — the toolbar's words
        /// for the same standing (デザイン規約 §リモートへ送る).
        function test_the_push_row_stands_out_where_the_push_would_stay_here() {
            root.standOn({ push: root.pushingHere })
            verify(pushRow.offered)
            verify(pushRow.blocked)
            compare(pushRow.code, "push")
            compare(pushRow.text, "to…")
            compare(pushRow.holdPresses, 0)
            compare(pushRow.blockedReason, "Cannot push — its upstream is not supported")

            // The next card on a branch that can be sent drops it.
            root.standOn({ push: root.pushing("ready") })
            verify(!pushRow.blocked)
        }

        function test_the_push_row_asks_for_the_push_its_form_says() {
            root.standOn({ push: root.pushing("ready") })
            pushRow.triggered()
            compare(root.asked, "push feature/topic-a  ", "a plain push, nothing leased")

            // Leased to the commit the card showed the far side on.
            root.standOn({ push: root.pushing("diverged") })
            pushRow.held()
            compare(root.asked, "push feature/topic-a lease fed789")

            // The question opens where the branch already points.
            root.standOn({ push: root.pushing("publish") })
            pushRow.triggered()
            compare(root.asked, "publish feature/topic-a origin/feature/topic-a")
        }

        function test_a_row_that_names_no_branch_has_no_card() {
            root.standOn({ kind: "tag", name: "v1.0", full: "v1.0", offers: [] })
            verify(!branchCard.applies)
        }

        /// git's refusal lands while the menu stands: the row turns into the held `-D` under the hand
        /// (デザイン規約 §左メニューの所作), and the pair follows.
        function test_a_refusal_turns_the_row_into_the_held_force_delete() {
            root.standOn({})
            compare(deleteRow.code, "branch --delete")
            compare(deleteRow.note, "")
            compare(bothDeleteRow.note, "")

            branchCard.forceDeleteBranch = "feature/topic-a"
            compare(deleteRow.holdPresses, Metrics.holdPressesWarned, "the plain two are no longer enough")
            // Commits nothing else reaches, but the discard record brings them back with the name
            // (デザイン規約 §回数押し の色の表): the warning, not the danger.
            compare(deleteRow.holdTone, Theme.warning, "what goes comes back from the record")
            compare(bothDeleteRow.note, "not merged")
            compare(bothDeleteRow.holdTone, Theme.warning, "so the pair stays the warning too")
            // With no press under way, the chip and the note already say what the row now runs.
            compare(deleteRow.code, "branch -D")
            compare(deleteRow.note, "not merged")
        }

        /// Where the drawn rows cannot answer, git is asked — by request, like any other.
        function test_the_early_answer_comes_off_the_rows_or_is_asked_of_git() {
            root.standOn({ merged: "no" })
            verify(branchCard.deleteAnswered)
            verify(!branchCard.deleteMerged)
            verify(branchCard.deleteAsked)
            compare(root.asked, "", "the rows answered, so nobody was asked")
            compare(deleteRow.holdPresses, Metrics.holdPressesWarned,
                    "and the row is already the warning's three")

            root.standOn({ merged: "" })
            verify(!branchCard.deleteAnswered)
            verify(branchCard.deleteAsked)
            compare(root.asked, "check feature/topic-a")

            // git's late answer dresses the row the same way, by name.
            branchCard.checkedBranch = "feature/topic-a"
            branchCard.checkedMerged = "no"
            verify(deleteRow.holdPresses > 0)
            branchCard.checkedMerged = "unknown"
            compare(deleteRow.holdPresses, Metrics.holdPressesPlain, "a read that fell over is not a refusal")

            // And a card over a row nobody can delete asks nothing at all.
            root.standOn({ offers: root.current })
            verify(!branchCard.deleteAsked)
            compare(root.asked, "")
        }

        function test_each_row_asks_for_the_write_in_the_words_it_takes() {
            root.standOn({})
            // Pressed through as the plain delete: the menu stays up for git's answer (`BranchDeleteRow`'s `onHeld`).
            deleteRow.held()
            compare(root.asked, "delete branch feature/topic-a")

            root.standOn({})
            branchCard.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a origin/feature/topic-a true")

            root.standOn({})
            branchCard.unsetUpstreamItem.held()
            compare(root.asked, "unset feature/topic-a")

            // What reaches the remote is leased to the reading's own commit.
            root.standOn({})
            remoteDeleteRow.held()
            compare(root.asked, "delete-remote origin/feature/topic-a def456")

            root.standOn({})
            bothDeleteRow.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a false def456")

            // Once the local half is `-D`, the pair runs the forced form.
            root.standOn({})
            branchCard.forceDeleteBranch = "feature/topic-a"
            bothDeleteRow.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a true def456")

            // A remote row's own delete goes to the reading it names, leased to the commit the row stands on.
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", remoteCounterpartOid: "", offers: ["delete"] })
            deleteRow.held()
            compare(root.asked, "delete-remote origin/main abc123")
        }
    }
}
