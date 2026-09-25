import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The real `RefTagMenu` standing on the answers a menu would have read (`RefRowMenu.tagFacts`): which row has a seat,
// which greys and why, which push form is drawn, and what each press asks for. Which rows a state may offer is
// `offers::ref_menu`'s and is not recomputed here.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the card runs nothing itself.
    property string asked: ""

    /// Core's words (`offers::RefMenuOffers::words`) for a tag on each of its sides — data; the mapping is tested in
    /// `offers::tests::each_delete_row_needs_the_side_it_names`.
    readonly property var here: ["branch-here", "integrate", "delete", "push-tag"]
    readonly property var overThere: ["branch-here", "integrate", "delete-remote-tag"]
    readonly property var both: ["branch-here", "integrate", "delete", "push-tag", "delete-remote-tag",
                                 "delete-tag-everywhere"]
    /// The remote's copy on another commit: core takes both rows that reach the remote away.
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

        /// Read off `offered`: a card missing a row frames exactly like one that never offered it.
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
            for (const row of [card.deleteTagItem, card.deleteRemoteTagItem, card.deleteTagBothItem]) {
                if (row.offered)
                    compare(row.blockedReason, "", "a row with a side to name is pressable")
            }
        }

        /// A plain push refuses a name the remote has on another commit, so that case is the held, leased overwrite
        /// (デザイン規約 §相手の履歴を置き換える).
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

        /// The card's own rule, not core's: there is a name over there and only this row can say where it is, so the
        /// two rows core took away stand greyed (デザイン規約 §左メニューの所作 の削除の表). Any other missing offer
        /// takes the seat with it.
        function test_a_drifted_reading_keeps_the_two_rows_that_reach_it_and_greys_them() {
            root.standOn(root.drifted, "deadbee", false)
            verify(card.deleteRemoteTagItem.offered, "the seat is kept")
            verify(card.deleteTagBothItem.offered)
            compare(card.deleteRemoteTagItem.blockedReason, Words.remoteOnAnotherCommit)
            compare(card.deleteTagBothItem.blockedReason, Words.remoteOnAnotherCommit)
            verify(card.deleteTagItem.offered, "while the local delete is pressable")
            compare(card.deleteTagItem.blockedReason, "")
        }

        /// A stash is nobody's history, so it has no card at all.
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

        /// From a remote and a name the write cannot tell whether the sidebar row goes or only loses its badge, so
        /// the row's reading rides along (デザイン規約 §消す操作は先に画面から消す).
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
