import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The rows a tag's name grows, and what a press on one asks for: **the real card** (`RefTagMenu`), standing on the
// answers a menu carrying it would have read (`RefRowMenu.tagFacts`).
//
// **Core decides, the card draws.** Which rows a repository's state may offer is `offers::ref_menu`, said in words
// and covered there for every side a name can stand on; nothing here recomputes it. What is read back here is what
// the card does with those words — which row has a seat, which of them greys and why, which of the push row's two
// forms is drawn, and what each press asks the menu to run.
//
// **The drifted reading is the card's own rule**, and core has no part in it: a remote carrying the name on another
// commit leaves the two rows that reach it standing and out, because there *is* a name over there and this row is
// the only place that can say where it is standing (デザイン規約 §左メニューの所作 の削除の表).
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the card runs nothing itself.
    property string asked: ""

    /// The words core answers with for a tag standing on each of its sides, the list
    /// `offers::RefMenuOffers::words` is. **Data, not a rule** — the mapping from a repository's state to these is
    /// core's and is tested there (`offers::tests::each_delete_row_needs_the_side_it_names`).
    readonly property var here: ["branch-here", "integrate", "delete", "push-tag"]
    readonly property var overThere: ["branch-here", "integrate", "delete-remote-tag"]
    readonly property var both: ["branch-here", "integrate", "delete", "push-tag", "delete-remote-tag",
                                 "delete-tag-everywhere"]
    /// The same name with the remote's copy on another commit: core keeps the local delete and the push and takes
    /// both rows that reach the remote away.
    readonly property var drifted: ["branch-here", "integrate", "delete", "push-tag"]

    RefTagMenu {
        id: card
        canBranchHere: true
        onTagHereRequested: oidHex => root.asked = "tag-here " + oidHex
        onPushTagRequested: (remote, tag, lease) => root.asked = "push " + remote + " " + tag + " " + lease
        onDeleteTagRequested: tag => root.asked = "delete " + tag
        onDeleteRemoteTagRequested: (remote, tag, onlyThere) =>
            root.asked = "delete-remote " + remote + " " + tag + " " + onlyThere
        onDeleteTagEverywhereRequested: (tag, remote) => root.asked = "delete-both " + tag + " " + remote
    }

    /// The card standing on one of those answers, the way a menu hands it over.
    function standOn(offers, drift, onlyThere) {
        root.asked = ""
        card.standOn("tag", "v1.0", "abc123", {
            "pushRemote": "origin",
            "tagDriftOid": drift,
            "tagOnlyThere": onlyThere,
            "offers": offers
        })
    }

    TestCase {
        name: "TagCard"
        when: windowShown

        /// One row per side the name stands on, told apart by which of them is drawn at all: a card missing one
        /// frames exactly like a card that never offered it.
        function test_the_delete_rows_a_name_grows_data() {
            return [
                { tag: "made here", offers: root.here, local: true, remote: false, both: false },
                { tag: "only over there", offers: root.overThere, local: false, remote: true, both: false },
                { tag: "on both sides", offers: root.both, local: true, remote: true, both: true },
            ]
        }

        function test_the_delete_rows_a_name_grows(data) {
            root.standOn(data.offers, "", false)
            compare(card.deleteTagItem.offered, data.local, "tag --delete")
            compare(card.deleteRemoteTagItem.offered, data.remote, "push --delete")
            compare(card.deleteTagBothItem.offered, data.both, "both at once")
            // And nothing is out for a reason it does not have.
            for (const row of [card.deleteTagItem, card.deleteRemoteTagItem, card.deleteTagBothItem]) {
                if (row.offered)
                    compare(row.blockedReason, "", "a row with a side to name is pressable")
            }
        }

        /// The push row's two forms. A name the remote already has on another commit is refused outright by a plain
        /// push, so that case comes up as the leased overwrite: the long spelling, held, and pinned to the commit
        /// that was being shown (デザイン規約 §相手の履歴を置き換える).
        function test_the_push_row_takes_its_second_form_over_a_drifted_reading() {
            root.standOn(root.here, "", false)
            verify(card.pushTagItem.offered)
            compare(card.pushTagItem.code, "push")
            compare(card.pushTagItem.holdMs, 0, "an ordinary row: it adds a name and takes nothing away")

            root.standOn(root.drifted, "deadbee", false)
            verify(card.pushTagItem.offered)
            verify(card.pushTagItem.holdMs > 0, "the leased overwrite is held")
            compare(card.pushTagItem.holdTone, Theme.warning, "reaching past this machine is the warning tone")
        }

        /// **The card's own rule.** Core has taken both rows that reach the remote away over a drifted reading, and
        /// the card puts them back standing and out: there is a name over there, and this is the only place that can
        /// say where it is. Everything else that takes the offer away takes the seat with it.
        function test_a_drifted_reading_keeps_the_two_rows_that_reach_it_and_greys_them() {
            root.standOn(root.drifted, "deadbee", false)
            verify(card.deleteRemoteTagItem.offered, "the seat is kept")
            verify(card.deleteTagBothItem.offered)
            compare(card.deleteRemoteTagItem.blockedReason, Words.remoteOnAnotherCommit)
            compare(card.deleteTagBothItem.blockedReason, Words.remoteOnAnotherCommit)
            verify(card.deleteTagItem.offered, "while the local delete is pressable")
            compare(card.deleteTagItem.blockedReason, "")
        }

        /// A row that names no tag holds `Create tag here…` alone, and a stash takes the card off the menu: it is
        /// nobody's history, so there is nothing to mark and no card to open.
        function test_a_row_that_names_no_tag_holds_the_one_row_about_the_commit() {
            root.asked = ""
            card.standOn("branch", "feature/topic-a", "abc123",
                         { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] })
            verify(card.applies, "a branch row still offers the mark")
            verify(card.tagHereItem.offered)
            for (const row of [card.pushTagItem, card.deleteTagItem, card.deleteRemoteTagItem,
                               card.deleteTagBothItem]) {
                verify(!row.offered, "nothing a name answers for")
            }
            card.standOn("stash", "stash@{0}", "abc123",
                         { "pushRemote": "origin", "tagDriftOid": "", "tagOnlyThere": false, "offers": [] })
            verify(!card.applies, "and a stash has no card at all")
        }

        /// What each press asks for, in the words the write takes — the card runs none of them itself.
        function test_each_row_asks_for_the_write_in_the_words_it_takes() {
            root.standOn(root.both, "", false)
            card.tagHereItem.triggered()
            compare(root.asked, "tag-here abc123")

            root.standOn(root.both, "", false)
            card.pushTagItem.triggered()
            compare(root.asked, "push origin v1.0 ", "the plain push carries no lease")

            root.standOn(root.drifted, "deadbee", false)
            card.pushTagItem.held()
            compare(root.asked, "push origin v1.0 deadbee", "and the held one is pinned to the commit shown")

            root.standOn(root.both, "", false)
            card.deleteTagItem.held()
            compare(root.asked, "delete v1.0")

            root.standOn(root.both, "", false)
            card.deleteTagBothItem.held()
            compare(root.asked, "delete-both v1.0 origin")
        }

        /// The remote delete carries the row's own reading of which sides the name stood on: a name only the remote
        /// had leaves the sidebar with it, one held here keeps its row and loses the badge, and the write cannot
        /// tell the two apart from a remote and a name (デザイン規約 §消す操作は先に画面から消す).
        function test_the_remote_delete_carries_whether_the_name_was_only_over_there() {
            root.standOn(root.overThere, "", true)
            card.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote origin v1.0 true")

            root.standOn(root.both, "", false)
            card.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote origin v1.0 false")
        }
    }
}
