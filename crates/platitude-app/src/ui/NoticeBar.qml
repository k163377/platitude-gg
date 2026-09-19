pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The shape a report takes when there is nothing to decide (デザイン規約 §可否・警告の出し場所): the same bar the standing
// questions come down in, with the pill turned into the one word that takes it back up.
//
// **Nothing here failed and nothing is being asked.** The far side turned a write down under a rule of its own — a
// protected branch, a hook — and the whole answer is what it said for itself: this end could not have known beforehand
// and cannot do anything about it now. So the bar wears no state colour, the heading says what did not happen, and the
// line under it is the far side's own words, quoted — or this application's own where nobody over there ever saw the
// write (デザイン規約 §答えの要らない報せ).
//
// One report stands at a time, above whatever question is standing: both push the history down.
Rectangle {
    id: bar

    /// What did not happen, in this application's words (`Words.remoteRefused`).
    property string label: ""
    /// Why, in the words of whoever said no — passed through as it came where somebody over there said it, and
    /// written here where nobody did: the writes this end withheld, and the push git turned down without the far
    /// side hearing of it (`Words.writeReportedWhy`).
    property string detail: ""
    /// Which state, if any, this report is in — `danger` / `warning` / empty (`Words.reportTone`).
    property string tone: ""
    /// Whether the report stands. **The words stay put** while the bar goes back up, since the bar is on
    /// screen for the whole 200ms it spends going (`AskBar.open` carries the same rule and the reason).
    property bool open: false
    /// Whether the report hands Escape over to something standing above it. **Two enabled `StandardKey.Cancel`
    /// shortcuts in one window fire neither** (`tests/qml/tst_escape.qml`) — not one winning, both dying — and this
    /// bar and the ask bar can stand at the same time, so one of them has to give way. **Which one is decided in
    /// the single place the two meet** (`RepoPage`), and this is how the bar that gives way
    /// is told (デザイン規約 §答えの要らない報せ).
    property bool yieldsEscape: false

    /// Read and taken down. Nothing else follows from it — the write it is about is long over.
    signal acknowledged()
    /// The way out of a standing report, and **the body both gestures that take it enter** — the `OK` pill and
    /// Escape. Named so a headless run presses what a hand presses (verify-ui, `AskBar.dismiss`).
    function dismiss() {
        bar.acknowledged()
    }

    /// The height the words ask for, before the 200ms takes it there — and the two edges a run photographs on: all the
    /// way down, and all the way back up (`AskBar.settled` / `shut`, same reasoning).
    readonly property real openHeight: bar.open ? noticeRow.implicitHeight + 2 * Theme.spaceMd : 0
    readonly property bool settled: bar.openHeight > 0 && bar.implicitHeight === bar.openHeight
    readonly property bool shut: !bar.open && bar.implicitHeight === 0
    /// Automation: whether either line lost its tail to the bar's width. **Read off the fields themselves**
    /// (`Text.truncated`), because a bar that wrapped and a bar that cut are the same height in every reading that
    /// asks the bar instead of the words (`tests/qml/tst_reportdress.qml`).
    readonly property bool wordsCut: headingWord.truncated || detailWord.truncated
    /// Automation: the one control, so a run can read where it stands — the middle of the bar, however many lines
    /// the words take.
    readonly property alias pill: okPill

    clip: true
    color: Theme.bgElevated
    implicitHeight: bar.openHeight
    /// The same 200ms the question bar spends, **on the same two things**: coming down and going back up. A report
    /// already standing that is handed longer words — a second refusal while the first is still up — takes the room
    /// for them in the pass that draws them, or the clip shears whatever the extra lines pushed past the edge.
    /// Raised from the handler and lowered on arrival, both for the reasons `AskBar` carries.
    property bool travelling: false
    onSettledChanged: if (bar.settled) bar.travelling = false
    onShutChanged: if (bar.shut) bar.travelling = false
    Behavior on implicitHeight {
        enabled: bar.travelling
        NumberAnimation { duration: 200 }
    }
    // The band's own hairline, and **it always wears one of the two colours**: a press that was turned down has to say
    // so in colour, whoever turned it down (デザイン規約 §答えの要らない報せ). `danger` where the gesture is over and what was
    // asked for did not happen, `warning` where it is still going — which here is the rename that stopped between its
    // halves, and matches the boxes at the other end of the same axis (`SlimField.refused`).
    //
    // **The line is the whole of it**: the heading and the words under it stay in their own colours, the same way a
    // warned button keeps its word (§長押し「警告の色は枠と印が持つ」).
    BandRule {
        color: bar.tone === "danger" ? Theme.danger
             : bar.tone === "warning" ? Theme.warning
             : Theme.borderSubtle
    }

    // Opening hands the pill the focus, so the keyboard's way out needs no hunting for. A tick later: the
    // gesture that ran the write is still being delivered, and what it lands on takes the focus back if the pill
    // claims it first (`AskBar`).
    onOpenChanged: {
        // First, ahead of the bindings that read the same property: this is the travel the 200ms is for.
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
            // **Wrapped, never cut** (デザイン規約 §答えの要らない報せ): what did not happen names a branch and a remote,
            // and a heading that loses its tail is a report about something the reader cannot name.
            Label {
                id: headingWord
                Layout.fillWidth: true
                text: bar.label
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
                font.weight: Font.DemiBold
                wrapMode: Text.Wrap
            }
            // The far side's own sentences, run together and wrapped. A forge writes two or three of them
            // (`GH006: …` and then the rule that was broken) and **the rule is the last of them**, so a line cut at
            // the bar's width drops the one sentence the reader came for.
            //
            // **The bar grows with the words** and the middle steps down for it — nothing caps the height, so a hook
            // that writes a screenful takes a screenful (デザイン規約 §答えの要らない報せ).
            Label {
                id: detailWord
                Layout.fillWidth: true
                text: bar.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
                wrapMode: Text.Wrap
            }
        }
        // The one thing on the bar that acts, and it only takes the bar away.
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
            // Closed, it leaves the tab order by going disabled — Qt refuses to clear `activeFocusOnTab`
            // on the item holding the focus, and warns (`AskBar`).
            enabled: bar.open
            Accessible.role: Accessible.Button
            Accessible.name: okWord.text
            /// Whether the focus this pill holds is the one the bar handed it as it opened, or one a hand brought.
            /// **Nobody reached for it** in the first case, so the ring has nothing to report (`AskBar`).
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
                // `released` inside the pill, the same as the ask bar's: Qt stops emitting
                // `clicked` once its press-and-hold timer has gone off, so a pill held down would answer nothing.
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
    // Escape says the same thing as the pill: read, take it away. Heard as a shortcut,
    // because the focus may have been taken back by the list underneath (`AskBar`).
    //
    // **And it is the half of the pair that gives way** (`yieldsEscape`): a question standing over this report is what
    // the reader is being asked for, and this is only news.
    Shortcut {
        id: escapeKey
        // `sequences`: Cancel is more than one key on some platforms, and binding the single
        // form takes only the first of them (Qt warns about exactly this).
        sequences: [StandardKey.Cancel]
        enabled: bar.open && !bar.yieldsEscape
        // The pill's own body, for the reason the ask bar's Escape enters the ✕'s: one way out of a bar, and both
        // gestures walk it.
        onActivated: bar.dismiss()
    }
    /// Automation: whether Escape is this bar's to take at this moment (`AskBar.escapes`, read the same way).
    readonly property alias escapes: escapeKey.enabled
}
