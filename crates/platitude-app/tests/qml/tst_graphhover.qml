import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The graph's rows under a real pointer, with what the page keeps for them: the pane's hand (`HandWatch`), the host
// and its two seats (`RowHoverHost`), the chip's list and the commit's card. The real delegates, in a stand-in for the
// list and the pane.
//
// What a hand that walked to a row opens, and what a hand the rows moved under does not (デザイン規約 §グラフ行の
// ダブルクリック「グラフが動いて手の下へ来た行は点くが、手が動くまで何も開かない」, §左メニューの所作「手が動いていない所へ来た
// 行は開かない」): the search sending the view to a match (`GraphFind.goToMatch`), a wheel notch, the arrows — every
// move of the rows hands Qt's hover to whatever arrives under the still pointer (`tst_hoverunderstillhand.qml`), and
// the row must not take that for a hand that walked to it. And what holds the list over a hand it is drawn over, what
// a menu does to a card, what a second surface showing the same commit is, and what a leave from elsewhere is not.
Item {
    id: root
    width: 600
    // Eight rows of list, two rows of the choice beside it under them, and a strip under those that is no row: where
    // the hand parks.
    height: Theme.graphRowHeight * 11

    /// What the rows asked of the page, in order: `<row>:in` / `<row>:out` (`GraphList.rowHoverRequested`).
    property var asks: []
    /// Where the page's standing menu was raised and on what, "" for none — what the page tells the host as one door
    /// (`RepoPage.menuDoor`). The key is written first, so the door comes up whole.
    property string menuArea: ""
    property string menuKey: ""

    /// A record of `kind`, every field spelled out as `encode::chips_of` hands it.
    function record(kind, name) {
        return { "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false, "here": true,
                 "held": false, "locked": false, "remote": "", "key": kind + ":" + name }
    }
    /// A commit row as the model hands one over, every role the delegate requires. Rows 3 and 5 carry a chip, and
    /// row 6 a chip whose names stack (two branches and a tag: the front card and a fan of two).
    function commit(n) {
        return {
            "oid_hex": "c" + n + "00000000000000000000000000000000000000", "author": "someone",
            "author_time": 1700000000 + n, "subject": "commit " + n, "body": "", "node_lane": 0, "node_color": 0,
            "avatar": 0, "avatar_url": "", "co_authors": [], "geometry": [],
            "labels": n === 3 || n === 5 ? [root.record("branch", "topic-" + n)]
                    : n === 6 ? [root.record("branch", "stack-a"), root.record("branch", "stack-b"),
                                 root.record("tag", "v6")] : [],
            "stash_ref": "", "matched": false
        }
    }
    /// Where a menu raised on a row of the list stands: clear of the list and of row 5's chip.
    readonly property point listMenuAt: Qt.point(root.width - 200, Theme.graphRowHeight)
    property var rows: {
        const out = []
        for (let n = 0; n < 40; n++)
            out.push(root.commit(n))
        return out
    }
    function oid(n) {
        return root.rows[n].oid_hex
    }
    /// The same rows in a model that inserts and removes as the graph's does (`notify.rs`), so the delegates move
    /// rather than being built again. The list roles are strings here: a `ListModel` would turn an array into a
    /// sub-model, which the delegate cannot read; "" reads as no chips, no lanes, no co-authors.
    property bool liveModel: false
    ListModel {
        id: liveRows
    }
    function liveCommit(n) {
        return Object.assign(root.commit(n), { "labels": "", "co_authors": "", "geometry": "" })
    }
    /// Commits a reread took out of the graph, by id: a row rewritten in place to another commit leaves its own in
    /// the graph unless it is named here (`RowHoverHost.inGraph`).
    property var gone: ({})

    // `GraphPane` cut down to what the host reads off it (`RowHoverHost.graphPane`) and what the list's popup asks of
    // it (`RefListPopup.rowClicks`).
    Item {
        id: pane
        anchors.fill: parent
        readonly property alias view: list
        readonly property alias hand: paneHand
        readonly property var graphModel: QtObject {
            property bool provisionalOn: false
            function oidAt(row) {
                if (root.liveModel)
                    return row >= 0 && row < liveRows.count ? liveRows.get(row).oid_hex : ""
                return row >= 0 && row < root.rows.length ? root.oid(row) : ""
            }
            /// Where `oidHex`'s commit stands in the rows the list shows, -1 for one gone (`root.gone`).
            function rowOf(oidHex) {
                if (root.gone[oidHex] === true)
                    return -1
                if (!root.liveModel)
                    return root.rows.findIndex(row => row.oid_hex === oidHex)
                for (let i = 0; i < liveRows.count; i++) {
                    if (liveRows.get(i).oid_hex === oidHex)
                        return i
                }
                return -1
            }
        }
        // HEAD's stand-in, down unless a case stands it up (`test_the_stand_ins_list_…`): its chip a real stack.
        readonly property var headPin: QtObject {
            readonly property var chipItem: pinStack
            property bool visible: false
            property bool wanted: false
            property int headRow: -1
            property string standsFor: ""
        }
        RefChipStack {
            id: pinStack
            z: 1
            x: Metrics.labelColW - width - Theme.spaceXs
            y: Theme.spaceXs
            visible: pane.headPin.visible
            records: [root.record("branch", "main"), root.record("tag", "v0")]
        }
        function hushRowClicks() {}
        function pinPressed(row, modifiers) {}
        function noteRowClick(oidHex, chip) { return true }
        function dropRowRename() {}
        function rowRenameArmed(chip) { return false }
        function heldRowClick(oidHex, modifiers) {}
        readonly property bool rowClickGuarded: false

        HandWatch {
            id: paneHand
        }

        // `GraphList` cut down to what a row reads off `ListView.view` (the rest is the real delegate's).
        ListView {
            id: list
            width: root.width
            height: Theme.graphRowHeight * 8
            clip: true
            reuseItems: true
            model: root.liveModel ? liveRows : root.rows
            delegate: GraphRowDelegate {}
            // The rows moved under the hand: a scroll, and the list stepping down the pane for the find bar
            // (`GraphPane`) — but not the view putting itself back over its rows after a reread.
            onContentYChanged: {
                if (list.keepingRows)
                    paneHand.rowsKept()
                else
                    paneHand.rowsMoved(list.rereading)
            }
            onYChanged: paneHand.rowsMoved()

            readonly property HandWatch hand: paneHand
            readonly property var dividers: QtObject {
                readonly property bool lineWanted: false
            }
            property Item shield: null
            property bool keepingRows: false
            property bool rereading: false
            function shieldAt(scenePoint) { return false }
            function haltGlide() {}
            function clampY(y) {
                const minY = list.originY - list.topMargin
                return Math.max(minY, Math.min(y, Math.max(minY, list.originY + list.contentHeight - list.height
                                                                  + list.bottomMargin)))
            }
            function rowOnScreen(row) { return false }
            property string listKey: host.listKey
            property string cardKey: host.cardKey
            property real labelWidth: Metrics.labelColW
            property real graphColWidth: 60
            property real graphFullWidth: 60
            property real graphXOffset: 0
            property int headRow: -1
            property string namingOid: ""
            property string namingText: ""
            property string namingOpenedWith: ""
            property string namingMode: "branch"
            property string namingKind: ""
            property string withheldOid: ""
            property string withheldWhy: ""
            property string withheldMark: ""
            property bool namingRefused: false
            property string namingRefusedWhy: ""
            property string namingRefusedMark: ""
            property string askOid: ""
            property bool askDanger: false
            property bool findOn: false
            property var chosenOids: ({})
            property int chosenCount: 0
            property bool renameWaiting: false
            readonly property bool clicksHushed: false
            readonly property bool clickGuarded: false
            function renameArmed(chip) { return false }
            function noteClick(oidHex, chip) { return true }
            function dropRename() {}
            function takeKeyboard() {}
            signal rowSelected(string oidHex, int atRow, int modifiers)
            signal rowMenuRequested(string oidHex, var chip)
            signal rowSwitchRequested(string oidHex, var chip)
            signal carriedOpenRequested(string path)
            signal carriedMenuRequested(string path, string name)
            signal rowRenameRequested(string oidHex, var chip)
            signal chipExpandRequested(string oidHex, int atRow, var records, var anchor)
            signal chipCollapseRequested(string oidHex, var anchor)
            signal rowHoverRequested(var row, bool inside)
            signal rowRewritten(var row)
            signal chipChanged(string oidHex, var anchor)
            onRowRewritten: row => host.rowGone(row)
            onChipChanged: (oidHex, anchor) => host.chipChanged(oidHex, anchor)
            signal namingSubmitted(string oidHex, string name, string mode)
            signal namingCancelled()
            // Wired as `RepoPage` wires the pane to the host.
            onRowHoverRequested: (row, inside) => {
                root.asks.push(row.index + ":" + (inside ? "in" : "out"))
                host.pointRow(row, inside, "graph")
            }
            onChipExpandRequested: (oidHex, atRow, records, anchor) => host.pointChip(oidHex, atRow, records, anchor)
            onChipCollapseRequested: (oidHex, anchor) => host.unpointChip(oidHex, anchor)
            // A right-click raises the row's menu on the graph (`RepoPage.openRowMenu`), over the hand: `offer` raises
            // it at the platform's cursor (`QQuickMenu::popup` reads `QCursor::pos`), which a test's pointer does not
            // move, so the hand's place is handed in as the place `offer` takes (`KeyMenu.at`).
            onRowMenuRequested: (oidHex, chip) => {
                root.menuKey = oidHex
                root.menuArea = "graph"
                KeyMenu.ask(paneHand.at, () => rowMenu.offer())
            }
        }
        // The graph's own walk over the list, for what a reread asks of the view (`GraphRowWalk.shiftRows`).
        GraphRowWalk {
            id: walk
            view: list
            graphModel: pane.graphModel
            asking: false
        }
    }
    // Another surface showing commits the graph shows too — the right pane's rows under a choice, the real
    // `ChosenCommitRow`s in a list of their own with that pane's hand (`DetailsPane`): another area, so another
    // target. Four rows, two in view, so the list scrolls.
    ListModel {
        id: chosenRows
    }
    /// A row as `GraphModel.chosenRows` names one; the list roles are strings, as `liveCommit`'s.
    function chosenRecord(n) {
        return { "oid": root.oid(n), "sha8": "c" + n + "000000", "subject": "commit " + n, "body": "",
                 "author": "someone", "authorTime": 1700000000 + n, "avatar": 0, "avatarUrl": "", "mates": "",
                 "laneAbove": "", "laneBelow": "" }
    }
    Item {
        id: chosenPane
        x: root.width - 200
        y: Theme.graphRowHeight * 8
        width: 200
        height: Theme.graphRowHeight * 2
        z: 1
        // The right pane's hand, as the host is handed it (`DetailsPane.hand`).
        HandWatch {
            id: chosenHand
        }
        AppListView {
            id: chosenList
            anchors.fill: parent
            clip: true
            model: chosenRows
            // The rows moved under the hand (`DetailsPane`).
            onContentYChanged: chosenHand.rowsMoved()
            delegate: ChosenCommitRow {
                cardKey: host.cardKey
                paneHand: chosenHand
                onHoverRequested: (row, inside) => host.pointRow(row, inside, "chosen")
                onRewritten: row => host.rowGone(row)
            }
        }
    }
    /// The page's row menu, cut down to one row: raised at the hand (`AppMenu.offer`), the door noted before it shows
    /// and cleared as it goes (`RepoPage.menuDoor`). `acted` counts its row being pressed.
    property int acted: 0
    AppMenu {
        id: rowMenu
        AppMenuItem {
            text: "Act"
            onTriggered: root.acted++
        }
        onClosed: root.menuArea = ""
    }
    RowHoverHost {
        id: host
        graphPane: pane
        currentBranch: "main"
        // The stacked rows' second lines: none, every branch standing alone.
        branchesModel: QtObject {
            function upstreamGoneOf(name) { return "" }
            function upstreamOf(name) { return "" }
        }
        remotesModel: QtObject {
            function trackedBy(name) { return "" }
        }
        tagsModel: null
        tagAgainst: "origin"
        menuDoor: root.menuArea === "" ? null : { "area": root.menuArea, "key": root.menuKey }
        chosenHand: chosenHand
        // A right-click on a row of the list raises the row's menu on the list (`RepoPage.openRowMenu`) — at a place of
        // its own here (`listMenuAt`; `offer` would raise it at the platform's cursor, which a test does not move).
        onRecordMenuAsked: (oidHex, chip) => {
            root.menuKey = oidHex
            root.menuArea = "refList"
            KeyMenu.ask(root.listMenuAt, () => rowMenu.offer())
        }
    }

    TestCase {
        name: "GraphHover"
        when: windowShown

        function init() {
            // The strip under the rows, away from the chosen row: on the pane, on no row.
            mouseMove(root, 1, root.height - 1)
            if (rowMenu.visible)
                rowMenu.dismiss()
            tryVerify(() => !rowMenu.visible, undefined, "no menu stands")
            root.menuArea = ""
            root.menuKey = ""
            root.acted = 0
            root.gone = ({})
            chosenRows.clear()
            for (const n of [2, 4, 6, 8])
                chosenRows.append(root.chosenRecord(n))
            chosenList.contentY = 0
            chosenList.forceLayout()
            tryVerify(() => chosenList.itemAtIndex(0) !== null && chosenList.itemAtIndex(0).oid_hex === root.oid(2),
                      undefined, "the choice's rows are up")
            list.reuseItems = false
            root.liveModel = false
            list.reuseItems = true
            list.y = 0
            list.contentY = 0
            list.forceLayout()
            // Not waited for a frame: a case that left the list where it was draws nothing new.
            tryVerify(() => list.contentY === 0 && list.itemAtIndex(0) !== null, undefined, "the list is at its top")
            // The list put back moved its rows under the hand (`HandWatch.stillHeld`): a step along the strip lifts it.
            mouseMove(root, 2, root.height - 1)
            mouseMove(root, 1, root.height - 1)
            verify(!paneHand.stillHeld, "the hand is its own again")
            tryVerify(() => !host.hoverCard.visible && !host.listPopup.visible, undefined,
                      "each case starts with nothing out: "
                      + JSON.stringify({ "card": host.hoverCard.visible, "cardOpened": host.hoverCard.opened,
                                         "list": host.listPopup.visible, "listOpened": host.listPopup.opened,
                                         "listKey": host.listKey, "cardKey": host.cardKey,
                                         "known": paneHand.known, "at": [paneHand.at.x, paneHand.at.y],
                                         "lit": litRows(), "asks": root.asks }))
            root.asks = []
        }

        /// The rows lit right now, by index.
        function litRows() {
            const lit = []
            for (let i = 0; i < list.count; i++) {
                const row = list.itemAtIndex(i)
                if (row && row.lit)
                    lit.push(i)
            }
            return lit
        }
        function litIs(rows) {
            return JSON.stringify(litRows()) === JSON.stringify(rows)
        }
        function rowY(index) {
            return index * Theme.graphRowHeight - list.contentY + Theme.graphRowHeight / 2
        }
        /// The hand walked into the middle of the list that is out.
        function handIntoList() {
            const card = host.listPopup.background
            mouseMove(card, card.width / 2, card.height / 2)
            tryVerify(() => host.listPopup.pointerInside, undefined, "the hand is in the list")
        }
        /// HEAD's stand-in up, standing for the first commit, and its chip's list out.
        function standInListOut() {
            pane.headPin.standsFor = root.oid(0)
            pane.headPin.wanted = true
            pane.headPin.visible = true
            host.pointChip(root.oid(0), 0, pinStack.records, pinStack)
            tryVerify(() => host.listPopup.opened && host.listOnStandIn, undefined, "the stand-in's list is out")
        }
        /// A hand come to rest on the words of row `index`, in the list's coordinates.
        function restOn(index) {
            mouseMove(list, list.labelWidth + list.graphColWidth + 40, rowY(index))
            tryVerify(() => litIs([index]), undefined, "the row the hand walked to is lit: " + litRows())
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(index), undefined,
                      "and its card is out after the rest")
            verify(host.hoverCard.opened, "the card itself stands")
        }
        /// Onto the left end of row `index`'s chip column — the stretch its chip's card leaves bare, the chip
        /// standing at the column's right end.
        function ontoBareStretch(index) {
            mouseMove(list, Theme.spaceSm, rowY(index))
            tryVerify(() => host.listKey === "chip:" + root.oid(index) + ":row", undefined, "the chip's list is out")
            tryVerify(() => host.listPopup.opened, undefined, "the list itself stands")
            verify(!host.listPopup.pointerInside, "beside the hand, not under it")
        }

        function test_a_hand_that_walks_to_a_row_lights_it_and_opens_its_card_after_the_rest() {
            mouseMove(list, list.labelWidth + list.graphColWidth + 40, rowY(2))
            tryVerify(() => litIs([2]), undefined, "lit the moment the hand is on it: " + litRows())
            verify(!host.hoverCard.visible, "the card waits for the rest")
            tryVerify(() => host.hoverCard.opened, undefined, "and comes out after it")
            compare(host.cardKey, "card:graph:" + root.oid(2))
            compare(root.asks.indexOf("2:in") >= 0, true, "the row asked once the hand arrived")
        }

        /// The row under a hand that has not moved since the rows did, in the list's own numbering.
        function underStillHand() {
            return list.indexAt(list.labelWidth + list.graphColWidth + 40,
                                2 * Theme.graphRowHeight + Theme.graphRowHeight / 2 + list.contentY)
        }
        /// The row the pane's hand is over now, in the list's own numbering.
        function rowUnderHand() {
            const p = list.mapFromItem(null, paneHand.at.x, paneHand.at.y)
            return list.indexAt(p.x, p.y + list.contentY)
        }

        /// The search sends the view to a match: the row that arrives under the still hand lights, as Qt hands it
        /// the hover, but opens nothing — its ask waits on the seat for the hand — and the row the hand was on, gone
        /// with the move, took its card with it.
        function test_a_row_the_search_brings_under_a_still_hand_opens_nothing() {
            restOn(2)
            list.positionViewAtIndex(25, ListView.Center)
            list.forceLayout()
            verify(waitForRendering(list), "the move is on screen")
            tryVerify(() => !host.hoverCard.visible, undefined, "the card went with the row")
            const under = underStillHand()
            tryVerify(() => litIs([under]), undefined, "the row brought under the hand is lit: " + litRows())
            // waits(paced): the subject is a card that must **not** come out, so the wait is the rest a row would
            // count before asking; nothing here answers sooner.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "", "but no card is out under a hand that did not move")
            verify(host.cardAskKept, "its ask is kept for the hand's move")
        }

        /// A notch of the wheel moves the rows the same way: the row brought under the hand lights and opens nothing
        /// until the hand moves; once it has, its rest opens its card.
        function test_a_row_the_wheel_brings_under_a_still_hand_opens_nothing_until_the_hand_moves() {
            restOn(2)
            list.contentY = Theme.graphRowHeight * 3
            list.forceLayout()
            verify(waitForRendering(list), "the move is on screen")
            tryVerify(() => !host.hoverCard.visible, undefined, "the card went with the row")
            tryVerify(() => litIs([5]), undefined, "the row brought under the hand is lit: " + litRows())
            // waits(paced): as above — a card that must **not** come out.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "", "nothing opens under the still hand")

            // A tremor where it rested: the hand's own move, and the rest judges — the row's card comes after it
            // (デザイン規約 §左メニューの所作「手が動いたかの見切り」).
            mouseMove(list, list.labelWidth + list.graphColWidth + 42,
                      2 * Theme.graphRowHeight + Theme.graphRowHeight / 2)
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(5), undefined,
                      "the hand moving opens its card after the rest")
            verify(litIs([5]), "on the row it is on: " + litRows())
        }

        /// The chip's list opens over the chip under a hand that has stopped there, so the pane hears the hand
        /// leave the moment the card is drawn. The row answers that leave as it answers any other — the list is let
        /// go — and the card keeps it, Qt handing the card the hand in the same delivery (`HoverSeat` holds for the
        /// hand inside).
        function test_a_list_drawn_over_a_still_hand_keeps_it() {
            const row = list.itemAtIndex(3)
            const chip = row.chipItem
            const at = chip.mapToItem(list, chip.width / 2, chip.height / 2)
            mouseMove(list, at.x, at.y)
            tryVerify(() => host.listPopup.opened, undefined, "the chip asked for its list")
            compare(host.listKey, "chip:" + root.oid(3) + ":row")
            tryVerify(() => host.listPopup.pointerInside && !paneHand.known, undefined,
                      "the card is drawn over the hand and takes it off the pane")
            // waits(paced): the subject is a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.listPopup.opened, "and the list stands under the hand, which it holds itself")
            verify(row.lit, "the row keeps its band under its own list")
        }

        function test_a_hand_gone_from_the_bare_stretch_takes_the_list_down() {
            ontoBareStretch(3)
            mouseMove(root, 1, root.height - 1)
            tryVerify(() => !host.listPopup.opened, undefined, "the leave lets it go and the beat takes it down")
            compare(host.listKey, "")
        }

        function test_a_hand_back_from_the_card_is_held() {
            ontoBareStretch(3)
            // Into the card, then back out onto the bare stretch: the row hears the hand arrive and asks again.
            handIntoList()
            mouseMove(list, Theme.spaceSm, rowY(3))
            tryVerify(() => !host.listPopup.pointerInside && host.listPointed, undefined,
                      "the hand is back on the chip's column and the list is asked for again")
            // waits(paced): the beat the card's leave started, which must **not** take the list down.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.listPopup.opened, "and the list stands for the hand on its row")
        }

        /// The same commit shown twice is two targets: the chosen row's card is not the graph row's, so the graph
        /// row wears no band for it, and a leave from the graph row does not take it down.
        function test_the_same_commit_in_another_area_is_another_target() {
            restOn(2)
            const chosen = chosenList.itemAtIndex(0)
            verify(chosen !== null, "the choice's first row is built")
            mouseMove(chosen, chosen.width / 2, chosen.height / 2)
            tryVerify(() => chosen.lit, undefined, "the choice's row is lit for the hand on it")
            tryVerify(() => host.cardKey === "card:chosen:" + root.oid(2), undefined,
                      "the chosen row's card takes the graph row's place after its own rest")
            verify(host.hoverCard.opened, "one card, handed over")
            verify(litIs([]), "and the graph row wears no band for the chosen row's card: " + litRows())
            // A late leave from the graph's row — the hand left it a while ago — is not a leave from this target.
            list.rowHoverRequested(list.itemAtIndex(2), false)
            // waits(paced): the beat such a leave would start if it were taken, which must **not** run.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.hoverCard.opened, "the card is still out")
            compare(host.cardKey, "card:chosen:" + root.oid(2))
        }

        /// A right-click on the row: the press takes the card down (a press outside is a dismissal) and raises the
        /// row's menu on the graph. The menu gone with the hand still where it pressed, the card stays down — the
        /// reader dismissed it — until the hand moves, which with the rest is a hover of its own.
        function test_a_right_click_takes_the_card_down_and_a_move_of_the_hand_brings_it_back() {
            const x = list.labelWidth + list.graphColWidth + 40
            restOn(2)
            mouseClick(list, x, rowY(2), Qt.RightButton)
            compare(root.menuArea, "graph", "the row asked for its menu")
            tryVerify(() => !host.hoverCard.opened, undefined, "the card went down at the press")
            compare(host.cardKey, "")
            // The menu let go: the hand is still where it pressed.
            const asksUnderMenu = JSON.stringify(root.asks)
            const underMenu = JSON.stringify({ "known": paneHand.known, "at": [paneHand.at.x, paneHand.at.y],
                                               "covered": paneHand.covered, "menu": rowMenu.visible,
                                               "seat": host.cardSeatState, "latched": host.cardLatched,
                                               "kept": host.cardAskKept })
            rowMenu.dismiss()
            tryVerify(() => root.menuArea === "", undefined, "the menu went")
            const atOnce = JSON.stringify({ "visible": host.hoverCard.visible, "seat": host.cardSeatState,
                                            "latched": host.cardLatched, "kept": host.cardAskKept })
            // waits(paced): a card that must **not** come back on its own; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!host.hoverCard.visible, "the dismissed card stays down under the still hand: "
                   + JSON.stringify({ "atOnce": atOnce, "underMenu": underMenu, "asksUnderMenu": asksUnderMenu,
                                      "asks": root.asks, "known": paneHand.known,
                                      "at": [paneHand.at.x, paneHand.at.y], "covered": paneHand.covered,
                                      "still": paneHand.stillHeld, "cardKey": host.cardKey,
                                      "pointed": host.cardPointed }))
            compare(host.cardKey, "")
            // A tremor along the same row: the hand's own move, and the rest judges — a hover.
            mouseMove(list, x + 2, rowY(2))
            tryVerify(() => host.hoverCard.opened, undefined, "the card is asked for again and comes out: "
                      + JSON.stringify({ "known": paneHand.known, "still": paneHand.stillHeld,
                                         "at": [paneHand.at.x, paneHand.at.y], "pointed": host.cardPointed,
                                         "x": list.itemAtIndex(2).pointerRowX, "part": list.itemAtIndex(2).pointedPart,
                                         "visible": host.hoverCard.visible, "asks": root.asks }))
            compare(host.cardKey, "card:graph:" + root.oid(2))
        }

        /// The graph's reread rewrites the row under the hand to another commit in place (`splice_notified`), its own
        /// gone from the graph (an amend made elsewhere): the card it had out goes with the commit, the row wears no
        /// band for the one it shows now, and nothing opens until the hand moves — any move, the rows having stayed
        /// where they were.
        function test_the_row_under_the_hand_rewritten_to_another_commit_lets_its_card_go() {
            const x = list.labelWidth + list.graphColWidth + 40
            restOn(2)
            const row = list.itemAtIndex(2)
            root.gone = { [root.oid(2)]: true }
            row.oid_hex = root.oid(39)
            tryVerify(() => !host.hoverCard.opened, undefined, "the card went with the commit")
            compare(host.cardKey, "")
            // waits(paced): a card that must **not** come out for the commit that arrived under the still hand.
            wait(Metrics.tipDelayMs * 2)
            verify(litIs([]), "the row wears no band for the commit it was given: " + litRows())
            compare(host.cardKey, "")
            mouseMove(list, x + 2, rowY(2))
            tryVerify(() => litIs([2]), undefined, "the hand moving reads the row again: " + litRows())
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(39), undefined,
                      "and its card is of the commit it shows now")
            // The row given back its own commit, for the cases after this one.
            row.oid_hex = root.oid(2)
        }

        /// The rows moving under the hand take what is out down at once, and bring nothing back on their own: the
        /// row under the hand now lights, as Qt's own lists hand it the hover on a scroll.
        function test_the_rows_moving_under_the_hand_take_the_card_down() {
            restOn(2)
            list.contentY = Theme.graphRowHeight
            tryVerify(() => !host.hoverCard.opened, undefined, "the card went with the move")
            // waits(paced): a card that must **not** come back under the still hand.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "")
            verify(litIs([3]), "the row under the hand now is lit: " + litRows())
        }

        /// The find bar opening steps the whole list down the pane with `contentY` where it was (`GraphPane`'s
        /// `anchors.topMargin`): the rows move under a still hand all the same. The row the hand was resting on is
        /// not under it any more and opens nothing; what came under it lights, and opens nothing until the hand moves.
        function test_the_list_stepping_down_under_a_still_hand_opens_nothing() {
            const x = list.labelWidth + list.graphColWidth + 40
            mouseMove(list, x, rowY(2))
            tryVerify(() => litIs([2]), undefined, "the hand walked to row 2: " + litRows())
            list.y = Theme.graphRowHeight
            // waits(paced): a card that must **not** come; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!host.hoverCard.visible, "no card for a row that moved from under the hand: " + host.cardKey)
            compare(host.cardKey, "")
            verify(litIs([1]), "the row that came under the hand is lit: " + litRows())
            verify(paneHand.stillHeld, "the hand is held until it walks")
            // A row along: a walk. Row 1 is under the hand now, the list having stepped down a row.
            mouseMove(list, x + Theme.graphRowHeight, rowY(1))
            tryVerify(() => litIs([1]), undefined, "the row under the walked hand is lit: " + litRows())
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(1), undefined, "and its card is asked for")
        }

        /// The list out on row `index`'s chip, and the reader putting it down: a press on the bare stretch, outside
        /// the list. It stays down for the hand on the chip, whatever the hand does there (`HoverSeat.expands`).
        function putDownOnBareStretch(index) {
            ontoBareStretch(index)
            mouseClick(list, Theme.spaceSm, rowY(index))
            tryVerify(() => !host.listPopup.visible, undefined, "the press outside the list put it down")
            mouseMove(list, Theme.spaceSm + 2, rowY(index))
            // waits(paced): a list that must **not** come back under the hand moving on the chip; the beat, twice.
            wait(Metrics.hoverKeepMs * 2)
            verify(!host.listPopup.visible, "not put back for a move on the chip")
            compare(host.listKey, "")
        }
        /// The hand back on row `index`'s chip from wherever it went: the list it put down is asked for afresh.
        function backOnBareStretch(index, from) {
            mouseMove(list, Theme.spaceSm, rowY(index))
            tryVerify(() => host.listKey === "chip:" + root.oid(index) + ":row", undefined,
                      "back from " + from + ", the list comes out again: " + JSON.stringify({
                          "listKey": host.listKey, "known": paneHand.known, "pointed": host.listPointed }))
            tryVerify(() => host.listPopup.opened, undefined, "and stands")
        }

        /// Walked off the chip along the pane — onto the row's words — and back.
        function test_a_list_put_down_comes_back_with_the_hand_back_from_the_rows_words() {
            putDownOnBareStretch(3)
            mouseMove(list, list.labelWidth + list.graphColWidth + 40, rowY(3))
            tryVerify(() => litIs([3]) && !host.listPointed, undefined, "the hand is on the row's words")
            backOnBareStretch(3, "the row's words")
        }

        /// Gone from the pane onto the one beside it, which has a hand of its own (the right pane's): the graph's
        /// rows never hear it go, and the leave they tell is the hand leaving the pane, with no menu standing.
        function test_a_list_put_down_comes_back_with_the_hand_back_from_the_pane_beside() {
            putDownOnBareStretch(3)
            const chosen = chosenList.itemAtIndex(0)
            mouseMove(chosen, chosen.width / 2, chosen.height / 2)
            tryVerify(() => !paneHand.known && chosenHand.known, undefined, "the hand is on the pane beside")
            backOnBareStretch(3, "the pane beside")
        }

        /// Gone out of the window, and back.
        function test_a_list_put_down_comes_back_with_the_hand_back_from_outside_the_window() {
            putDownOnBareStretch(3)
            mouseMove(root, root.width + 10, rowY(3))
            tryVerify(() => !paneHand.known, undefined, "the hand is out of the window")
            backOnBareStretch(3, "outside the window")
        }

        /// Onto another chip, whose list the hand opens — told, it may be, before the chip it left lets go — and back.
        function test_a_list_put_down_comes_back_with_the_hand_back_from_another_chips() {
            putDownOnBareStretch(5)
            mouseMove(list, Theme.spaceSm, rowY(3))
            tryVerify(() => host.listKey === "chip:" + root.oid(3) + ":row", undefined, "the other chip's list is out")
            backOnBareStretch(5, "another chip's list")
        }

        /// The row's menu raised on the chip takes the list down as a dismissal, and the menu over the hand takes the
        /// hand off the pane. The menu gone with the hand where it was, the list stays down — closing a menu brings
        /// nothing back — until the hand walks off the chip and comes back.
        function test_a_list_the_rows_menu_took_down_stays_down_as_the_menu_goes() {
            ontoBareStretch(3)
            mouseClick(list, Theme.spaceSm, rowY(3), Qt.RightButton)
            tryVerify(() => rowMenu.visible && root.menuArea === "graph", undefined, "the row's menu stands")
            tryVerify(() => !host.listPopup.visible, undefined, "and took the list down")
            tryVerify(() => !paneHand.known, undefined, "the menu over the hand takes it off the pane")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            tryVerify(() => paneHand.known, undefined, "the hand is heard again where it was")
            // waits(paced): a list that must **not** come back as the menu goes; the beat, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(!host.listPopup.visible, "the menu gone, the list stays down under the hand on the chip")
            mouseMove(list, Theme.spaceSm + 2, rowY(3))
            // waits(paced): the same, for a move on the chip.
            wait(Metrics.hoverKeepMs * 2)
            verify(!host.listPopup.visible, "nor does a move on the chip bring it back")
            mouseMove(list, list.labelWidth + list.graphColWidth + 40, rowY(3))
            tryVerify(() => !host.listPointed, undefined, "the hand walked off the chip")
            backOnBareStretch(3, "the row's words")
        }

        /// The row's menu raised from the keyboard stands where the keyboard is, not over the hand, and takes the list
        /// down as a dismissal. The hand leaving the pane meanwhile — along the chip's column, so the rows hear it
        /// nowhere off the chip — has left: Escape, and back on the chip, the list comes out again.
        function test_a_list_a_menu_elsewhere_took_down_comes_back_with_the_hand_back_from_outside_the_window() {
            ontoBareStretch(3)
            root.menuKey = root.oid(3)
            root.menuArea = "graph"
            KeyMenu.ask(root.listMenuAt, () => rowMenu.offer())
            tryVerify(() => rowMenu.visible, undefined, "the row's menu stands")
            tryVerify(() => !host.listPopup.visible, undefined, "and took the list down")
            verify(paneHand.known && !rowMenu.pointerInside, "away from the hand, which the pane still hears")
            mouseMove(root, -10, rowY(3))
            tryVerify(() => !paneHand.known, undefined, "the hand is out of the window")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            backOnBareStretch(3, "outside the window, the menu standing elsewhere")
        }

        /// The row's menu raised over the hand hides it from the pane. The hand walking out of the menu onto the pane
        /// beside has left: the menu says it lost the hand, and nothing over the graph has it. Escape, and back on the
        /// chip, the list comes out again.
        function test_a_list_the_rows_menu_took_down_comes_back_with_the_hand_out_of_the_menu_and_back() {
            ontoBareStretch(3)
            mouseClick(list, Theme.spaceSm, rowY(3), Qt.RightButton)
            tryVerify(() => rowMenu.visible && root.menuArea === "graph", undefined, "the row's menu stands")
            tryVerify(() => !host.listPopup.visible && !paneHand.known, undefined, "over the hand, the list down")
            verify(paneHand.hidden, "the hand is hidden under the menu")
            const chosen = chosenList.itemAtIndex(0)
            mouseMove(chosen, chosen.width / 2, chosen.height / 2)
            tryVerify(() => chosenHand.known && !rowMenu.pointerInside, undefined,
                      "out of the menu, on the pane beside")
            verify(!paneHand.hidden, "the graph's hand left")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            backOnBareStretch(3, "the pane beside, out of the menu")
        }

        /// The row's menu reaching past the graph onto the pane beside, put down with the hand in that part: Qt hands
        /// the pane beside the hand, and the graph never hears it — the hand had left the graph, and back on the chip
        /// the list comes out again. Put down with the hand over the chip, the list stays down
        /// (`test_a_list_the_rows_menu_took_down_stays_down_as_the_menu_goes`).
        function test_a_list_a_menu_took_down_comes_back_from_the_menus_part_past_the_pane() {
            ontoBareStretch(3)
            root.menuKey = root.oid(3)
            root.menuArea = "graph"
            KeyMenu.ask(Qt.point(chosenPane.x - 40, chosenPane.y - Theme.spaceSm), () => rowMenu.offer())
            tryVerify(() => rowMenu.visible && !host.listPopup.visible, undefined, "the menu took the list down")
            mouseMove(root, chosenPane.x + 20, chosenPane.y + Theme.spaceSm)
            tryVerify(() => rowMenu.pointerInside && !paneHand.known, undefined, "in the menu, over the pane beside")
            verify(paneHand.hidden, "hidden from the graph")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            tryVerify(() => chosenHand.known && !rowMenu.pointerInside, undefined, "the pane beside has the hand")
            verify(!paneHand.hidden, "the graph never heard it: the hand left")
            backOnBareStretch(3, "the menu's part over the pane beside")
        }

        /// The row's menu standing over the chip with the hand on its row, put down: the row's handler keeps the hand
        /// until Qt has handed it back to the graph (`AppMenuItem.handOn`) — heard on the chip it never left, the list
        /// stays down.
        function test_a_list_the_rows_menu_took_down_stays_down_as_it_goes_with_the_hand_on_its_row() {
            ontoBareStretch(3)
            root.menuKey = root.oid(3)
            root.menuArea = "graph"
            KeyMenu.ask(Qt.point(0, rowY(3) - Theme.graphRowHeight / 2), () => rowMenu.offer())
            tryVerify(() => rowMenu.visible && !host.listPopup.visible, undefined, "the menu took the list down")
            tryVerify(() => rowMenu.itemAt(0).handOn && !paneHand.known, undefined, "the hand on the menu's row")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            tryVerify(() => paneHand.known, undefined, "the hand is heard again where it was, on the chip")
            // waits(paced): a list that must **not** come back as the menu goes; the beat, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(!host.listPopup.visible, "the menu gone, the list stays down under the hand on the chip")
        }

        /// HEAD's stand-in's list (デザイン規約 §グラフの中で HEAD を見失わない「外乱では閉じない」): the view putting
        /// itself back over its rows after a reread (`GraphList.keepingRows`) moved nothing, and the list stays; the
        /// reader's own move of the rows takes it down, as a row's.
        function test_the_stand_ins_list_stays_through_a_reread_and_goes_with_the_readers_move() {
            standInListOut()
            list.keepingRows = true
            list.contentY = list.contentY + Theme.graphRowHeight
            list.keepingRows = false
            verify(host.listPopup.opened, "a reread putting the view back moved nothing: the list stays")
            list.contentY = list.contentY + Theme.graphRowHeight
            tryVerify(() => !host.listPopup.opened, undefined, "the reader's move takes it down")
            pane.headPin.visible = false
            pane.headPin.wanted = false
            pane.headPin.standsFor = ""
        }
        /// …but standing for another commit — one made on the branch elsewhere — its chip is another chip, and what the
        /// list said is gone: the list goes, the hand inside it or not (デザイン規約 §グラフの中で HEAD を見失わない).
        function test_the_stand_ins_list_goes_as_it_stands_for_another_commit() {
            standInListOut()
            handIntoList()
            // As the stand-in tells it (`GraphHeadPin.rewritten`): another commit, its chip's names with it.
            pane.headPin.standsFor = root.oid(1)
            pinStack.records = [root.record("branch", "main")]
            host.rowGone(pane.headPin)
            tryVerify(() => !host.listPopup.opened, undefined, "the list goes with the commit it was of")
            pinStack.records = [root.record("branch", "main"), root.record("tag", "v0")]
            pane.headPin.visible = false
            pane.headPin.wanted = false
            pane.headPin.standsFor = ""
        }
        /// …and gone from its edge with the hand in the list — HEAD's names still on their way after a reread — it
        /// leaves the list where it stands: back for the same commit, the list stands on it again and says its names;
        /// back for another, the list goes.
        function test_the_stand_ins_list_waits_for_it_with_the_hand_inside() {
            standInListOut()
            handIntoList()
            pane.headPin.visible = false
            host.rowGone(pane.headPin)
            verify(host.listPopup.opened, "the list stays where it stands")
            pinStack.records = [root.record("branch", "main")]
            pane.headPin.visible = true
            verify(host.listPopup.opened, "back for the same commit, it stays")
            compare(host.listKey, "chip:" + root.oid(0) + ":pin", "on the stand-in again")
            compare(host.listPopup.records.length, 1, "and says the names as they are now")
            pane.headPin.visible = false
            host.rowGone(pane.headPin)
            verify(host.listPopup.opened, "gone again, the list waits")
            pane.headPin.standsFor = root.oid(1)
            host.rowGone(pane.headPin)
            tryVerify(() => !host.listPopup.opened, undefined, "standing for another commit, the list goes")
            pinStack.records = [root.record("branch", "main"), root.record("tag", "v0")]
            pane.headPin.wanted = false
            pane.headPin.standsFor = ""
        }

        /// A reread writes a commit in above the rows — every row rewritten in place, one more at the end
        /// (`splice_notified`, which writes every row before it tells one) — and the view puts itself back over them
        /// in the same turn (`GraphRowWalk.shiftRows`): nothing moved under the hand, and the card it rested on stays,
        /// on the row that shows its commit now (デザイン規約 §hover のツールチップ「外乱では閉じない」).
        function test_a_reread_the_view_takes_back_keeps_the_card_on_its_commit() {
            liveRows.clear()
            for (let n = 0; n < 20; n++)
                liveRows.append(root.liveCommit(n))
            list.reuseItems = false
            root.liveModel = true
            list.forceLayout()
            tryVerify(() => list.itemAtIndex(2) !== null && list.itemAtIndex(2).oid_hex === root.oid(2),
                      undefined, "the live rows are up")
            restOn(2)
            // Written from the last row up, so a row is told only once its commit stands one row down — as the
            // splice writes them all before it tells any.
            liveRows.append(root.liveCommit(19))
            for (let i = 19; i > 0; i--)
                liveRows.set(i, root.liveCommit(i - 1))
            liveRows.set(0, root.liveCommit(30))
            list.keepingRows = true
            list.contentY = list.contentY + Theme.graphRowHeight
            list.keepingRows = false
            verify(host.hoverCard.opened, "the card stays")
            compare(host.cardKey, "card:graph:" + root.oid(2), "of the commit it was of")
            tryVerify(() => litIs([3]), undefined, "the row that shows it now is the lit one: " + litRows())
            verify(host.cardPointed, "and the hand is on it")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.hoverCard.opened, "and stays")
        }

        /// A reread the view could not take back moves the rows under the hand (`HandWatch.rowsMoved(true)`): the card
        /// the hand is inside stays where it stands; the reader's own move takes it down, the hand inside it or not.
        function test_a_reread_the_view_cannot_take_back_leaves_the_card_the_hand_is_in() {
            restOn(2)
            mouseMove(root, host.hoverCard.x + Theme.spaceMd, host.hoverCard.y + Theme.spaceMd)
            tryVerify(() => host.hoverCard.pointerInside, undefined, "the hand is in the card")
            paneHand.rowsMoved(true)
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.hoverCard.opened, "the card the hand is in stays")
            paneHand.rowsMoved()
            tryVerify(() => !host.hoverCard.opened, undefined, "the reader's move takes it down")
        }
        /// …the same through the view's own weighing (`GraphRowWalk.shiftRows`, then `RowHoverHost.reseat` as the page
        /// asks): a history shorter than the view, every row rewritten in place and one more built at the end, laid out
        /// as the view weighs the shift — a row the reread put down, no reader's move. The card the hand is in stays.
        function test_a_reread_on_a_short_history_leaves_the_card_the_hand_is_in() {
            liveRows.clear()
            for (let n = 0; n < 5; n++)
                liveRows.append(root.liveCommit(n))
            list.reuseItems = false
            root.liveModel = true
            list.forceLayout()
            tryVerify(() => list.itemAtIndex(2) !== null && list.itemAtIndex(2).oid_hex === root.oid(2),
                      undefined, "the live rows are up")
            restOn(2)
            mouseMove(root, host.hoverCard.x + Theme.spaceMd, host.hoverCard.y + Theme.spaceMd)
            tryVerify(() => host.hoverCard.pointerInside, undefined, "the hand is in the card")
            liveRows.append(root.liveCommit(4))
            for (let i = 4; i > 0; i--)
                liveRows.set(i, root.liveCommit(i - 1))
            liveRows.set(0, root.liveCommit(30))
            walk.shiftRows(1)
            host.reseat()
            verify(paneHand.stillHeld, "the rows moved under the hand, as the reread's")
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.hoverCard.opened, "the card the hand is in stays")
            compare(host.cardKey, "card:graph:" + root.oid(2), "of the commit it was of")
        }

        /// The row's menu took the list down, and the hand left the chip under the menu: a press outside the menu,
        /// on a row further down, takes the menu down, and the hand is first heard there — off the chip. Back on the
        /// chip, the list comes out again.
        function test_a_list_the_rows_menu_took_down_comes_back_with_the_hand_heard_elsewhere_and_back() {
            const words = list.labelWidth + list.graphColWidth + 40
            ontoBareStretch(3)
            mouseClick(list, Theme.spaceSm, rowY(3), Qt.RightButton)
            tryVerify(() => rowMenu.visible && !paneHand.known, undefined, "the menu stands over the hand")
            mouseClick(list, words, rowY(7))
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "the press outside took the menu down")
            mouseMove(list, words + 1, rowY(7))
            tryVerify(() => litIs([7]), undefined, "the hand is heard on row 7: " + litRows())
            backOnBareStretch(3, "row 7, where it was first heard")
        }

        /// A chip whose names stack opens its list on the stack as the reader saw it, every sheet drawn — not where
        /// the front card stands once the list has taken the sheets down: the stack keeps its right edge and drops by
        /// half its fan (`RefChipStack.lift`).
        function test_a_stacked_chips_list_is_seated_on_the_stack_as_it_was_drawn() {
            const stack = list.itemAtIndex(6).chipItem
            verify(stack.fanW > 0 && stack.lift > 0, "the chip stacks its names: " + stack.fanW + ", " + stack.lift)
            const drawn = stack.mapToItem(host, 0, 0)
            ontoBareStretch(6)
            const card = host.listPopup
            compare(card.x, drawn.x - Theme.spaceXs - card.padding, "seated on the stack's left edge as drawn")
            compare(card.y, drawn.y - card.padding - card.chipInset, "and on its top as drawn")
        }

        /// A row taken out of the list leaves its delegate in the pool, `visible` where the row stood: it answers no
        /// hand there, and asks for nothing of the commit that went.
        function test_a_row_taken_out_of_the_list_answers_no_hand() {
            liveRows.clear()
            for (let n = 0; n < 6; n++)
                liveRows.append(root.liveCommit(n))
            list.reuseItems = false
            root.liveModel = true
            list.forceLayout()
            // Reused from here: the row taken out goes to the pool.
            list.reuseItems = true
            tryVerify(() => list.itemAtIndex(5) !== null && list.itemAtIndex(5).oid_hex === root.oid(5), undefined,
                      "the live rows are up")
            const x = list.labelWidth + list.graphColWidth + 40
            mouseMove(list, x, rowY(5))
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(5), undefined, "the last row's card is out")
            const gone = list.itemAtIndex(5)
            liveRows.remove(5)
            tryVerify(() => !host.hoverCard.visible, undefined, "the card went with its row")
            mouseMove(list, x + 2, rowY(5))
            // waits(paced): a card that must **not** come for the row that went; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "", "the delegate left in the pool asks for nothing: " + JSON.stringify({
                "pooled": gone.pooled, "visible": gone.visible, "y": gone.y, "oid": gone.oid_hex }))
            verify(!gone.lit, "nor lights")
        }

        /// A menu raised on a row of the list is about that list: the hand on another chip meanwhile does not hand
        /// the list over (the menu would be left standing on nothing). The menu gone, the hand decides again.
        function test_a_menu_on_the_list_keeps_it_from_the_hand_on_another_chip() {
            ontoBareStretch(3)
            verify(host.listPopup.menuRow(0), "the list's first row is built, and its menu is asked for")
            compare(root.menuArea, "refList", "the menu was raised on the list")
            compare(root.menuKey, root.oid(3), "on the list's commit")
            mouseMove(list, Theme.spaceSm, rowY(5))
            tryVerify(() => host.listAskKept, undefined,
                      "row 5's chip asked, and the ask is kept behind the menu's hold")
            // waits(paced): a list that must **not** be handed over; the beat it would take, twice over.
            wait(Metrics.hoverKeepMs * 2)
            compare(host.listKey, "chip:" + root.oid(3) + ":row", "the list stays the menu's")
            verify(host.listPopup.opened, "and stands")
            // Escape lets the menu go: the hand is on row 5's chip column.
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "the menu went")
            tryVerify(() => host.listKey === "chip:" + root.oid(5) + ":row", undefined,
                      "the menu gone, the chip under the hand opens its list: " + host.listKey)
        }

        /// Escape on the row's menu: the menu goes, and the card the press dismissed stays down under the still hand
        /// until it walks.
        function test_escape_on_the_rows_menu_leaves_the_dismissed_card_down() {
            const x = list.labelWidth + list.graphColWidth + 40
            restOn(2)
            mouseClick(list, x, rowY(2), Qt.RightButton)
            tryVerify(() => rowMenu.visible, undefined, "the row's menu stands")
            keyClick(Qt.Key_Escape)
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "Escape took the menu down")
            // waits(paced): a card that must **not** come back on its own; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!host.hoverCard.visible, "the dismissed card stays down under the still hand")
            mouseMove(list, x + Theme.graphRowHeight, rowY(2))
            tryVerify(() => host.hoverCard.opened, undefined, "a walk brings it back")
        }

        /// The menu's own row pressed: the menu goes having acted, and the card stays down the same.
        function test_the_menus_row_pressed_closes_it_and_the_card_stays_down() {
            const x = list.labelWidth + list.graphColWidth + 40
            restOn(2)
            mouseClick(list, x, rowY(2), Qt.RightButton)
            tryVerify(() => rowMenu.visible, undefined, "the row's menu stands")
            const item = rowMenu.itemAt(0)
            mouseClick(item, item.width / 2, item.height / 2)
            tryVerify(() => root.acted === 1 && !rowMenu.visible, undefined, "the row acted and the menu went")
            // waits(paced): a card that must **not** come back on its own; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!host.hoverCard.visible, "the dismissed card stays down under the hand where the menu was")
            compare(host.cardKey, "")
        }

        /// A press outside the menu, on another row: the menu goes, and the hand is on that row by a walk — it lights
        /// and its card is asked for.
        function test_a_press_outside_the_menu_is_a_walk_to_where_it_pressed() {
            const x = list.labelWidth + list.graphColWidth + 40
            restOn(2)
            mouseClick(list, x, rowY(2), Qt.RightButton)
            tryVerify(() => rowMenu.visible, undefined, "the row's menu stands")
            mouseClick(list, x, rowY(5))
            tryVerify(() => !rowMenu.visible && root.menuArea === "", undefined, "the press outside took the menu down")
            // The hand is where it pressed; its next move is heard there (the menu heard the press, not the rows).
            mouseMove(list, x + 1, rowY(5))
            tryVerify(() => litIs([5]), undefined, "the row pressed is lit: " + litRows())
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(5), undefined, "and its card is asked for")
        }

        /// Rows moving below the list's box — the ones it keeps built past its edge, the current row among them, which
        /// a reread moves as the selection follows its commit — move nothing under the hand: the card it is in stays.
        function test_rows_moving_below_the_view_take_nothing_down() {
            liveRows.clear()
            for (let n = 6; n < 18; n++)
                liveRows.append(root.liveCommit(n))
            list.reuseItems = false
            root.liveModel = true
            list.forceLayout()
            tryVerify(() => list.itemAtIndex(2) !== null && list.itemAtIndex(2).oid_hex === root.oid(8),
                      undefined, "the live rows are up")
            // The selection moves without the view, as `GraphPane.setCurrentRow` moves it.
            list.currentIndex = 11
            list.contentY = 0
            list.forceLayout()
            tryVerify(() => list.contentY === 0 && list.currentItem !== null && list.currentItem.y >= list.height,
                      undefined, "the current row is built below the view")
            const x = list.labelWidth + list.graphColWidth + 40
            mouseMove(list, x, rowY(2))
            tryVerify(() => host.cardKey === "card:graph:" + root.oid(8), undefined, "row 2's card is out")
            mouseMove(root, host.hoverCard.x + Theme.spaceMd, host.hoverCard.y + Theme.spaceMd)
            tryVerify(() => host.hoverCard.pointerInside, undefined, "the hand is in the card")
            const was = list.currentItem.y
            liveRows.insert(10, root.liveCommit(31))
            tryVerify(() => list.currentItem.y !== was, undefined, "the current row moved, below the view")
            verify(host.hoverCard.visible, "the card the hand is in stays")
            verify(!paneHand.stillHeld, "and nothing holds the hand")
            list.currentIndex = -1
        }

        /// The choice's rows scrolling under a still hand: the card goes, and the row that came under the hand lights
        /// but opens nothing until the hand walks.
        function test_the_choices_rows_moving_under_a_still_hand_open_nothing() {
            const chosen = chosenList.itemAtIndex(0)
            mouseMove(chosen, chosen.width / 2, chosen.height / 2)
            tryVerify(() => host.cardKey === "card:chosen:" + root.oid(2), undefined, "the row's card is out")
            chosenList.contentY = Theme.graphRowHeight
            tryVerify(() => !host.hoverCard.opened, undefined, "the card went with the move")
            verify(chosenHand.stillHeld, "the right pane's hand is held")
            // waits(paced): a card that must **not** come back under the still hand.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "")
            const under = chosenList.itemAtIndex(1)
            verify(under !== null && under.lit, "the row that came under the hand is lit")
            // A row along the list's first row — the one that came under the hand: a walk.
            mouseMove(chosenList, chosenList.width / 2 + Theme.graphRowHeight, Theme.graphRowHeight / 2)
            tryVerify(() => host.cardKey === "card:chosen:" + root.oid(4), undefined,
                      "a walk: the row under the hand now is asked for: " + host.cardKey)
        }

        /// The row rewritten to another commit with the hand in its card, its own gone from the graph: the card is of
        /// a commit that is no more, and goes as the owner's close — the hand inside it or not.
        function test_the_row_rewritten_with_the_hand_in_its_card_takes_the_card_down() {
            restOn(2)
            mouseMove(root, host.hoverCard.x + Theme.spaceMd, host.hoverCard.y + Theme.spaceMd)
            tryVerify(() => host.hoverCard.pointerInside, undefined, "the hand is in the card")
            const row = list.itemAtIndex(2)
            root.gone = { [root.oid(2)]: true }
            row.oid_hex = root.oid(36)
            tryVerify(() => !host.hoverCard.visible, undefined, "the card went with the commit the row showed")
            compare(host.cardKey, "")
            row.oid_hex = root.oid(2)
        }
        /// …but its own still in the graph — a reread wrote others in above it, and the view could not put itself back
        /// over it: the card the hand is in stays where it stands (デザイン規約 §hover のツールチップ「外乱では閉じない」).
        function test_the_row_rewritten_with_its_commit_still_in_the_graph_keeps_the_card_the_hand_is_in() {
            restOn(2)
            mouseMove(root, host.hoverCard.x + Theme.spaceMd, host.hoverCard.y + Theme.spaceMd)
            tryVerify(() => host.hoverCard.pointerInside, undefined, "the hand is in the card")
            const row = list.itemAtIndex(2)
            row.oid_hex = root.oid(36)
            // waits(paced): a card that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.hoverCard.opened, "the card stays with the hand in it")
            compare(host.cardKey, "card:graph:" + root.oid(2))
            row.oid_hex = root.oid(2)
        }

        /// The row rewritten to another commit with the hand in the list its chip unfolded, its own gone from the
        /// graph: the list is of a commit that is no more and goes — the hand inside it or not, and the row's chips the
        /// same as before, so nothing but the row's going says so (`RowHoverHost.rowGone`, by the chip the list was
        /// asked from).
        function test_the_row_rewritten_with_the_hand_in_its_list_takes_the_list_down() {
            ontoBareStretch(3)
            handIntoList()
            const row = list.itemAtIndex(3)
            root.gone = { [root.oid(3)]: true }
            row.oid_hex = root.oid(37)
            tryVerify(() => !host.listPopup.opened, undefined, "the list went with the commit the row showed")
            compare(host.listKey, "")
            row.oid_hex = root.oid(3)
        }
        /// …but its own still in the graph: the list the hand is in stays.
        function test_the_row_rewritten_with_its_commit_still_in_the_graph_keeps_the_list_the_hand_is_in() {
            ontoBareStretch(3)
            handIntoList()
            const row = list.itemAtIndex(3)
            row.oid_hex = root.oid(37)
            // waits(paced): a list that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.listPopup.opened, "the list stays with the hand in it")
            compare(host.listKey, "chip:" + root.oid(3) + ":row")
            row.oid_hex = root.oid(3)
        }
        /// A reread tells every row its names again — a fresh array, changed or not (`splice_notified`, the gone set):
        /// names told unchanged leave the list under the hand reading it as it is.
        function test_the_rows_names_told_again_unchanged_leave_the_list_with_the_hand_in_it() {
            ontoBareStretch(3)
            handIntoList()
            const row = list.itemAtIndex(3)
            const said = row.chipSays
            row.labels = [root.record("branch", "topic-3")]
            compare(row.chipSays, said, "the same names, told again")
            // Told again, the names reach the host in the same call (`chipChanged` → `RowHoverHost.chipChanged`).
            verify(host.listPopup.opened, "the list stays with the hand in it")
            compare(host.listKey, "chip:" + root.oid(3) + ":row")
            row.labels = root.commit(3).labels
        }
        /// The list's seat and size, to set beside the same names opened afresh.
        function seatOfList() {
            return JSON.stringify({ "x": host.listPopup.x, "y": host.listPopup.y, "width": host.listPopup.width })
        }
        /// Away from the chip, and back onto it: the list it opens afresh, as `seatOfList` puts it.
        function seatOpenedAfresh(index) {
            mouseMove(root, 1, root.height - 1)
            tryVerify(() => !host.listPopup.visible, undefined, "the list went with the hand")
            ontoBareStretch(index)
            return seatOfList()
        }
        /// …and names that did change are said in place: the list stays, with the chip's names as they are now, seated
        /// and measured as those names open afresh (`RowHoverHost.chipChanged` / `seatRefList`; デザイン規約 §hover の
        /// ツールチップ「外乱では閉じない」).
        function test_the_rows_names_changed_are_said_in_place_with_the_hand_in_the_list() {
            ontoBareStretch(3)
            handIntoList()
            const row = list.itemAtIndex(3)
            row.labels = [root.record("branch", "topic-3"), root.record("tag", "v3")]
            compare(host.listPopup.records.length, 2, "the list says the names as they are now")
            // waits(paced): a list that must **not** go: the beat a leave starts, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(host.listPopup.opened, "and stays")
            const said = seatOfList()
            compare(seatOpenedAfresh(3), said, "seated and measured as the names open afresh")
            row.labels = root.commit(3).labels
        }
        /// …and a chip left with no names has nothing to say: the list goes, the hand inside it or not (デザイン規約
        /// §hover のツールチップ「外乱では閉じない」).
        function test_a_chip_left_with_no_names_takes_its_list_with_it() {
            ontoBareStretch(3)
            handIntoList()
            const row = list.itemAtIndex(3)
            row.labels = []
            verify(!host.listPopup.opened, "the list goes with the names, in the turn they went: "
                   + host.listPopup.records.length)
            row.labels = root.commit(3).labels
        }
        /// …and fewer names leave the list as narrow as those names open afresh, on the chip as it stands: the chip
        /// keeps its right edge, so its left edge moves right with them.
        function test_fewer_names_said_in_place_narrow_the_list_as_they_would_open() {
            ontoBareStretch(6)
            handIntoList()
            const row = list.itemAtIndex(6)
            const wide = host.listPopup.width
            row.labels = [root.record("tag", "v6")]
            compare(host.listPopup.records.length, 1, "the list says the one name")
            verify(host.listPopup.width < wide,
                   "narrower than the three names: " + host.listPopup.width + " of " + wide)
            const said = seatOfList()
            compare(seatOpenedAfresh(6), said, "seated and measured as the name opens afresh")
            row.labels = root.commit(6).labels
        }

        /// A choice's row inserted above the one the hand is on moves it: told as a move, as the graph's rows are.
        function test_a_choices_row_inserted_above_the_hand_moves_the_rows_under_it() {
            const chosen = chosenList.itemAtIndex(0)
            mouseMove(chosen, chosen.width / 2, chosen.height / 2)
            tryVerify(() => host.cardKey === "card:chosen:" + root.oid(2), undefined, "the row's card is out")
            chosenRows.insert(0, root.chosenRecord(9))
            tryVerify(() => !host.hoverCard.opened, undefined, "the card went with the move")
            verify(chosenHand.stillHeld, "the right pane's hand is held")
            // waits(paced): a card that must **not** come back under the still hand.
            wait(Metrics.tipDelayMs * 2)
            compare(host.cardKey, "")
        }
    }
}
