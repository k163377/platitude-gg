import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The real `RefTagMenu` standing on the answers a menu would have read (`RefMenuFacts.stand`): which row has a seat,
// which greys and why, which push form is drawn, and what each press asks for. Which rows a state may offer, and which
// of them grey, is `offers::ref_menu`'s and is not recomputed here.
Item {
    id: root
    width: 320
    height: 240

    /// What a press asked for, last one wins: the card runs nothing itself.
    property string asked: ""

    /// Core's answer (`GitFacts.refMenu` — the part the card reads) for a tag on each of its sides — data; the rule is
    /// tested in `offers::tests::ref_menu::each_delete_row_needs_the_side_it_names`. The two deletes that reach over
    /// there come as seats: `live`, `greyed`, or empty for none.
    readonly property var here: ({ "pushTag": true, "delete": true, "deleteRemoteTag": "", "deleteTagEverywhere": "" })
    readonly property var overThere: ({ "pushTag": false, "delete": false, "deleteRemoteTag": "live",
                                        "deleteTagEverywhere": "" })
    readonly property var both: ({ "pushTag": true, "delete": true, "deleteRemoteTag": "live",
                                   "deleteTagEverywhere": "live" })
    /// A reading held back — the remote's copy on another commit, or several remotes carrying the name — keeps both
    /// rows that reach over there in their seats, greyed.
    readonly property var drifted: ({ "pushTag": true, "delete": true, "deleteRemoteTag": "greyed",
                                      "deleteTagEverywhere": "greyed" })

    RefTagMenu {
        id: card
        canBranchHere: true
        onTagHereRequested: oidHex => root.asked = "tag-here " + oidHex
        onPushTagRequested: (remote, tag, lease) => root.asked = "push " + remote + " " + tag + " " + lease
        onDeleteTagRequested: tag => root.asked = "delete " + tag
        onDeleteRemoteTagRequested: (remote, tag, onlyThere, expect) =>
            root.asked = "delete-remote " + remote + " " + tag + " " + onlyThere + " " + expect
        onDeleteTagEverywhereRequested: (tag, remote, expect) =>
            root.asked = "delete-both " + tag + " " + remote + " " + expect
    }

    /// The card on core's answer `menu`, the tag's record as `NavSectionModel.tagMenu` answers for a name `origin`
    /// carries: the deletes reach it, and a drift holds them back. `over` replaces any of the record's fields, or
    /// `tagAt`. `again`: why the menu stands again with the card up (`RefTagMenu.standOn`'s — `follow` / `wait`), none
    /// as it is raised.
    function standOn(menu, drift, onlyThere, over, again) {
        root.asked = ""
        const tag = {
            "pushRemote": "origin",
            "lease": drift,
            "reach": "origin",
            "heldBack": drift !== "" ? "drift" : "",
            "carriers": "origin",
            "rowGoes": onlyThere
        }
        const facts = { "card": tag, "menu": menu, "tagAt": "abc123" }
        for (const key in (over || {})) {
            if (key === "tagAt")
                facts.tagAt = over[key]
            else
                tag[key] = over[key]
        }
        card.standOn("tag", "v1.0", "abc123", facts, again === undefined ? "" : again)
    }

    TestCase {
        name: "TagCard"
        when: windowShown

        // A shape held by a case that failed part way would hold the next case's rows.
        function cleanup() {
            card.releaseShape()
        }

        /// Read off `offered`: a card missing a row frames exactly like one that never offered it.
        function test_the_delete_rows_a_name_grows_data() {
            return [
                { tag: "made here", menu: root.here, local: true, remote: false, both: false },
                { tag: "only over there", menu: root.overThere, local: false, remote: true, both: false },
                { tag: "on both sides", menu: root.both, local: true, remote: true, both: true },
            ]
        }

        function test_the_delete_rows_a_name_grows(data) {
            root.standOn(data.menu, "", false)
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
            compare(card.pushTagItem.holdPresses, 0, "an ordinary row: it adds a name and takes nothing away")

            root.standOn(root.drifted, "deadbee", false)
            verify(card.pushTagItem.offered)
            verify(card.pushTagItem.holdPresses > 0, "the leased overwrite is held")
            compare(card.pushTagItem.holdTone, Theme.warning, "reaching past this machine is the warning tone")
        }

        /// There is a name over there and only this row can say where it is, so the two rows that reach it keep their
        /// seats greyed (デザイン規約 §左メニューの所作 の削除の表), and say why. A seat core does not give is no row.
        function test_a_drifted_reading_keeps_the_two_rows_that_reach_it_and_greys_them() {
            root.standOn(root.drifted, "deadbee", false)
            verify(card.deleteRemoteTagItem.offered, "the seat is kept")
            verify(card.deleteTagBothItem.offered)
            compare(card.deleteRemoteTagItem.blockedReason, "Differs from origin")
            compare(card.deleteTagBothItem.blockedReason, "Differs from origin")
            verify(card.deleteTagItem.offered, "while the local delete is pressable")
            compare(card.deleteTagItem.blockedReason, "")
        }

        /// Several remotes carry the name and none of them is the one this window acts on: no remote can be picked
        /// unasked, so the two rows reaching over there stand greyed and name who has it — each is reached from its
        /// own line (デザイン規約 §左メニューの所作 の削除の表).
        function test_a_name_several_remotes_carry_unasked_greys_the_rows_reaching_them() {
            root.standOn(root.drifted, "", false, { "reach": "", "heldBack": "unnamed", "carriers": "fork, mirror" })
            verify(card.deleteRemoteTagItem.offered, "the seat is kept")
            verify(card.deleteTagBothItem.offered)
            compare(card.deleteRemoteTagItem.blockedReason, "Several remotes have it: fork, mirror")
            compare(card.deleteTagBothItem.blockedReason, "Several remotes have it: fork, mirror")
            compare(card.deleteRemoteTagItem.codeName, "v1.0", "and names no remote it cannot reach")
            compare(card.deleteRemoteTagItem.nameMark, "tag", "behind its mark")

            root.standOn({ "pushTag": false, "delete": false, "deleteRemoteTag": "greyed", "deleteTagEverywhere": "" },
                         "", false, { "reach": "", "heldBack": "unnamed", "carriers": "fork, mirror" })
            verify(card.deleteRemoteTagItem.offered)
            verify(!card.deleteTagBothItem.offered, "a name not held here has no both")
        }

        /// The deletes need not reach where the pushes go — the one remote carrying a name origin lacks, or one the
        /// reader named — so the row says which.
        function test_the_remote_delete_names_the_remote_it_reaches() {
            root.standOn(root.both, "", false)
            compare(card.deleteRemoteTagItem.text, "v1.0 from origin")
            compare(card.pushTagItem.text, "to origin")

            root.standOn(root.overThere, "", true, { "reach": "fork", "carriers": "fork" })
            compare(card.deleteRemoteTagItem.text, "v1.0 from fork")
            card.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote fork v1.0 true abc123", "sent where the row said, leased to its commit")

            root.standOn(root.both, "", false, { "reach": "fork" })
            card.deleteTagBothItem.held()
            compare(root.asked, "delete-both v1.0 fork abc123")
        }

        /// A stash is nobody's history, so it has no card at all.
        function test_a_row_that_names_no_tag_holds_the_one_row_about_the_commit() {
            root.asked = ""
            card.standOn("branch", "feature/topic-a", "abc123", { "card": undefined, "tagAt": "" })
            verify(card.applies, "a branch row still offers the mark")
            verify(card.tagHereItem.offered)
            for (const row of [card.pushTagItem, card.deleteTagItem, card.deleteRemoteTagItem,
                               card.deleteTagBothItem]) {
                verify(!row.offered, "nothing a name answers for")
            }
            card.standOn("stash", "stash@{0}", "abc123", { "card": undefined, "tagAt": "" })
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
            compare(root.asked, "delete-both v1.0 origin abc123")
        }

        /// The push stays pinned to the remote and the reading the menu went up with: the name over there moved since
        /// — a fetch outside or the one the menu waited for brought another reading — the row keeps what it showed and
        /// greys asking for the menu to be raised again, since the held overwrite would replace a commit it never
        /// showed (デザイン規約 §メニュー「表示も押下も開いた時」).
        function test_standing_again_holds_the_push_lease_and_greys_a_moved_reading() {
            root.standOn(root.drifted, "deadbee", false)
            card.holdShape()
            root.standOn(root.drifted, "c0ffee", false, {}, "follow")
            compare(card.pushTagItem.refName, "origin")
            verify(card.pushTagItem.withdrawn, "the push greys where it stands")
            compare(card.pushTagItem.blockedWhy, card.pushTagItem.withdrawnWhy)
            card.pushTagItem.held()
            compare(root.asked, "push origin v1.0 deadbee", "a press that got through would still be the one shown")

            card.releaseShape()
            root.standOn(root.here, "", false)
            card.holdShape()
            root.standOn(root.drifted, "c0ffee", false, {}, "follow")
            verify(card.pushTagItem.withdrawn, "a plain push that would now overwrite is the reading moving too")
            compare(card.pushTagItem.holdPresses, 0, "and the row keeps the form it showed")

            card.releaseShape()
            root.standOn(root.drifted, "deadbee", false)
            card.holdShape()
            root.standOn(root.drifted, "c0ffee", false, {}, "wait")
            verify(card.pushTagItem.withdrawn, "and so when the command the menu waited for brought it")

            card.releaseShape()
            root.standOn(root.drifted, "c0ffee", false)
            verify(card.pushTagItem.offered, "a menu raised afresh stands on the reading as it is")
            card.pushTagItem.held()
            compare(root.asked, "push origin v1.0 c0ffee")
        }

        /// A push that stopped applying for a while — the tag deleted here with the menu up — keeps the lease it
        /// showed: made again on another reading, the row stays grey rather than offer an overwrite it never showed.
        function test_a_push_that_stopped_applying_keeps_the_lease_it_showed() {
            root.standOn(root.drifted, "deadbee", false)
            card.holdShape()
            root.standOn(root.overThere, "deadbee", true, {}, "follow")
            verify(card.pushTagItem.withdrawn, "gone here, the push greys")
            root.standOn(root.drifted, "c0ffee", false, {}, "follow")
            verify(card.pushTagItem.withdrawn, "made again on another reading, it stays grey")
            compare(card.state.tagDriftOid, "deadbee", "still on the lease it showed")
        }

        /// Presses under way on the push are made for the tag where it stood (`AppMenuItem.premise`): moved here part
        /// way (`git tag -f`), the presses so far count for nothing, and the ones left over fire nothing.
        function test_presses_under_way_count_for_nothing_once_the_tag_moves_here() {
            root.standOn(root.drifted, "deadbee", false)
            card.holdShape()
            card.pushTagItem.holdDrive.begin()
            card.pushTagItem.holdDrive.letUp()
            root.standOn(root.drifted, "deadbee", false, { "tagAt": "abc999" }, "follow")
            verify(card.pushTagItem.holdDrive.stale, "the presses were made for the tag where it was")
            for (let i = 1; i < Metrics.holdPressesWarned; i++) {
                card.pushTagItem.holdDrive.begin()
                card.pushTagItem.holdDrive.letUp()
            }
            compare(root.asked, "", "so the presses left over fire nothing")
        }

        /// From a remote and a name the write cannot tell whether the sidebar row goes or only loses its badge, so
        /// the row's reading rides along (デザイン規約 §消す操作は先に画面から消す).
        function test_the_remote_delete_carries_whether_the_name_was_only_over_there() {
            root.standOn(root.overThere, "", true)
            card.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote origin v1.0 true abc123")

            root.standOn(root.both, "", false)
            card.deleteRemoteTagItem.held()
            compare(root.asked, "delete-remote origin v1.0 false abc123")
        }
    }
}
