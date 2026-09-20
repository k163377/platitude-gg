import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The delete table a branch's name grows, and what a press on one asks for: **the real card** (`RefBranchMenu`),
// standing on the answers a menu carrying it would have read (`RefRowMenu.branchFacts`).
//
// **Core decides, the card draws.** Which rows a repository's state may offer is `offers::ref_menu`, said in words
// and covered there; nothing here recomputes it. What is read back is what the card does with those words — the
// branch's fixed table where rows stay and grey, the assembled rule the other kinds keep, which gesture each row
// takes, the colour it wears, and the swap to the held `-D` when git's refusal lands while the card stands.
//
// **The sentences are the half nothing else can read.** A greyed row that greyed for the wrong reason frames exactly
// like one that greyed for the right one, and the two the card writes itself carry an em dash — which a headless
// run's `must_say` can never match on Windows (verify-ui §Windows での実行・デバッグの罠). So until this file they
// were judged nowhere at all.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the card runs nothing itself.
    property string asked: ""

    /// The words core answers with, packed as `offers::RefMenuOffers::words` packs them. **Data, not a rule** — what
    /// a repository's state comes to is core's and is tested there.
    readonly property string free: "switch branch-here integrate delete delete-remote set-upstream"
    /// The branch the working tree is on: no move, no delete, and the rows say so.
    readonly property string current: "branch-here integrate set-upstream current"
    /// One another working copy holds: `switch` stays and asks, the delete does not.
    readonly property string held: "switch asks branch-here integrate set-upstream"
    /// A reading standing on another commit: the local delete stays, the one that reaches over there goes.
    readonly property string drifted: "switch branch-here integrate delete set-upstream"
    /// Something running: every write is out at once.
    readonly property string busy: "switch asks"

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

    /// The card standing on one of those answers, the way a menu hands it over. `merged` is what the drawn rows
    /// already say (`yes` / `no`, empty for a branch only git can answer for).
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

        /// **A branch's three delete forms are a fixed table**: rows that cannot be chosen stay and grey out, so the
        /// current branch's card opens on rows saying why rather than on nothing at all (デザイン規約 §メニュー、by
        /// design). What each says is the point — a row out for the wrong reason frames like one out for the right
        /// one.
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
                    // The one line of the three that names something from outside this repository, so the one that
                    // hands the tip a word to stand the tree mark against (デザイン規約 §ref の種別).
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

        /// The reading over there is the one row with a reason of its own, and **the local delete can be pressable at
        /// the same time** — which is the pair the two runs of `delete-blocked-tip` were a pair for.
        function test_a_drifted_reading_is_out_while_the_local_delete_is_not() {
            root.standOn({ offers: root.drifted, remoteDrifted: true })
            compare(card.deleteItem.blockedReason, "", "the branch here goes")
            compare(card.deleteRemoteItem.blockedReason, Words.remoteOnAnotherCommit)
            compare(card.deleteBothItem.blockedReason, Words.remoteOnAnotherCommit)
        }

        /// Nothing is out when nothing is in the way, and the words each row wears are the command it runs.
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

        /// The setting above the table follows its own word and nothing else: it writes configuration about a branch,
        /// which the branch the tree is on and one another copy holds both still take (offers::ref_menu).
        function test_the_upstream_row_follows_its_own_word() {
            root.standOn({})
            verify(card.upstreamItem.offered)
            root.standOn({ offers: root.current })
            verify(card.upstreamItem.offered, "the branch the tree is on writes settings like any other")
            root.standOn({ offers: root.busy })
            verify(!card.upstreamItem.offered, "and nothing writes while something is running")
        }

        /// A remote-tracking row keeps the assembled rule instead: only the rows it can name, and its own delete is
        /// the held one that reaches past this machine.
        function test_a_remote_row_keeps_only_the_rows_it_can_name() {
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: "switch branch-here integrate delete" })
            verify(card.applies)
            verify(card.deleteItem.offered)
            compare(card.deleteItem.code, "push --delete")
            verify(card.deleteItem.holdMs > 0)
            compare(card.deleteItem.holdTone, Theme.warning)
            verify(!card.deleteRemoteItem.offered, "a reading has no reading of its own")
            verify(!card.deleteBothItem.offered)
            verify(!card.upstreamItem.offered, "and no setting of its own either")
        }

        /// A row that names no branch takes the card off the menu — the same answer from either entrance.
        function test_a_row_that_names_no_branch_has_no_card() {
            root.standOn({ kind: "tag", name: "v1.0", full: "v1.0", offers: "" })
            verify(!card.applies)
        }

        /// **git's refusal lands while the card stands**, and the row it lands on turns into the held `-D` where the
        /// hand already is (デザイン規約 §左メニューの所作). The pair follows it, note and colour together.
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
            // The chip and the note follow the length the press was given, which is the live one while no press is
            // under way — so the row says what it now runs before a hand reaches it.
            compare(card.deleteItem.code, "branch -D")
            compare(card.deleteItem.note, "not merged")
        }

        /// The answer the drawn rows already hold dresses the row as the card opens, with nobody asked. Where they
        /// cannot answer, git is — and that ask is a request like any other.
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

        /// What each press asks for, in the words the write takes.
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

            // Once the local half goes as `-D` the pair says so, so the entrance runs the forced form.
            root.standOn({})
            card.forceDeleteBranch = "feature/topic-a"
            card.deleteBothItem.held()
            compare(root.asked, "delete-both feature/topic-a origin/feature/topic-a true")

            // A remote row's own delete goes to the reading it names.
            root.standOn({ kind: "remote", name: "origin/main", full: "origin/main",
                           remoteCounterpart: "", offers: "delete" })
            card.deleteItem.held()
            compare(root.asked, "delete-remote origin/main")
        }
    }
}
