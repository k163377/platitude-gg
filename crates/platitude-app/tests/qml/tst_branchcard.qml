import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The real branch card (`RefBranchMenu`) standing on the answers a menu would hand it (`RefRowMenu.branchFacts`):
// which rows it keeps, what each says, the gesture and colour each takes, and what a press asks for. Which rows a
// state may offer is `offers::ref_menu`'s and tested there.
//
// The greyed rows' sentences are judged here: a wrong reason frames like a right one.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the card runs nothing itself.
    property string asked: ""

    /// Word lists as `offers::RefMenuOffers::words` answers them — data, not a rule.
    readonly property var free: ["switch", "branch-here", "integrate", "delete", "delete-remote", "set-upstream"]
    /// The branch the working tree is on: no move, no delete, and the rows say so.
    readonly property var current: ["branch-here", "integrate", "set-upstream", "current"]
    /// One another working copy holds: `switch` stays and leads there without asking, the delete goes.
    readonly property var held: ["switch", "branch-here", "integrate", "set-upstream"]
    /// A reading standing on another commit: the local delete stays, the one that reaches over there goes.
    readonly property var drifted: ["switch", "branch-here", "integrate", "delete", "set-upstream"]
    /// Something running: every write is out at once.
    readonly property var busy: ["switch"]

    RefBranchMenu {
        id: card
        onDeleteRequested: (kind, id, name, oidHex) => root.asked = "delete " + kind + " " + id
        onUpstreamRequested: (branch, counterpart) => root.asked = "upstream " + branch + " " + counterpart
        onCheckDeleteRequested: branch => root.asked = "check " + branch
        onForceDeleteRequested: branch => root.asked = "force " + branch
        onDeleteRemoteRequested: remoteRef => root.asked = "delete-remote " + remoteRef
        onDeleteEverywhereRequested: (branch, remoteRef, forced) =>
            root.asked = "delete-both " + branch + " " + remoteRef + " " + forced
    }

    /// `merged` is what the drawn rows already say (`yes` / `no`, empty when only git can answer).
    function standOn(fields) {
        const all = { "kind": "branch", "name": "feature/topic-a", "full": "feature/topic-a",
                      "heldByWorktree": "", "holderLeaf": "", "remoteCounterpart": "origin/feature/topic-a",
                      "remoteDrifted": false, "open": true, "merged": "yes", "offers": root.free }
        for (const key in fields)
            all[key] = fields[key]
        root.asked = ""
        card.standOn(all.kind, all.name, all.full, "abc123", {
            "heldByWorktree": all.heldByWorktree,
            "holderLeaf": all.holderLeaf,
            "remoteCounterpart": all.remoteCounterpart,
            "remoteDrifted": all.remoteDrifted,
            "open": all.open,
            "merged": all.merged,
            "offers": all.offers
        })
    }

    TestCase {
        name: "BranchCard"
        when: windowShown

        /// A branch's three delete forms are a fixed table whose rows stay and grey (デザイン規約 §メニュー).
        function test_why_each_row_of_a_branchs_table_is_out_data() {
            return [
                {
                    tag: "the branch the tree is on",
                    fields: { offers: root.current },
                    reason: "Switch away first — this is the branch you are on",
                },
                {
                    tag: "held by another working copy",
                    fields: { offers: root.held, heldByWorktree: "C:/copies/topic", holderLeaf: "topic" },
                    reason: "Checked out in another working copy — topic",
                    // The one reason naming something outside this repository, so the tip gets a word for the tree
                    // mark (デザイン規約 §ref の種別).
                    marks: "topic",
                },
                {
                    tag: "something running",
                    fields: { offers: root.busy },
                    reason: Words.otherCommandRunning,
                },
            ]
        }

        function test_why_each_row_of_a_branchs_table_is_out(data) {
            root.standOn(data.fields)
            verify(card.deleteItem.offered, "the table keeps its seats")
            compare(card.deleteItem.blockedReason, data.reason)
            compare(card.deleteBothItem.blockedReason, data.reason, "and the pair names the half that runs first")
            const marks = data.marks === undefined ? "" : data.marks
            compare(card.deleteItem.tipMarkWord, marks, "the word the tip stands a mark against")
            compare(card.deleteBothItem.tipMarkWord, marks)
        }

        function test_a_drifted_reading_is_out_while_the_local_delete_is_not() {
            root.standOn({ offers: root.drifted, remoteDrifted: true })
            compare(card.deleteItem.blockedReason, "", "the branch here goes")
            compare(card.deleteRemoteItem.blockedReason, Words.remoteOnAnotherCommit)
            compare(card.deleteBothItem.blockedReason, Words.remoteOnAnotherCommit)
        }

        function test_a_branch_nothing_holds_back_offers_all_three() {
            root.standOn({})
            for (const row of [card.deleteItem, card.deleteRemoteItem, card.deleteBothItem]) {
                verify(row.offered)
                compare(row.blockedReason, "")
            }
            compare(card.deleteItem.code, "branch --delete")
            compare(card.deleteRemoteItem.code, "push --delete")
            compare(card.deleteItem.holdMs, 0, "the everyday delete is a click; git is what may still refuse it")
            verify(card.deleteRemoteItem.holdMs > 0, "reaching past this machine is held")
            compare(card.deleteRemoteItem.holdTone, Theme.warning)
            compare(card.deleteBothItem.holdTone, Theme.warning, "while the local half is not a force delete")
        }

        /// It writes configuration, which the current branch and a held one still take (offers::ref_menu).
        function test_the_upstream_row_follows_its_own_word() {
            root.standOn({})
            verify(card.upstreamItem.offered)
            root.standOn({ offers: root.current })
            verify(card.upstreamItem.offered, "the branch the tree is on writes settings like any other")
            root.standOn({ offers: root.busy })
            verify(!card.upstreamItem.offered, "and nothing writes while something is running")
        }

        /// A remote-tracking row keeps the assembled rule; its own delete is the held one reaching past this machine.
        function test_a_remote_row_keeps_only_the_rows_it_can_name() {
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: ["switch", "branch-here", "integrate", "delete"] })
            verify(card.applies)
            verify(card.deleteItem.offered)
            compare(card.deleteItem.code, "push --delete")
            verify(card.deleteItem.holdMs > 0)
            compare(card.deleteItem.holdTone, Theme.warning)
            verify(!card.deleteRemoteItem.offered, "a reading has no reading of its own")
            verify(!card.deleteBothItem.offered)
            verify(!card.upstreamItem.offered, "and no setting of its own either")
        }

        function test_a_row_that_names_no_branch_has_no_card() {
            root.standOn({ kind: "tag", name: "v1.0", full: "v1.0", offers: [] })
            verify(!card.applies)
        }

        /// git's refusal lands while the card stands: the row turns into the held `-D` under the hand
        /// (デザイン規約 §左メニューの所作), and the pair follows.
        function test_a_refusal_turns_the_row_into_the_held_force_delete() {
            root.standOn({})
            compare(card.deleteItem.code, "branch --delete")
            compare(card.deleteItem.note, "")
            compare(card.deleteBothItem.note, "")

            card.forceDeleteBranch = "feature/topic-a"
            verify(card.deleteItem.holdMs > 0, "a click is no longer enough")
            compare(card.deleteItem.holdTone, Theme.danger, "what goes lives nowhere else")
            compare(card.deleteBothItem.note, "not merged")
            compare(card.deleteBothItem.holdTone, Theme.danger, "so the pair is danger too")
            // With no press under way, the chip and the note already say what the row now runs.
            compare(card.deleteItem.code, "branch -D")
            compare(card.deleteItem.note, "not merged")
        }

        /// Where the drawn rows cannot answer, git is asked — by request, like any other.
        function test_the_early_answer_comes_off_the_rows_or_is_asked_of_git() {
            root.standOn({ merged: "no" })
            verify(card.deleteAnswered)
            verify(!card.deleteMerged)
            verify(card.deleteAsked)
            compare(root.asked, "", "the rows answered, so nobody was asked")
            verify(card.deleteItem.holdMs > 0, "and the row is already the held one")

            root.standOn({ merged: "" })
            verify(!card.deleteAnswered)
            verify(card.deleteAsked)
            compare(root.asked, "check feature/topic-a")

            // git's late answer dresses the row the same way, by name.
            card.checkedBranch = "feature/topic-a"
            card.checkedMerged = "no"
            verify(card.deleteItem.holdMs > 0)
            card.checkedMerged = "unknown"
            compare(card.deleteItem.holdMs, 0, "a read that fell over is not a refusal")

            // And a card over a row nobody can delete asks nothing at all.
            root.standOn({ offers: root.current })
            verify(!card.deleteAsked)
            compare(root.asked, "")
        }

        function test_each_row_asks_for_the_write_in_the_words_it_takes() {
            root.standOn({})
            card.deleteItem.picked()
            compare(root.asked, "delete branch feature/topic-a")

            root.standOn({})
            card.upstreamItem.triggered()
            compare(root.asked, "upstream feature/topic-a origin/feature/topic-a")

            root.standOn({})
            card.deleteRemoteItem.held()
            compare(root.asked, "delete-remote origin/feature/topic-a")

            root.standOn({})
            card.deleteBothItem.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a false")

            // Once the local half is `-D`, the pair runs the forced form.
            root.standOn({})
            card.forceDeleteBranch = "feature/topic-a"
            card.deleteBothItem.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a true")

            // A remote row's own delete goes to the reading it names.
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: ["delete"] })
            card.deleteItem.held()
            compare(root.asked, "delete-remote origin/main")
        }
    }
}
