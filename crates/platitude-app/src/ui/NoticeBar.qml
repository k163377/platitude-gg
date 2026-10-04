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
    /// Why, in the words of whoever said no — the far side's as they came, or this application's where the write
    /// never reached it (`Words.writeReportedWhy`).
    property string detail: ""
    /// Which state, if any, this report is in — `danger` / `warning` / empty (`Words.reportTone`).
    property string tone: ""
    /// The word in the heading that is a working copy's name, which wears the tree mark in front of it (デザイン規約
    /// §ref の種別「名前の印」); empty for none. Set in the heading's own text, so a translation that moves the word
    /// leaves the sentence plain (`Words.roomInSentence`).
    property string markWord: ""
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
    /// same height (`tests/qml/tst_reportdress.qml`). The heading is a `CardText`, which wraps and has no cap to cut at.
    readonly property bool wordsCut: detailWord.truncated
    /// Automation: whether the heading's tree mark found its word, and where it stands against the heading.
    readonly property alias markSeat: headingWord.markSeat
    readonly property alias markShown: headMark.visible
    /// …and how wide the mark's ink is — the room the name has to have been pushed by.
    readonly property alias markInk: headMark.inkWidth
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
    // (デザイン規約 §長押し「警告の色は枠と印が持つ」).
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
            // **Wrapped, never cut** (デザイン規約 §答えの要らない報せ): the heading names a branch and a remote. A
            // `CardText`, since only a text field answers where a character stands — what the tree mark in front of a
            // working copy's name is set against (as `SharedToolTip`).
            CardText {
                id: headingWord
                Layout.fillWidth: true
                text: bar.label
                /// How many spaces the mark's ink takes; the sentence's own space before the word is the gap.
                readonly property int markRoom:
                    bar.markWord === "" || spaceRuler.advanceWidth <= 0
                    ? 0 : Math.ceil(headMark.inkWidth / spaceRuler.advanceWidth)
                /// Where the name stands: a heading opens with it (`Words.markSeatIn`).
                readonly property int markAt: Words.markSeatIn(bar.label, bar.markWord, false)
                /// Where that gap came out (empty when none). One binding over its inputs, since `charRect` is a call
                /// (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
                readonly property rect markSeat: {
                    if (headingWord.markRoom <= 0 || headingWord.markAt < 0 || headingWord.markup === ""
                            || headingWord.width <= 0 || headingWord.height <= 0)
                        return Qt.rect(0, 0, 0, 0)
                    return headingWord.charRect(headingWord.markAt + headingWord.markRoom)
                }
                markup: headingWord.markRoom > 0
                        ? Words.roomInSentence(bar.label, headingWord.markAt, headingWord.markRoom) : ""
                color: Theme.textPrimary
                pixelSize: Theme.fontMd
                weight: Theme.fontWeightStrong
                /// A space in the heading's own font and weight, so a family with a wider space opens a wider gap.
                TextMetrics {
                    id: spaceRuler
                    font.family: Theme.uiFamily
                    font.pixelSize: Theme.fontMd
                    font.weight: Theme.fontWeightStrong
                    text: " "
                }
                /// Set against the word, so mark and name read as one: the ink ends where the word begins.
                NavIcon {
                    id: headMark
                    visible: headingWord.markSeat.width > 0 || headingWord.markSeat.height > 0
                    kind: "tree"
                    tint: Theme.textSecondary
                    width: Theme.iconSm
                    height: Theme.iconSm
                    x: headingWord.markSeat.x - (Theme.iconSm + headMark.inkWidth) / 2
                    y: headingWord.markSeat.y + (headingWord.markSeat.height - Theme.iconSm) / 2
                }
            }
            // Wrapped: a forge writes the broken rule last (after `GH006: …`), so a cut drops the sentence the reader
            // came for. Nothing caps the height (デザイン規約 §答えの要らない報せ).
            Label {
                id: detailWord
                Layout.fillWidth: true
                text: bar.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                wrapMode: Text.Wrap
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
