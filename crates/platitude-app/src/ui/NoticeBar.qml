pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A report with nothing to decide (デザイン規約 §答えの要らない報せ): the question bar's shape, with the pill turned into
// the one word that takes it back up. One report stands at a time, above whatever question is standing.
//
// `OK` only says "read": one click takes the bar down, and Escape does the same — a count of presses is for an answer
// that loses something (§回数押し). No ✕: the question bar's ✕ walks away without answering, and a report has nowhere
// to walk away to.
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
        if (bar.detailPlace === "" || bar.detailHref === "" || noticeBody.item === null
                || !noticeBody.item.linkDrawn)
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

    /// The most the bar may stand at: what its column leaves it with the graph's rows kept and what else stands there
    /// at its least (`RepoPage`, デザイン規約 §窓の床). Past it the words scroll, a line of them at the least; the `OK`
    /// stays on the bar's middle.
    property real room: Infinity
    /// The bar at its least — a line of the heading beside the `OK` — which the column and the window's floor keep for
    /// it (`PageLayout.panesMinHeight`), whatever words it holds (`lineRuler`).
    readonly property real leastHeight: Math.ceil(2 * Theme.spaceMd
                                                  + Math.max(lineRuler.setLineHeight, Theme.controlHeight))
    /// What the window's floor keeps for it: its least, the same whatever it says.
    readonly property real floorHeight: bar.leastHeight
    /// The height the words ask for, and the two edges a run photographs on (as `AskBar.settled` / `shut`). It grows
    /// by the wrapped line up to `room`, and a run shoots that as `AskBar.openHeight` says.
    readonly property real openHeight: bar.open && noticeBody.item !== null
                                       ? noticeBody.item.implicitHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    readonly property bool shut: !bar.open && bar.implicitHeight === 0
    /// Automation: whether either line lost its tail. **Read off the fields**: a wrapped bar and a cut one can be the
    /// same height (`tests/qml/tst_reportdress.qml`). The heading is a `CardText` that wraps; only a name in it is cut,
    /// at the name ceiling (`holdsNames`).
    readonly property bool wordsCut: noticeBody.item !== null && noticeBody.item.wordsCut
    /// Automation: whether some of the words are a scroll away (`room`) — kept, not cut.
    readonly property bool wordsScroll: noticeBody.item !== null && noticeBody.item.wordsScroll
    /// Automation: whether the heading's first mark found its word, and where that word stands against the heading.
    readonly property var markSeat: noticeBody.item !== null ? noticeBody.item.markSeat : Qt.rect(0, 0, 0, 0)
    readonly property bool markShown: bar.markSeat.width > 0 || bar.markSeat.height > 0
    /// …and the mark's room in front of the name: the seat as the heading lays it out (`CardText.seatWidth`).
    readonly property real markCell: noticeBody.item !== null ? noticeBody.item.markCell : 0
    /// Automation: the one control, so a run can read where it stands — the middle of the bar, however many lines
    /// the words take. Null until the first report (`noticeBody`).
    readonly property var pill: noticeBody.item !== null ? noticeBody.item.pill : null

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

    // A line of the heading with a name on it, never shown: a name is set in git's face beside the words' and stands
    // the line taller than words alone, so a field's own line moves with what it says (`CardText.lineHeight`). What a
    // line stands at whatever the bar holds — the least the column and the window's floor keep — is read here
    // (`CardText.setLineHeight`).
    CardText {
        id: lineRuler
        visible: false
        text: "main"
        marks: [[0, "branch", Theme.textLink, 4]]
        holdsNames: true
        pixelSize: Theme.fontMd
        weight: Theme.fontWeightStrong
    }

    // Other words start at their first line, wherever the last ones were scrolled to: the heading says what it is
    // about.
    onLabelChanged: if (noticeBody.item !== null) noticeBody.item.toTop()
    onDetailChanged: if (noticeBody.item !== null) noticeBody.item.toTop()

    // Opening hands the pill the focus — a tick later, or the gesture still being delivered takes it back (`AskBar`).
    onOpenChanged: {
        // First, ahead of the height binding (`travelling`) — and the first report builds the words (`noticeBody`),
        // ahead of it too.
        bar.travelling = true
        if (bar.open) {
            noticeBody.active = true
            noticeBody.item.takeOpening()
        }
    }

    // The words and the `OK`, built by the first report and kept (rules-refs/app-ui.md — the `WindowDialogSeat`
    // line): most pages never report one, and a report going back up keeps its words while the bar travels
    // (`open`). What the column and the window's floor read stays out here (`leastHeight` off `lineRuler`).
    Loader {
        id: noticeBody
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.spaceMd
        active: false
        sourceComponent: RowLayout {
            id: noticeRow
            /// What the bar reads off the words (`wordsCut` … `pill`).
            readonly property bool wordsCut: detailWord.truncated
            readonly property bool wordsScroll: wordLines.scrolls
            readonly property var markSeat: headingWord.firstMarkSeat
            readonly property real markCell: headingWord.firstMarkCell
            readonly property Item pill: okPill
            /// The line naming a place drew its link (`pressDetailLink`).
            readonly property bool linkDrawn: detailLink.visible && detailLink.placeAt >= 0
            function toTop() {
                wordLines.toTop()
            }
            /// The pill takes the focus a tick later: the gesture still being delivered would take it back.
            function takeOpening() {
                okPill.tookTheOpening = true
                Qt.callLater(okPill.forceActiveFocus)
            }
            spacing: Theme.spaceMd
            // A column short of room scrolls them rather than cut them (`room`).
            CappedLines {
                id: wordLines
                Layout.fillWidth: true
                // Whole lines of the heading, down: a line cut through its waist under the first reads as a stray mark,
                // and a part of one past the room would come off the graph. One beside the `OK` at the least, on its
                // middle. No line measured yet (the ruler is still being made), no cap: an unbounded room over a 0 is
                // no number.
                cap: {
                    const line = lineRuler.setLineHeight
                    const left = Math.floor(bar.room - 2 * Theme.spaceMd)
                    return line > 0 ? Math.max(1, Math.floor(left / line)) * line : Infinity
                }
                gap: noticeRow.spacing
                spacing: Theme.spaceXs
                // **Wrapped, never cut** (デザイン規約 §答えの要らない報せ): the heading names a branch and a remote — a name alone is
                // cut at the name ceiling, given back whole under the hand (`holdsNames`). A `CardText`, since only a
                // text field answers where a character stands — what the mark in front of each name is set against (as
                // `TipSeat`). Each mark is a sign on its name, as everywhere a name stands (デザイン規約 §アイコンの席 の修飾).
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
                // Wrapped, never cut (デザイン規約 §答えの要らない報せ). A line can carry somebody's own words (a lock's reason) and
                // run long; the bar grows by the words and the middle of the page goes down by as much, up to what the
                // column has, past which they scroll. Plain text: a `<` in those words is a character, not markup.
                PlainTextLabel {
                    id: detailWord
                    Layout.fillWidth: true
                    visible: bar.detailPlace === "" && bar.detailMarks.length === 0
                    text: bar.detail
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                    wrapMode: Text.Wrap
                }
                // A line naming a place to go, or a name behind its mark: a `CardText`, the one field whose words can
                // carry a link (as `TipSeat`) or say where a name stands, the place in the tip's dress — `textMuted`,
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
                    color: Theme.textSecondary
                    pixelSize: Theme.fontSm
                    onLinkAsked: href => bar.linkAsked(href)
                }
            }
            // Stays in the middle of the bar however many lines the words take.
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
                // Closed, it leaves the tab order by going disabled — Qt refuses to clear `activeFocusOnTab` on the
                // item holding the focus (`AskBar`).
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
                    // `released`, not `clicked`: Qt drops `clicked` once its press-and-hold timer has gone off
                    // (`AskBar`).
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
