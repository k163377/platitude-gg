pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A report with nothing to decide (デザイン規約 §答えの要らない報せ): the question bar's shape, with the pill turned into
// the one word that takes it back up. One report stands at a time, above whatever question is standing.
Rectangle {
    id: bar

    /// What did not happen, in this application's words (`Words.writeReported`).
    property string label: ""
    /// Why, in this application's words too — who said no, and where their own words are (`Words.writeReportedWhy`).
    property string detail: ""
    /// Which state, if any, this report is in — `danger` / `warning` / empty (`Words.reportTone`).
    property string tone: ""
    /// The names in the heading that wear their kind's mark in front of them (デザイン規約 §ref の種別「名前の印」) —
    /// a worktree's, a branch's, a tag's, the remote's — at the seat the heading's own sentence gives the name
    /// (`Words.reportMarks`), so a translation that moves the name moves the mark with it (`CardText.marks`). Empty for
    /// none.
    property var marks: []
    /// The same for the line under it.
    property var detailMarks: []
    /// Automation: whether the heading names a worktree — the report whose name is a folder's, not a ref's.
    readonly property bool namesWorktree: bar.marks.some(mark => mark[1] === "tree" || mark[1] === "home")
    /// The one word of `detail` that is a place to go, and where (デザイン規約 §hover のツールチップ「2 行目の中の 1 語は
    /// 行き先にできる」, `Words.commandsHref`); empty for none.
    property string detailPlace: ""
    property string detailHref: ""
    /// That word was pressed; the page goes there (`RepoPage.tipLinkAsked`).
    signal linkAsked(string href)
    /// Automation (`PGG_AUTO_ACT=notice-link`): presses the place word at its own signal — the glyphs are the one part
    /// a run cannot press (`Main.pressTipLink`). Answers whether there was one to press.
    function pressDetailLink() {
        // Only a line that drew its link has one to press: the place word missing from the sentence leaves it plain.
        if (bar.detailPlace === "" || bar.detailHref === "" || !detailLink.visible || detailLink.placeAt < 0)
            return false
        bar.linkAsked(bar.detailHref)
        return true
    }
    /// Whether the report stands. **The words stay put while the bar goes back up** (`AskBar.open`).
    property bool open: false
    /// Whether Escape goes to something standing above this report — two enabled `StandardKey.Cancel` shortcuts in
    /// one window fire neither, and `RepoPage` decides (rules-refs/app-ui.md「Esc は窓に 1 本しか生きられない」).
    property bool yieldsEscape: false

    /// Read and taken down. Nothing else follows from it — the write it is about is long over.
    signal acknowledged()
    /// The one body the `OK` pill and Escape both enter, so a headless run presses what a hand presses
    /// (`AskBar.dismiss`).
    function dismiss() {
        bar.acknowledged()
    }

    /// The height the words ask for, and the two edges a run photographs on (as `AskBar.settled` / `shut`).
    readonly property real openHeight: bar.open ? noticeRow.implicitHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    readonly property bool shut: !bar.open && bar.implicitHeight === 0
    /// Automation: whether either line lost its tail. **Read off the fields**: a wrapped bar and a cut one can be the
    /// same height (`tests/qml/tst_reportdress.qml`). The heading is a `CardText` that wraps; only a name in it is cut,
    /// at the name ceiling (`holdsNames`).
    readonly property bool wordsCut: detailWord.truncated
    /// Automation: whether the heading's first mark found its word, and where that word stands against the heading.
    readonly property alias markSeat: headingWord.firstMarkSeat
    readonly property bool markShown: headingWord.firstMarkSeat.width > 0 || headingWord.firstMarkSeat.height > 0
    /// …and the mark's room in front of the name: the seat as the heading lays it out (`CardText.seatWidth`).
    readonly property alias markCell: headingWord.firstMarkCell
    /// Automation: the one control, so a run can read where it stands — the middle of the bar, however many lines
    /// the words take.
    readonly property alias pill: okPill

    clip: true
    color: Theme.bgElevated
    implicitHeight: bar.openHeight
    /// Animates only the travel down and back up (`AskBar.travelling`): a standing report handed longer words takes
    /// the room in the pass that draws them, or the clip shears the extra lines.
    property bool travelling: false
    onSettledChanged: if (bar.settled) bar.travelling = false
    onShutChanged: if (bar.shut) bar.travelling = false
    Behavior on implicitHeight {
        enabled: bar.travelling
        NumberAnimation { duration: 200 }
    }
    // The hairline carries the tone (`Words.reportTone`); the heading and words keep their own colours
    // (デザイン規約 §回数押し「警告の色は枠と印が持つ」).
    BandRule {
        color: bar.tone === "danger" ? Theme.danger
             : bar.tone === "warning" ? Theme.warning
             : Theme.borderSubtle
    }

    // Opening hands the pill the focus — a tick later, or the gesture still being delivered takes it back (`AskBar`).
    onOpenChanged: {
        // First, ahead of the height binding (`travelling`).
        bar.travelling = true
        if (bar.open) {
            okPill.tookTheOpening = true
            Qt.callLater(okPill.forceActiveFocus)
        }
    }

    RowLayout {
        id: noticeRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spaceMd
        spacing: Theme.spaceMd
        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            // **Wrapped, never cut** (デザイン規約 §答えの要らない報せ): the heading names a branch and a remote — a name
            // alone is cut at the name ceiling, given back whole under the hand (`holdsNames`). A `CardText`, since
            // only a text field answers where a character stands — what the mark in front of each name is set against
            // (as `TipSeat`). Each mark is a sign on its name, as everywhere a name stands (デザイン規約
            // §アイコンの席 の修飾).
            CardText {
                id: headingWord
                Layout.fillWidth: true
                text: bar.label
                marks: bar.marks
                holdsNames: true
                color: Theme.textPrimary
                pixelSize: Theme.fontMd
                weight: Theme.fontWeightStrong
            }
            // Wrapped, never cut: nothing caps the height (デザイン規約 §答えの要らない報せ). Plain text: a lock's reason
            // is somebody's own words, and a `<` in them is a character, not markup.
            Label {
                id: detailWord
                Layout.fillWidth: true
                visible: bar.detailPlace === "" && bar.detailMarks.length === 0
                text: bar.detail
                textFormat: Text.PlainText
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                wrapMode: Text.Wrap
            }
            // A line naming a place to go, or a name behind its mark: a `CardText`, the one field whose words can carry
            // a link (as `TipSeat`) or say where a name stands, the place in the tip's dress — `textMuted`,
            // `textSecondary` under the pointer, underlined.
            CardText {
                id: detailLink
                Layout.fillWidth: true
                visible: bar.detailPlace !== "" || bar.detailMarks.length > 0
                text: bar.detail
                marks: bar.detailMarks
                // The field links it among the names it sets as code (`CardText.place`).
                place: bar.detailPlace
                placeHref: bar.detailHref
                placeTint: detailLink.pointedLink !== "" ? Theme.textSecondary : Theme.textMuted
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
                onLinkAsked: href => bar.linkAsked(href)
            }
        }
        Rectangle {
            id: okPill
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: okWord.implicitWidth + 2 * Theme.spaceMd
            implicitHeight: Theme.controlHeight
            radius: Theme.radiusSm
            color: okMouse.containsMouse ? Theme.bgHover : "transparent"
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            activeFocusOnTab: true
            // Closed, it leaves the tab order by going disabled — Qt refuses to clear `activeFocusOnTab` on the item
            // holding the focus (`AskBar`).
            enabled: bar.open
            Accessible.role: Accessible.Button
            Accessible.name: okWord.text
            /// The focus came with a press, or was handed over on opening: no ring for either (`AskBar`).
            property bool tookAPress: false
            property bool tookTheOpening: false
            onActiveFocusChanged: {
                if (okPill.activeFocus)
                    return
                okPill.tookAPress = false
                okPill.tookTheOpening = false
            }
            Rectangle {
                anchors.fill: parent
                anchors.margins: -Theme.spaceXs / 2
                color: "transparent"
                border.color: Theme.borderFocus
                border.width: Theme.borderWidth
                radius: Theme.radiusMd
                visible: okPill.activeFocus && !okPill.tookAPress && !okPill.tookTheOpening
            }
            Label {
                id: okWord
                anchors.centerIn: parent
                text: qsTr("OK")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
            }
            MouseArea {
                id: okMouse
                anchors.fill: parent
                hoverEnabled: true
                onPressed: okPill.tookAPress = true
                // `released`, not `clicked`: Qt drops `clicked` once its press-and-hold timer has gone off (`AskBar`).
                onReleased: if (containsMouse) bar.dismiss()
            }
            Keys.onPressed: event => {
                if (event.key !== Qt.Key_Space && event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter)
                    return
                bar.dismiss()
                event.accepted = true
            }
        }
    }
    // Escape as a shortcut: the focus may be back on the list underneath (`AskBar`). It gives way to a standing
    // question (`yieldsEscape`) — this is only news.
    Shortcut {
        id: escapeKey
        // `sequences`: Cancel is more than one key on some platforms, and `sequence` takes only the first.
        sequences: [StandardKey.Cancel]
        enabled: bar.open && !bar.yieldsEscape
        onActivated: bar.dismiss()
    }
    /// Automation: whether Escape is this bar's to take at this moment (`AskBar.escapes`, read the same way).
    readonly property alias escapes: escapeKey.enabled
}
