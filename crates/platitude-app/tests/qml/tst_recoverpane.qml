import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The discard log as it stands over the left menu (`RecoverPane`): nothing the hand does over it
// reaches the menu underneath, its `✕` is the band's whole right end, a ring says the reflogs are being read, an empty
// answer says so, a rest on an entry opens its card, and the keyboard it takes from the graph walks nothing there.
Item {
    id: root
    width: 300
    height: 400

    /// How many Escapes reached the page's own handler.
    property int escapes: 0

    /// What lies under the list in the left menu — the filter, its fold block, the sections. Anything it hears through
    /// the list is a leak; the strip right of the list leaves some of it bare, for the hand to start from.
    MouseArea {
        id: under
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        property int presses: 0
        property int wheels: 0
        onPressed: under.presses++
        onWheel: wheel => under.wheels++
    }

    readonly property RecoverEntries words: RecoverEntries {
        id: words
    }

    /// The page's side of the list, in the shape `RecoverPane` and `RecoverToggle` read.
    readonly property QtObject page: QtObject {
        id: page
        property var recoverEntries: []
        property int recoverPick: -1
        property bool recoverAnswered: true
        property bool recoverReading: false
        property bool recoverFailed: false
        property string recoverError: ""
        property bool recoverOpen: true
        property bool recoverLit: false
        property var recoverWords: words
        property real recoverNow: 1790000000
        function toggleRecover() {
            page.recoverOpen = !page.recoverOpen
        }
        /// The entries pressed since a case cleared this, in order.
        property var pickPresses: []
        function pickRecover(index) {
            page.pickPresses = page.pickPresses.concat([index])
            page.recoverPick = page.recoverPick === index ? -1 : index
        }
    }

    /// The page's shape around the list: a focus scope, which catches a keyboard let go in it, with its handler on the
    /// key's way up (`RepoPage`).
    FocusScope {
        anchors.fill: parent
        Keys.onEscapePressed: event => {
            root.escapes++
            event.accepted = true
        }
        RecoverPane {
            id: pane
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 260
            curPage: page
        }
    }

    TestCase {
        name: "RecoverPane"
        when: windowShown

        /// Two minutes before the moment the list reads its times against.
        function entry(title) {
            return { key: "reset " + title,
                     kind: "reset", mark: "branch", tint: Theme.accent, badged: false, paired: false, title: title,
                     titleMarks: [], listTitle: title, listMarks: [], at: page.recoverNow - 120, worktree: "",
                     worktreeMark: "tree",
                     parts: [{ mark: "branch", tint: Theme.accent, text: "2 commits",
                               restore: "as main-pgg-restored", restoreMarks: [], look: "commit" }] }
        }

        /// The rows the list holds on screen — not the ones it pooled — as `<key>@<y>`.
        function standingRows() {
            const kids = pane.entryList.contentItem.children
            const out = []
            for (let i = 0; i < kids.length; i++) {
                if (kids[i].key !== undefined && kids[i].visible && kids[i].pooled !== true)
                    out.push(kids[i].key + "@" + kids[i].y)
            }
            return out.join(",")
        }
        function init() {
            pane.visible = true
            page.recoverEntries = []
            // The list laid out empty before the case writes its own rows: emptied and filled again with no layout
            // between, the last case's delegates can stay standing.
            tryVerify(() => pane.entryList.count === 0 && standingRows() === "", undefined, "the list emptied")
            // A hand the last case left held (a read moved the rows under it while it read a card) lets go with a move
            // of its own, over the pane — its first place heard there is where the hold is measured from
            // (`HandWatch.stillHeld`).
            mouseMove(pane, pane.width / 2, pane.height - 4)
            mouseMove(pane, pane.width / 2 + 2, pane.height - 4)
            tryVerify(() => !pane.hand.stillHeld, undefined, "no hold left over")
            // The hand parked off the pane: rows read in under a hand that stayed would hold it
            // (`HandWatch.rowsMoved`).
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !pane.hand.known)
            page.recoverPick = -1
            page.recoverAnswered = true
            page.recoverReading = false
            page.recoverFailed = false
            page.recoverError = ""
            page.recoverOpen = true
            under.presses = 0
            under.wheels = 0
            root.escapes = 0
            pane.entryList.focus = false
            pane.filterBox.focus = false
            pane.filterText = ""
        }

        /// Every visible item under `item` that `pick` answers for, depth first — not inside a row the list pooled,
        /// which stays `visible` where it last stood, undrawn (`EntryRow.pooled`).
        function found(item, pick) {
            const out = []
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.pooled === true)
                    continue
                if (child.visible && pick(child))
                    out.push(child)
                out.push(...found(child, pick))
            }
            return out
        }
        /// Where each turning ring stands, down the pane.
        function rings() {
            return found(pane, item => item.kind === "spinner").map(ring => ring.mapToItem(pane, 0, 0).y)
        }

        function test_nothing_the_hand_does_over_the_list_reaches_the_menu_under_it() {
            page.recoverEntries = [entry("main reset")]
            verify(waitForRendering(pane))
            // The band's empty middle — between the seat and the `✕`, wherever the font puts the seat's end — the
            // strip along its foot where the filter runs on, and the ground below the rows.
            const seat = found(pane, item => item.captioned !== undefined)[0]
            const gap = (seat.width + pane.width - Theme.headerHeight) / 2
            verify(seat.width < gap - 1 && gap + 1 < pane.width - Theme.headerHeight, "the band has a gap")
            // Where, and whether the list is still up after the press there: the band's whole width is the close.
            for (const [x, y, stays] of [[gap, 10, false], [gap, Theme.headerHeight - 2, false], [gap, 300, true]]) {
                page.recoverOpen = true
                // Hover is handed out on the next frame: the hand starts on the bare menu, so a list that takes it
                // is seen taking it — the menu lets go.
                mouseMove(root, root.width - 10, y)
                tryVerify(() => under.containsMouse, undefined, "the bare menu has the hand")
                mouseMove(pane, x, y)
                tryVerify(() => !under.containsMouse, undefined, "hover reached the menu at " + [x, y])
                mouseClick(pane, x, y)
                compare(page.recoverOpen, stays, "a press at " + [x, y] + (stays ? " closed the list" : " missed the close"))
                mouseWheel(pane, x, y, 0, -120)
            }
            compare(under.presses, 0)
            compare(under.wheels, 0)
        }

        function test_the_close_is_the_bands_whole_right_end() {
            // The block's far corner, where the fold block's own corner lies under it.
            mouseClick(pane, pane.width - 2, 2)
            verify(!page.recoverOpen)
            page.recoverOpen = true
            mouseClick(pane, pane.width - Theme.headerHeight + 2, Theme.headerHeight - 2)
            verify(!page.recoverOpen)
            compare(under.presses, 0)
        }

        function test_a_ring_says_the_reflogs_are_being_read() {
            page.recoverAnswered = false
            page.recoverReading = true
            verify(waitForRendering(pane))
            const first = rings()
            compare(first.length, 1, "one ring before the first answer")
            verify(first[0] > Theme.headerHeight, "in the list, under the band")

            page.recoverEntries = [entry("main reset")]
            verify(waitForRendering(pane))
            const again = rings()
            compare(again.length, 1, "one ring over the entries already up")
            verify(again[0] < Theme.headerHeight, "beside the heading")

            page.recoverReading = false
            page.recoverAnswered = true
            compare(rings().length, 0)
        }

        function test_an_empty_answer_says_nothing_was_thrown_away() {
            tryVerify(() => found(pane, item => item.text === "Nothing was thrown away here").length === 1)
            const said = found(pane, item => item.text === "Nothing was thrown away here")
            const at = said[0].mapToItem(pane, 0, 0)
            verify(at.y > 2 * Theme.headerHeight, "in the list, under the filter")
            // Not a row: no seat's indent, centred across the list.
            fuzzyCompare(at.x + said[0].width / 2, pane.width / 2, 1)
        }

        function test_a_rest_on_an_entry_opens_its_card_and_the_row_keeps_its_time() {
            page.recoverEntries = [entry("feature/a-branch-name-long-enough-to-be-cut-at-the-time rebased")]
            page.recoverPick = 0
            verify(waitForRendering(pane))
            const titles = found(pane, item => item.pieces !== undefined && item.cutting !== undefined)
            compare(titles.length, 1, "one row")
            verify(titles[0].cutting, "its name cut — never opened out in the list")
            compare(found(pane, item => item.text === "2m ago").length, 1, "the row's time stays")
            verify(pane.restOnShown(0))
            tryVerify(() => pane.cardOpen, undefined, "the card opened")
            compare(pane.cardAt, 0)
        }

        /// Enough entries to overflow the list.
        function manyEntries(n) {
            const out = []
            for (let i = 0; i < n; i++)
                out.push(entry("main-" + i + " reset"))
            return out
        }
        /// The log as `entries`, laid out from the list's first row — its origin moves as rows come and go above what
        /// is on screen (Qt keeps those rows where they stand).
        function showEntries(entries) {
            page.recoverEntries = entries
            tryVerify(() => pane.entryList.count === entries.length && !pane.rowKeep.busy, undefined, "the rows are in")
            pane.entryList.positionViewAtBeginning()
            pane.entryList.forceLayout()
        }
        /// The list scrolled `rows` rows down from its first row — its origin read laid out, which a layout owed moves.
        function scrollTo(rows) {
            pane.entryList.forceLayout()
            pane.entryList.contentY = pane.entryList.originY + rows * Theme.rowHeight
            pane.entryList.forceLayout()
        }
        /// The row at `index` of the list, as the real pointer reaches it.
        function rowAt(index) {
            return pane.entryList.itemAtIndex(index)
        }
        function restOnRow(index) {
            tryVerify(() => rowAt(index) !== null, undefined, "row " + index + " is built")
            const row = rowAt(index)
            const list = pane.entryList
            const y = index * Theme.rowHeight + Theme.rowHeight / 2 - (list.contentY - list.originY)
            mouseMove(list, list.width / 2, y)
            tryVerify(() => row.handOn, undefined, "the hand is on the row")
            tryVerify(() => pane.cardOpen && pane.cardAt === index, undefined, "its card is out after the rest")
        }

        /// The rows read the hand off the pane, as the graph's do: the row the hand walked onto lights and opens its
        /// card after the rest.
        function test_a_hand_that_walks_to_an_entry_lights_it_and_opens_its_card() {
            showEntries(manyEntries(30))
            restOnRow(2)
        }

        /// The list scrolling under a still hand: the card goes, the row that came under the hand lights, and its card
        /// comes only once the hand has moved.
        function test_the_rows_scrolling_under_a_still_hand_open_nothing() {
            showEntries(manyEntries(30))
            restOnRow(2)
            scrollTo(3)
            tryVerify(() => !pane.cardOpen, undefined, "the card went with the move")
            verify(pane.hand.stillHeld, "the hand is held")
            // waits(paced): a card that must **not** come back under the still hand; the rest it would take.
            wait(Metrics.tipDelayMs * 2)
            verify(!pane.cardOpen, "nothing opened under a hand that did not move")
            const under = rowAt(5)
            verify(under !== null && under.handOn, "the row that came under the hand is lit")
            // A tremor is the hand's own move: the row's card comes after the rest.
            mouseMove(pane.entryList, pane.entryList.width / 2 + 2, 2 * Theme.rowHeight + Theme.rowHeight / 2)
            tryVerify(() => pane.cardOpen && pane.cardAt === 5, undefined,
                      "a move asks for the card of the row under it")
            verify(under.handOn, "and the row stays lit")
        }

        /// The entries the filter leaves typed under a still hand: the rows moving under it are the reader's — the card
        /// goes, and the row that came under the hand opens only once the hand has moved.
        function test_the_rows_a_filter_moves_under_a_still_hand_open_nothing() {
            showEntries(manyEntries(30))
            restOnRow(2)
            pane.filterText = "main-1"
            tryVerify(() => !pane.cardOpen, undefined, "the card went with the rows")
            verify(pane.hand.stillHeld, "the hand is held")
            // waits(paced): a card that must **not** come back under the still hand; the rest it would take.
            wait(Metrics.tipDelayMs * 2)
            verify(!pane.cardOpen, "nothing opened under a hand that did not move")
            pane.filterText = ""
        }

        /// A list of `n` entries with a fresh one written in at `at` — what a read of the log brings when something was
        /// thrown away since (the newest first).
        function withFresh(n, at) {
            const all = manyEntries(n)
            all.splice(at, 0, entry("fresh reset"))
            return all
        }
        /// The hand into the card that is out, as a hand walks into it to read — once the card stands at its size: it
        /// opens a few pixels wide, and the window pushes it back from its edge as it grows (`Popup.margins`). Read
        /// off its place, the same on two looks in a row: a frame waited for may never come over a scene at rest.
        function intoCard() {
            const card = pane.entryHoverCard
            let last = ""
            tryVerify(() => {
                const at = card.contentItem.mapToItem(root, 0, 0)
                const now = [at.x, at.y, card.contentItem.width, card.contentItem.height].join(",")
                const stood = card.opened && card.contentItem.width > 0 && now === last
                last = now
                return stood
            }, undefined, "the card stands at its size")
            const at = card.contentItem.mapToItem(root, card.contentItem.width / 2, card.contentItem.height / 2)
            mouseMove(root, at.x, at.y)
            tryVerify(() => card.pointerInside, undefined, "the hand is in the card")
        }

        /// A read that writes an entry in above the screen moves nothing under the hand (Qt keeps the rows on
        /// screen where they stand): the card stays, on the entry it was of — whose number moved
        /// (デザイン規約 §hover のツールチップ「外乱では閉じない」).
        function test_an_entry_read_in_above_the_screen_moves_nothing_under_the_hand() {
            showEntries(manyEntries(30))
            scrollTo(3)
            restOnRow(5)
            const row = rowAt(5)
            page.recoverEntries = withFresh(30, 0)
            tryVerify(() => !pane.rowKeep.busy, undefined, "the read is weighed")
            verify(pane.cardOpen, "the card stays")
            compare(pane.cardAt, 6, "on its entry, one down the log now")
            verify(row.handOn, "the hand is on the same row")
            verify(!pane.hand.stillHeld, "and nothing moved under it")
        }

        /// …and one written in between the top of the screen and the hand's row is taken back by the scroll: the row
        /// stays under the hand.
        function test_an_entry_read_in_above_the_hand_is_taken_back_by_the_scroll() {
            showEntries(manyEntries(30))
            scrollTo(3)
            restOnRow(6)
            const row = rowAt(6)
            const at = row.mapToItem(pane, 0, 0).y
            const from = pane.entryList.contentY
            page.recoverEntries = withFresh(30, 4)
            tryVerify(() => !pane.rowKeep.busy, undefined, "the read is weighed")
            compare(row.mapToItem(pane, 0, 0).y, at, "the row stands where it stood")
            compare(pane.entryList.contentY, from + Theme.rowHeight, "the list went down by the entry written in")
            verify(pane.cardOpen && pane.cardAt === 7, "the card stays on its entry")
            verify(!pane.hand.stillHeld, "nothing moved under the hand")
        }

        /// A list too short to scroll cannot take it back: the rows move under the hand as the read's, and the card the
        /// hand is reading stays where it stands — it does not follow its row from under the hand.
        function test_a_card_the_hand_is_in_stays_where_it_stands_as_a_read_moves_its_row() {
            showEntries(manyEntries(5))
            restOnRow(2)
            intoCard()
            const card = pane.entryHoverCard
            // Where it is drawn, the window's own push back from its edge included (`Popup.margins`).
            const at = card.contentItem.mapToItem(root, 0, 0)
            const wide = card.width
            page.recoverEntries = withFresh(5, 0)
            tryVerify(() => !pane.rowKeep.busy, undefined, "the read is weighed")
            verify(pane.hand.stillHeld, "the rows moved under the hand, as the read's")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(pane.cardOpen, "the card the hand is in stays")
            compare(card.width, wide, "as wide, saying the same")
            compare(card.contentItem.mapToItem(root, 0, 0), at, "where it stood")
            compare(pane.cardAt, 3, "of its entry")
        }

        /// …and with the hand on the row, not in its card, the row going from under the hand takes the card with it, as
        /// a scroll does: the row that came under the hand opens only once the hand has moved.
        function test_the_hand_on_the_row_lets_the_card_go_as_a_read_moves_the_row() {
            showEntries(manyEntries(5))
            restOnRow(2)
            page.recoverEntries = withFresh(5, 0)
            tryVerify(() => !pane.cardOpen, undefined, "the card went with its row's place")
            verify(pane.hand.stillHeld, "the hand is held")
            // waits(paced): a card that must **not** come under the still hand; the rest it would take.
            wait(Metrics.tipDelayMs * 2)
            verify(!pane.cardOpen, "nothing opened under a hand that did not move")
            tryVerify(() => rowAt(2) !== null && rowAt(2).handOn, undefined, "the row that came under it is lit")
            mouseMove(pane.entryList, pane.entryList.width / 2 + 2, 2 * Theme.rowHeight + Theme.rowHeight / 2)
            tryVerify(() => pane.cardOpen && pane.cardAt === 2, undefined,
                      "a move asks for the card of the row under it")
        }

        /// The keyboard the list takes from the graph: the arrows walk nothing there, and move no row of the view.
        function test_the_keyboard_comes_to_the_list_and_the_arrows_go_nowhere() {
            showEntries(manyEntries(30))
            pane.takeKeyboard()
            verify(pane.entryList.activeFocus, "the list has the keyboard")
            const y = pane.entryList.contentY
            const current = pane.entryList.currentIndex
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Down)
            compare(pane.entryList.currentIndex, current, "the arrows walked the rows")
            compare(pane.entryList.contentY, y, "the arrows moved the view")
        }

        /// A caret in the filter is where the reader is typing: the keyboard is here already.
        function test_a_caret_in_the_filter_stays_where_it_is() {
            pane.filterBox.forceActiveFocus()
            verify(pane.filterBox.activeFocus)
            pane.takeKeyboard()
            verify(pane.filterBox.activeFocus, "the caret was taken out of the filter")
        }

        /// Escape climbs to the page's handler from either place the keyboard can be in the list, so the next one is
        /// still heard.
        function test_escape_climbs_to_the_page_from_the_list_and_from_the_filter() {
            pane.takeKeyboard()
            keyClick(Qt.Key_Escape)
            compare(root.escapes, 1, "from the list")
            pane.filterBox.forceActiveFocus()
            keyClick(Qt.Key_Escape)
            compare(root.escapes, 2, "from the filter")
        }

        /// Off screen — the list shut, the left menu folded — it takes no keyboard, and lets go of what it held: Qt
        /// keeps active focus on an item it made invisible.
        function test_a_list_off_screen_takes_no_keyboard_and_lets_go_of_what_it_held() {
            pane.visible = false
            pane.takeKeyboard()
            verify(!pane.entryList.focus, "a list off screen took the keyboard")
            pane.visible = true
            pane.takeKeyboard()
            verify(pane.entryList.activeFocus)
            pane.visible = false
            verify(!pane.entryList.activeFocus, "the list kept the keyboard off screen")
            pane.visible = true
            verify(!pane.entryList.activeFocus, "the list took the keyboard back by coming on screen")
        }

        /// The band's `✕` holds the keyboard after its press, as a button does, and goes off screen with the list it
        /// closed: Space opens nothing again, and the next Escape still climbs to the page.
        function test_space_after_the_close_opens_nothing() {
            mouseClick(pane, pane.width - 2, 2)
            verify(!page.recoverOpen)
            pane.visible = false
            keyClick(Qt.Key_Space)
            verify(!page.recoverOpen, "Space on the hidden close opened the list again")
            keyClick(Qt.Key_Escape)
            compare(root.escapes, 1, "Escape went quiet")
        }

        /// An entry a read takes out of the log takes its card with it, the hand inside or not: what it said is gone.
        function test_an_entry_read_away_takes_its_card_with_it() {
            showEntries(manyEntries(5))
            restOnRow(2)
            intoCard()
            const left = manyEntries(5)
            left.splice(2, 1)
            page.recoverEntries = left
            tryVerify(() => !pane.cardOpen, undefined, "the card went with its entry")
        }

        /// The stand-in's own row, off the rows: whether it stands, for which entry, and at which edge of the list.
        function pinAt(place, title) {
            const pin = pane.pickStandIn
            tryCompare(pane, "pinPlace", place, undefined, "where the stand-in stands")
            verify(pin.visible, "it stands")
            compare(pin.entry.listTitle, title, "for the entry picked")
        }
        /// The list's y of the stand-in's middle.
        function pinMiddle() {
            return pane.pickStandIn.y + Theme.rowHeight / 2
        }

        /// The row picked scrolled off, its stand-in rides the edge it went out of, whole, and steps aside as the row
        /// comes back: never two of the same entry on screen.
        function test_the_picked_row_scrolled_off_stands_in_at_that_edge() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            pane.entryList.forceLayout()
            tryVerify(() => pane.pinned.key !== "", undefined, "the pick is weighed")
            compare(pane.pinPlace, "none", "the row is on screen")
            verify(!pane.pickStandIn.visible, "so nothing stands in for it")

            scrollTo(3.5)
            pinAt("top", "main-3 reset")
            compare(pane.pickStandIn.y, 0, "cut by the edge, it is whole there")
            scrollTo(10)
            pinAt("top", "main-3 reset")
            scrollTo(3)
            compare(pane.pinPlace, "none", "back on screen, whole")
            verify(!pane.pickStandIn.visible)

            page.recoverPick = 29
            scrollTo(0)
            pinAt("bottom", "main-29 reset")
            compare(pane.pickStandIn.y + pane.pickStandIn.height, pane.entryList.height, "on the foot of the list")
            pane.entryList.positionViewAtEnd()
            pane.entryList.forceLayout()
            compare(pane.pinPlace, "none", "at the list's end, the row is on screen")

            page.recoverPick = -1
            tryVerify(() => pane.pinned.key === "", undefined, "the pick is weighed")
            compare(pane.pinPlace, "none", "nothing picked, nothing stands in")
        }

        /// A read that moves the picked entry's number, the page moving the pick after it, leaves the stand-in on the
        /// entry.
        function test_a_read_above_the_picked_entry_leaves_the_stand_in_on_it() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            page.recoverEntries = withFresh(30, 0)
            page.recoverPick = 4
            tryVerify(() => !pane.rowKeep.busy && pane.pinned.at === 4, undefined, "the read is weighed")
            pinAt("top", "main-3 reset")
            compare(pane.pinned.row, 4, "its row one further down")
        }

        /// A filter leaving the picked entry's row out: the stand-in takes a row's height at the list's head, and the
        /// rows go down by it — the first of them whole under it. The same with a filter that leaves nothing, whose
        /// line stands under the seat too.
        function test_a_filter_that_leaves_the_picked_row_out_seats_the_stand_in() {
            // Rows left over the list's height, the first where it stood: a list too short to scroll, or a first row
            // gone with the filter, has the view placing the rows anew whatever the seat does.
            showEntries(withFresh(30, 4))
            page.recoverPick = 4
            pane.filterText = "main-"
            tryVerify(() => pane.pinSeated, undefined, "the row went with the filter")
            pinAt("seat", "fresh reset")
            compare(pane.pickStandIn.y, 0)
            tryVerify(() => rowAt(0) !== null && rowAt(0).mapToItem(pane.entryList, 0, 0).y === Theme.rowHeight,
                      undefined, "the first row the filter left is under the seat")

            pane.filterText = "zzz"
            const said = () => found(pane, item => item.text === "No results found")
            tryVerify(() => said().length === 1, undefined, "the filter left nothing")
            pinAt("seat", "fresh reset")
            const line = said()[0].mapToItem(pane.entryList, 0, 0).y
            verify(line >= pane.pickStandIn.height, "the line stands clear of the seat: " + line)

            pane.filterText = ""
            tryVerify(() => pane.pinned.row === 4, undefined, "the row is back")
            pane.entryList.forceLayout()
            compare(pane.entryList.topMargin, 0, "and the seat gone")
            compare(pane.entryList.contentY, pane.entryList.originY, "the rows up by the row it took")
            compare(pane.pinPlace, "none", "with the row on screen")
        }

        /// The seat opening with no row moving — the entry picked under a filter that already leaves it out, as a run
        /// does it — takes the rows down by itself all the same.
        function test_a_seat_opening_over_rows_at_rest_takes_them_down() {
            showEntries(withFresh(30, 4))
            pane.filterText = "main-"
            tryVerify(() => pane.shownEntries.count === 30, undefined, "the filter left the rest")
            pane.entryList.forceLayout()
            compare(rowAt(0).mapToItem(pane.entryList, 0, 0).y, 0, "the first row stands at the list's head")
            page.recoverPick = 4
            pinAt("seat", "fresh reset")
            tryVerify(() => rowAt(0).mapToItem(pane.entryList, 0, 0).y === Theme.rowHeight, undefined,
                      "the row is under the seat")
        }

        /// The hand on the stand-in is on the stand-in: the row under it neither lights nor asks, and the card that
        /// opens is the picked entry's. Off it, the hand is on the row again.
        function test_a_hand_on_the_stand_in_is_not_on_the_row_under_it() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            const under = rowAt(10)
            verify(under !== null, "a row lies under it")

            mouseMove(pane.entryList, pane.entryList.width / 2, pinMiddle())
            tryVerify(() => pane.pickStandIn.handOn, undefined, "the hand is on the stand-in")
            verify(!under.handOn, "not on the row under it")
            tryVerify(() => pane.cardOpen, undefined, "its card opens after the rest")
            compare(pane.entryHoverCard.entry.listTitle, "main-3 reset", "the picked entry's")

            // Off the pane first: the card hangs under the stand-in, over the rows the hand would come to.
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !pane.cardOpen && !pane.pickStandIn.handOn, undefined, "the hand left, and the card went")
            mouseMove(pane.entryList, pane.entryList.width / 2, 3 * Theme.rowHeight + Theme.rowHeight / 2)
            tryVerify(() => rowAt(13) !== null && rowAt(13).handOn, undefined, "the hand is on the row it came to")
        }

        /// The stand-in coming over the row under a still hand takes the hand, and going gives it back — with no row
        /// moving, so nothing but the stand-in says the hand has something else under it.
        function test_the_stand_in_coming_and_going_takes_a_still_hand_and_gives_it_back() {
            showEntries(manyEntries(30))
            scrollTo(10)
            mouseMove(pane.entryList, pane.entryList.width / 2, Theme.rowHeight / 2)
            tryVerify(() => rowAt(10) !== null && rowAt(10).handOn, undefined, "the hand is on the row")
            page.recoverPick = 3
            tryVerify(() => pane.pickStandIn.handOn, undefined, "the stand-in came under it")
            verify(!rowAt(10).handOn, "and the row under the stand-in has lost it")
            // Off again with a move of its own, so a hold left from its coming cannot answer for its going.
            mouseMove(pane.entryList, pane.entryList.width / 2 + 2, Theme.rowHeight / 2)
            tryVerify(() => !pane.hand.stillHeld, undefined, "the hand moved")
            page.recoverPick = -1
            tryVerify(() => rowAt(10).handOn, undefined, "the stand-in went: the row has it again")
            verify(!pane.pickStandIn.handOn)
            verify(pane.hand.stillHeld, "the row came under a hand that did not move")
            // waits(paced): a card that must **not** come under the still hand; the rest it would take.
            wait(Metrics.tipDelayMs * 2)
            verify(!pane.cardOpen, "so nothing opens until the hand moves")
        }

        /// The stand-in going from elsewhere is nothing to the hand: the row it is on keeps its card and its rest.
        function test_the_stand_in_going_from_elsewhere_leaves_the_card_under_the_hand() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            mouseMove(pane.entryList, pane.entryList.width / 2, 5.5 * Theme.rowHeight)
            tryVerify(() => pane.cardOpen && pane.cardAt === 15, undefined, "the row's card opens after the rest")
            // No press, which takes a hover card down of itself: the entry put down from the keyboard.
            page.recoverPick = -1
            tryCompare(pane, "pinPlace", "none")
            verify(!pane.hand.stillHeld, "nothing moved under the hand")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(pane.cardOpen, "the card stays")
        }

        /// A read leaves the card on the seat: its entry is still there and still on screen, with no row to its name
        /// (デザイン規約 §hover のツールチップ「外乱では閉じない」).
        function test_a_read_leaves_the_card_on_the_seated_stand_in() {
            showEntries(withFresh(30, 4))
            page.recoverPick = 4
            pane.filterText = "main-"
            tryCompare(pane, "pinPlace", "seat")
            mouseMove(pane.entryList, pane.entryList.width / 2, Theme.rowHeight / 2)
            tryVerify(() => pane.cardOpen, undefined, "the seat's card opens after the rest")
            const more = withFresh(30, 4)
            more.unshift(entry("main-newer reset"))
            page.recoverEntries = more
            page.recoverPick = 5
            tryVerify(() => !pane.rowKeep.busy, undefined, "the read is weighed")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(pane.cardOpen, "the card stays")
            compare(pane.cardAt, 5, "on its entry, one down the log now")
        }

        /// A read moves the picked entry's number and the page moves the pick after it, one at a time: the stand-in
        /// stays on the entry through that, its card up under the hand on it, and the hand's leaving still takes the
        /// card.
        function test_a_read_moving_the_picked_number_leaves_the_hand_on_the_stand_in() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            tryCompare(pane, "pinPlace", "top")
            mouseMove(pane.entryList, pane.entryList.width / 2, Theme.rowHeight / 2)
            tryVerify(() => pane.cardOpen, undefined, "its card opens after the rest")
            page.recoverEntries = withFresh(30, 0)
            page.recoverPick = 4
            tryVerify(() => !pane.rowKeep.busy, undefined, "the read is weighed")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(pane.cardOpen, "the card stays")
            compare(pane.cardAt, 4, "on its entry")
            verify(pane.pickStandIn.handOn, "with the hand on the stand-in")
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !pane.cardOpen, undefined, "the hand gone, the card goes")
        }

        /// The second click of a double-click is no press of its own: the stand-in gone with the first, it would pick
        /// the row that came under the hand, and on a row it would put down what the first picked.
        function test_a_double_click_is_one_press() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            tryCompare(pane, "pinPlace", "top")
            page.pickPresses = []
            // A delay, so the stand-in goes between the clicks as it does under a hand.
            mouseDoubleClickSequence(pane.entryList, pane.entryList.width / 2, Theme.rowHeight / 2, Qt.LeftButton,
                                     Qt.NoModifier, 10)
            compare(page.pickPresses, [3], "on the stand-in")
            compare(page.recoverPick, -1, "the entry put down, and no other picked")
            page.pickPresses = []
            mouseDoubleClickSequence(pane.entryList, pane.entryList.width / 2, 2.5 * Theme.rowHeight, Qt.LeftButton,
                                     Qt.NoModifier, 10)
            compare(page.pickPresses, [12], "on a row")
            compare(page.recoverPick, 12, "picked, and left picked")
        }

        /// A hand that trembles on a press is still a click, not a drag of the list (rules-refs/app-ui.md「行は渡された
        /// ドラッグを手放さない」).
        function test_a_hand_that_trembles_on_a_row_still_picks_it() {
            showEntries(manyEntries(30))
            scrollTo(10)
            const from = pane.entryList.contentY
            const x = pane.entryList.width / 2
            const y = 2 * Theme.rowHeight + 1
            mousePress(pane.entryList, x, y)
            // In steps, as a hand moves: past the drag distance the list takes the press, and still inside the row.
            for (let i = 1; i <= 7; i++)
                mouseMove(pane.entryList, x, y + i * 3)
            mouseRelease(pane.entryList, x, y + 21)
            compare(page.recoverPick, 12, "the row is picked")
            compare(pane.entryList.contentY, from, "and the list did not slide")
        }

        /// A press on the stand-in is a press on the row it stands for — the entry put down — and on nothing under it.
        function test_a_press_on_the_stand_in_answers_as_its_row_and_only_that() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            page.pickPresses = []
            mouseClick(pane.entryList, pane.entryList.width / 2, pinMiddle())
            compare(page.pickPresses, [3], "one press, on the entry it stands for — none on the row under it")
            compare(page.recoverPick, -1, "the entry is put down")
            tryCompare(pane, "pinPlace", "none")
            verify(!pane.pickStandIn.visible)
            compare(under.presses, 0)
            // Gone from there, the same place is the row's again.
            mouseClick(pane.entryList, pane.entryList.width / 2, pinMiddle())
            compare(page.pickPresses, [3, 10], "the row it stood over answers now")
        }

        /// The bar stands over the stand-in as it does over the rows: a press on it is the bar's, and the entry stays.
        function test_a_press_on_the_bar_over_the_stand_in_is_the_bars() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            const bar = pane.entryList.ScrollBar.vertical
            tryVerify(() => bar.visible, undefined, "the bar stands")
            page.pickPresses = []
            const from = pane.entryList.contentY
            mouseClick(pane.entryList, pane.entryList.width - bar.width / 2, pinMiddle())
            // The page the press sends glides on after the release, into whatever comes next.
            tryVerify(() => pane.entryList.contentY < from && !bar.stepping, undefined, "the bar paged the list")
            compare(page.pickPresses, [], "no press on the entry")
            compare(page.recoverPick, 3, "it stays picked")
        }

        /// A wheel over the stand-in scrolls the list under it.
        function test_a_wheel_over_the_stand_in_scrolls_the_list() {
            showEntries(manyEntries(30))
            page.recoverPick = 3
            scrollTo(10)
            pinAt("top", "main-3 reset")
            const from = pane.entryList.contentY
            mouseWheel(pane.entryList, pane.entryList.width / 2, pinMiddle(), 0, -120)
            tryVerify(() => pane.entryList.contentY > from && !pane.entryList.moving, undefined, "the rows moved")
            compare(pane.pinPlace, "top")
        }
    }
}
