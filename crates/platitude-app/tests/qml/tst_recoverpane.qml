import QtQuick
import QtTest
import platitude.ui

// The discard log as it stands over the left menu (`RecoverPane`, 破棄記録仕様.md): nothing the hand does over it
// reaches the menu underneath, its `✕` is the band's whole right end, a ring says the reflogs are being read, an empty
// answer says so, and a rest on an entry opens its card.
Item {
    id: root
    width: 300
    height: 400

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
        function pickRecover(index) {
            page.recoverPick = page.recoverPick === index ? -1 : index
        }
    }

    RecoverPane {
        id: pane
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 260
        curPage: page
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
            page.recoverEntries = []
            // The list laid out empty before the case writes its own rows.
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
            page.recoverEntries = [entry("Reset main")]
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

            page.recoverEntries = [entry("Reset main")]
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
            page.recoverEntries = [entry("Rebased feature/a-branch-name-long-enough-to-be-cut-at-the-time")]
            page.recoverPick = 0
            verify(waitForRendering(pane))
            const titles = found(pane, item => item.pieces !== undefined && item.cutting !== undefined)
            compare(titles.length, 1, "one row")
            verify(titles[0].cutting, "its name cut — never opened out in the list")
            compare(found(pane, item => item.text === "2 min ago").length, 1, "the row's time stays")
            verify(pane.restOnShown(0))
            tryVerify(() => pane.cardOpen, undefined, "the card opened")
            compare(pane.cardAt, 0)
        }

        /// Enough entries to overflow the list.
        function manyEntries(n) {
            const out = []
            for (let i = 0; i < n; i++)
                out.push(entry("Reset main-" + i))
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
        /// The list scrolled `rows` rows down from its first row.
        function scrollTo(rows) {
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
            all.splice(at, 0, entry("Reset fresh"))
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
    }
}
