import QtQuick
// For the attached ToolTip alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The list the seat opens over the left menu's sections (破棄記録仕様.md). The git commands' seat stays under it. One
// entry is picked at a time, and what it would bring back is told over the graph (`RecoverBand`); a rest on an entry
// opens its card, as a graph row's does (`RecoverHoverCard`).
Rectangle {
    id: pane

    /// The RepoPage this list belongs to.
    required property var curPage

    readonly property var entries: pane.curPage !== null ? pane.curPage.recoverEntries : []
    readonly property int picked: pane.curPage !== null ? pane.curPage.recoverPick : -1
    /// The reflogs are being read: on an empty list, before the first answer, and over the entries already up when
    /// the list reads again.
    readonly property bool reading: pane.curPage !== null && pane.curPage.recoverReading
    /// The log's words, its times included (`RecoverEntries`), and the moment the times are read against — the
    /// page's, which the band over the graph reads too.
    readonly property var words: pane.curPage !== null ? pane.curPage.recoverWords : null
    readonly property real now: pane.curPage !== null ? pane.curPage.recoverNow : 0
    /// What is typed into the filter. Automation writes it into the field itself, as a typist would.
    property alias filterText: filterField.text
    /// The entries the filter leaves, each with where it stands in the whole list — the number a pick goes by. Dealt
    /// again only when what it holds changed (`refilter`): a fresh array resets the list's rows and its scroll, and the
    /// times the filter reads move every half minute.
    property var shownEntries: []
    function refilter() {
        const line = filterField.text
        // Nothing typed leaves every entry in without asking the rule entry by entry.
        const asked = line.trim() !== ""
        const kept = []
        for (let i = 0; i < pane.entries.length; i++) {
            if (!asked || GitFacts.shownMatches(pane.wordsOf(pane.entries[i]), line))
                kept.push({ "entry": pane.entries[i], "at": i })
        }
        const same = kept.length === pane.shownEntries.length
            && kept.every((shown, i) => shown.entry === pane.shownEntries[i].entry && shown.at === pane.shownEntries[i].at)
        if (same)
            return
        pane.shownEntries = kept
        // The rows are dealt again under the hand, so a card names whatever row it stood on: it goes.
        pane.cardWanted = false
        entryCard.close()
    }
    onEntriesChanged: pane.refilter()
    onNowChanged: pane.refilter()
    Component.onCompleted: pane.refilter()
    /// Automation: the card is out, and for which entry (-1 for none).
    readonly property bool cardOpen: entryCard.opened
    property int cardAt: -1
    /// The row the card hangs from: rows are recycled (`AppListView.reuseItems`), so a row's number can change under a
    /// hand that never left it.
    property Item cardRow: null

    /// The words an entry shows, which the filter reads: what it did, where, what it took, and when — as the row
    /// writes its time and as its card does.
    function wordsOf(entry) {
        const words = [entry.title, entry.copy]
        for (let i = 0; i < entry.parts.length; i++)
            words.push(entry.parts[i].text)
        words.push(pane.words.ago(entry.at, pane.now), pane.words.whenWords(entry.at, pane.now))
        return words
    }

    /// A rest ran out on `row`: its card opens under the pointer, as a graph row's does (`RowHoverHost.openRowCard`).
    function openCard(row, pointerX) {
        if (entryCard.opened && pane.cardAt === row.at) {
            pane.cardWanted = true
            return
        }
        entryCard.entry = row.entry
        entryCard.when = pane.words.whenWords(row.entry.at, pane.now)
        const at = row.mapToItem(pane, pointerX, row.height)
        entryCard.x = at.x
        entryCard.y = at.y
        pane.cardAt = row.at
        pane.cardRow = row
        pane.cardWanted = true
        entryCard.open()
    }
    /// The hand is on a row, or left it: what holds the card up (`HoverCardHost`).
    property bool cardWanted: false
    /// Automation: rests a hand on the shown entry `index` — hover cannot be injected (verify-ui スキル).
    function restOnShown(index) {
        const row = list.itemAtIndex(index)
        if (row !== null)
            pane.openCard(row, row.width / 2)
        return row !== null
    }

    color: Theme.bgSurface

    // Laid over the left menu, which stays up underneath (`SidebarPane`): no press, hover or wheel the list leaves
    // falls through to the filter, its fold block or the sections (rules-refs/app-ui.md の `RecoverPane` の行).
    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.AllButtons
        onWheel: wheel => wheel.accepted = true
    }

    // ---- the band: the seat itself at the head, as the log's band carries `>_` (CommandsPane) ----
    // The filter band's height, which it stands over. **The whole band is the way out**: a press anywhere on it closes
    // the list, and the hand anywhere on it lights the `✕` it ends in.
    Rectangle {
        id: band
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.headerHeight
        color: Theme.bgElevated
        RowLayout {
            anchors.fill: parent
            spacing: Theme.spaceXs
            RecoverToggle {
                Layout.fillHeight: true
                captioned: true
                ruled: false
                curPage: pane.curPage
            }
            // Read again over the entries already up: the ring beside the heading (`WipBucketHeader`).
            SpinnerIcon {
                Layout.preferredWidth: Theme.iconSm
                Layout.preferredHeight: Theme.iconSm
                spinning: pane.reading && pane.entries.length > 0
            }
            Item { Layout.fillWidth: true }
            // The fold block's seat at the band's end (`SidebarFilterRow`).
            CloseToolButton {
                id: closeMark
                Layout.fillHeight: true
                seat: Theme.headerHeight
                pointedAt: bandHand.containsMouse
                Accessible.name: qsTr("Close the discards")
                onClicked: pane.curPage.toggleRecover()
            }
        }
        // Over the seat's caption, under the `✕`, which keeps its own press.
        MouseArea {
            id: bandHand
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.rightMargin: closeMark.width
            hoverEnabled: true
            onClicked: pane.curPage.toggleRecover()
        }
    }
    Rectangle {
        id: bandRule
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: band.bottom
        height: Theme.borderWidth
        color: Theme.borderSubtle
    }

    // ---- the filter, a row of its own under the band: the left menu's own field (`SidebarFilterRow`) ----
    Item {
        id: filterRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bandRule.bottom
        height: Theme.headerHeight
        TextField {
            id: filterField
            anchors.fill: parent
            onTextChanged: pane.refilter()
            font.pixelSize: Theme.fontMd
            // At the band's caption's x, as the left menu's field sits at its section headers'.
            leftPadding: Theme.spaceXs
            rightPadding: Theme.spaceXs
            topPadding: 0
            bottomPadding: 0
            placeholderText: qsTr("Filter")
            background: null
            // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
            ContextMenu.menu: null
            ContextMenu.onRequested: filterMenu.offer()
            Keys.onMenuPressed: event => event.accepted = filterMenu.offer()
            FieldMenuSeat {
                id: filterMenu
                editor: filterField
            }
        }
        BandRule {
            color: filterField.activeFocus || filterMenu.holding ? Theme.borderFocus : Theme.borderSubtle
        }
    }

    AppListView {
        id: list
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: filterRow.bottom
        anchors.bottom: parent.bottom
        model: pane.shownEntries
        delegate: EntryRow {
            pickedIndex: pane.picked
            page: pane.curPage
            host: pane
            now: pane.now
        }
    }
    // The first read, before any entry: a ring in a row's height, then the rows (`AppCombo`).
    SpinnerIcon {
        x: (pane.width - width) / 2
        y: filterRow.y + filterRow.height + (Theme.rowHeight - height) / 2
        width: Theme.iconMd
        height: Theme.iconMd
        spinning: pane.reading && pane.entries.length === 0
    }
    // An answer with nothing in it, or a filter that leaves nothing: not a row, so centred where the ring stood, as the
    // command log's empty list says its line (`CommandsPane`).
    Label {
        visible: pane.curPage !== null && pane.curPage.recoverAnswered && pane.shownEntries.length === 0
        x: (pane.width - width) / 2
        y: filterRow.y + filterRow.height + (Theme.rowHeight - height) / 2
        // The words list and search boxes everywhere answer an empty search with (VS Code, Windows, Chrome).
        text: pane.entries.length === 0 ? qsTr("Nothing was thrown away here") : qsTr("No results found")
        font.pixelSize: Theme.fontMd
        color: Theme.textMuted
    }
    // A read that failed, in git's words: the list holds nothing — no rows of an earlier read left to pick.
    Column {
        visible: pane.curPage !== null && pane.curPage.recoverFailed
        x: Theme.spaceXs
        y: filterRow.y + filterRow.height + Theme.spaceXs
        width: pane.width - 2 * Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            width: parent.width
            text: qsTr("What was thrown away could not be read")
            font.pixelSize: Theme.fontMd
            color: Theme.danger
            wrapMode: Text.Wrap
        }
        Label {
            width: parent.width
            text: pane.curPage !== null ? pane.curPage.recoverError : ""
            font.pixelSize: Theme.fontSm
            color: Theme.textSecondary
            wrapMode: Text.Wrap
        }
    }

    RecoverHoverCard {
        id: entryCard
        textWidth: pane.width
        onClosed: pane.cardAt = -1
    }
    HoverCardHost {
        card: entryCard
        pointedAt: pane.cardWanted
    }

    // One operation, in a left menu row's columns: its mark in the seat, what it did where the names begin, when at the
    // right edge. An inline component has its own ids, so the list's state comes in as properties.
    component EntryRow: Rectangle {
        id: row
        /// `{entry, at}` — the entry as the list words it, and where it stands in the whole list.
        required property var modelData
        property int pickedIndex: -1
        property var page: null
        /// The list, whose words for a time and whose card the row uses.
        property var host: null
        property real now: 0

        readonly property var entry: row.modelData.entry
        readonly property int at: row.modelData.at
        readonly property bool isPicked: row.pickedIndex === row.at

        width: ListView.view ? ListView.view.width : 0
        height: Theme.rowHeight
        color: row.isPicked ? Theme.bgSelected : rowHover.hovered ? Theme.bgHover : "transparent"

        // The card opens on a rest, as a graph row's does (規約 §hover のツールチップ「補足は待ってから開く」), and goes
        // a beat after the hand has left the row and the card both (`HoverCardHost`).
        HoverHandler {
            id: rowHover
            onHoveredChanged: {
                if (rowHover.hovered) {
                    rest.restart()
                    return
                }
                rest.stop()
                if (row.host !== null && row.host.cardRow === row)
                    row.host.cardWanted = false
            }
        }
        Timer {
            id: rest
            interval: Metrics.tipDelayMs
            onTriggered: {
                if (row.host !== null && rowHover.hovered)
                    row.host.openCard(row, rowHover.point.position.x)
            }
        }
        TapHandler {
            onTapped: row.page.pickRecover(row.at)
        }

        // The mark of what it took and where, in the seat a left menu row draws its own in (`NameCell`): the left
        // menu's tint for that kind (`RecoverEntries.tintOf`).
        RecoverMark {
            id: entryMark
            x: Theme.spaceXs
            anchors.verticalCenter: parent.verticalCenter
            kind: row.entry.mark
            tint: row.entry.tint
            badged: row.entry.badged
            paired: row.entry.paired
        }
        // The seat and the name, as every left menu row opens, cut at the end: what was done stays readable, the
        // name it was done to gives way. The whole of it is the card's and the band's. A pair of marks is wider than
        // the seat, and that row's name moves over by the difference.
        NameCell {
            id: title
            x: Theme.spaceXs + (row.entry.paired ? entryMark.pairSpread : 0)
            y: 0
            width: whenLabel.x - Theme.spaceXs - title.x
            height: Theme.rowHeight
            showChange: false
            nameCutAt: "end"
            name: row.entry.title
        }
        Label {
            id: whenLabel
            anchors.right: parent.right
            anchors.rightMargin: Theme.navBarGutter
            y: (Theme.rowHeight - whenLabel.height) / 2
            text: row.host !== null && row.host.words !== null ? row.host.words.ago(row.entry.at, row.now) : ""
            font.pixelSize: Theme.fontSm
            color: Theme.textMuted
        }
    }
}
