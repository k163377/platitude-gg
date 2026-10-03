pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The git commands this tab ran, oldest first. Hidden until asked for; raised on its own when something the user asked
// for fails. Its band is the left menu's last row carried across the window (デザイン規約 §git が言ったことを読む場所).
// Nothing survives the tab: the model keeps the last few hundred rows.
Rectangle {
    id: pane

    required property var commandsModel
    /// The page this panel belongs to; the seat in its band reads the log and the error line off it.
    required property var curPage
    /// A failure that never became a command row (a background read that gave up). Shown in the header; cleared by
    /// clicking it or by `Clear`.
    property string errorText: ""

    /// A reader was sent here from elsewhere in the window: framed in `accent` until their next move
    /// (`RepoPage.commandsAttention`, デザイン規約 §hover のツールチップ).
    property bool attention: false

    signal closeRequested()
    signal errorCleared()
    signal copyRequested(string text)

    color: Theme.bgBase

    /// Automation: the colour the `>_` in this band came out to (`PGG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: seat.markColor

    /// The panel opens on its newest row. Coming up is being built: the page builds the panel as the log is raised
    /// and takes it down as it is shut (`RepoPage.commandsSeat`), so completion is coming on screen.
    function cameUp() {
        pane.showLatest()
        pane.tellTheZone()
        // The list takes the keys, so Ctrl+C works before anyone clicks in.
        list.forceActiveFocus()
    }
    Component.onCompleted: pane.cameUp()

    /// The machine's UTC offset, which the model cannot work out itself (`CommandsModel.setZoneMinutes`). Said again
    /// each time the panel comes up, so a session carried across an offset change stamps later rows with the new one.
    function tellTheZone() {
        pane.commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    // A failure arriving with the panel up takes a reader who scrolled away back to the end; otherwise the rows
    // follow on their own (`list.follow`).
    Connections {
        target: pane.commandsModel
        function onFailure() {
            if (pane.visible)
                pane.showLatest()
        }
    }

    function showLatest() {
        list.follow = true
        list.positionViewAtEnd()
    }

    /// `Clear` empties the rows and the header line both — the mark is red for either — and takes the panel down
    /// (デザイン規約 §git が言ったことを読む場所).
    function clearPanel() {
        pane.commandsModel.clear()
        pane.errorCleared()
        pane.closeRequested()
    }

    /// Ctrl+C: copies the dragged-over text. An empty selection answers false, leaving the clipboard alone and letting
    /// the key fall through.
    function copySelection() {
        const text = pane.commandsModel.selectionText()
        if (text === "")
            return false
        pane.copyRequested(text)
        return true
    }

    /// Automation: the text-picking hand without a pointer, through the same three functions its handlers call.
    function pickText(fromRow, fromAt, toRow, toAt) {
        textPick.pressText(fromRow, fromAt)
        textPick.dragText(toRow, toAt)
        textPick.releaseText()
    }
    /// Automation: the same hand started on the ground under the last row, in pixels
    /// (`PGG_AUTO_ACT=commands-sweep`).
    function sweepGround(fx, fy) {
        return textPick.sweepFromGround(fx, fy)
    }
    /// Whether there is ground to sweep from (a full log has none), and where it begins.
    readonly property bool hasGround: textPick.hasGround
    readonly property real groundTop: textPick.groundTop
    /// Automation: how many rows wear a wash rectangle, and whether all do (`PGG_AUTO_ACT=commands-select`). Counted
    /// on the paint: a row can hold its selection and draw nothing, and an unwashed log photographs like an untouched
    /// one.
    function washTally() {
        let seen = 0
        let worn = 0
        for (let i = 0; i < list.count; i++) {
            const row = list.itemAtIndex(i)
            if (!row)
                continue
            seen++
            if (row.washRight > row.washX)
                worn++
        }
        return "worn=" + (seen > 0 && worn === seen) + " rows=" + seen + " washed=" + worn
    }

    // The attention frame, drawn since this panel has none; its top edge lies over the split's line
    // (デザイン規約 §hover のツールチップ「送った先は名乗る」).
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -Theme.splitterWidth
        visible: pane.attention
        color: "transparent"
        border.color: Theme.accent
        border.width: Theme.borderWidth
        // Over the band and the rows; the frame lies on the edges, where no control is.
        z: 1
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            BandRule { z: 1 }
            // No left margin: the seat carries the pane's inset, so the mark and the name keep their columns from the
            // foot of the list.
            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceXs

                CommandsToggle {
                    id: seat
                    Layout.fillHeight: true
                    captioned: true
                    ruled: false
                    counted: true
                    count: list.count
                    curPage: pane.curPage
                }
                // One line here; the failed row keeps the whole. Capped: eliding trims only the last line of a
                // multi-line message, and the band would grow over the rows.
                Label {
                    Layout.fillWidth: true
                    visible: pane.errorText !== ""
                    text: "— " + pane.errorText
                    elide: Text.ElideRight
                    maximumLineCount: 1
                    font.pixelSize: Theme.fontSm
                    color: Theme.danger
                    MouseArea {
                        anchors.fill: parent
                        onClicked: pane.errorCleared()
                    }
                }
                Item {
                    Layout.fillWidth: true
                    visible: pane.errorText === ""
                }

                AppCheckBox {
                    text: qsTr("Background reads")
                    // The band is shorter than a form row (`AppCheckBox` opens at the control height).
                    implicitHeight: Theme.iconLg
                    checked: pane.commandsModel.backgroundReads
                    // Timer reads would bury the rest, so they are off until asked for, and only from here on.
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: qsTr("Also record the reads this window makes on its own, from now on")
                    onToggled: pane.commandsModel.setBackgroundReads(checked)
                }
                HoverToolButton {
                    text: qsTr("Clear")
                    font.pixelSize: Theme.fontMd
                    onClicked: pane.clearPanel()
                }
                CloseToolButton {
                    onClicked: pane.closeRequested()
                }
            }
        }

        AppListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: pane.commandsModel
            // No top margin: the band is already the edge, and a sliver under it shows beneath a failure's lit ground.
            bottomMargin: Theme.spaceXs
            // The middle-button hand is the pane's, standing over the text picker (below).
            ownsHand: false

            /// Whether new rows pull the view along; reading further up stops it until the end is reached again.
            property bool follow: true
            onMovementEnded: list.follow = list.atYEnd
            onCountChanged: if (list.follow) list.positionViewAtEnd()

            // `StandardKey`, so the platform's copy is what is matched (規約 §diff の中身をコピーする).
            Keys.onPressed: event => {
                if (event.matches(StandardKey.Copy))
                    event.accepted = pane.copySelection()
            }

            delegate: CommandRowDelegate {
                width: list.width
                charW: pane.charW
                commandsModel: pane.commandsModel
                ruler: lineRuler
            }

            Label {
                // A ListView's children go to its contentItem, 0×0 while empty, so the label is pinned to the view.
                parent: list
                anchors.centerIn: parent
                visible: list.count === 0
                text: qsTr("Nothing yet — the commands this window runs turn up here")
                color: Theme.textMuted
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // ---- the hand that picks the text ---------------------------------------------------------------------------
    /// One measured column of the rows' mono font, for the row's two spacings only (`CommandRowDelegate.charW`).
    readonly property real charW: charMeasure.implicitWidth / charMeasure.text.length
    Text {
        id: charMeasure
        visible: false
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// Where a place in a row's column is drawn, and which place a point is over — asked of the column laid out. One
    /// for the panel: it answers about whichever column it was just handed.
    LineRuler {
        id: lineRuler
        // The rows' format and size: a ruler reading the column as markup, or at another size, measures text this
        // panel never draws.
        textFormat: TextEdit.PlainText
        font.pixelSize: Theme.fontSm
    }

    CommandsTextSelect {
        id: textPick
        view: list
        commandsModel: pane.commandsModel
        barRoom: list.barRoom
        ruler: lineRuler
        onScrollWanted: dy => list.contentY = list.clampY(list.contentY + dy)
    }

    // ---- the middle button ----------------------------------------------------------------------------------------
    /// Automation only: the middle-button hand (which cannot be injected), and the list it sends.
    readonly property alias hand: hand
    readonly property alias view: list
    // Declared after the text picker, so it stands over it: under it, the click ending a gesture would start a
    // selection (`AppListView.ownsHand`). The list's x / y are already in this frame, as for `CommandsTextSelect`.
    MiddleAutoScroll {
        id: hand
        x: list.x
        y: list.y
        width: list.width
        height: list.height
        // Standing while the log can scroll, as in `AppListView`.
        visible: list.ScrollBar.vertical.visible
        onDrifted: dy => {
            list.contentY = list.clampY(list.contentY + dy)
            // Where the hand leaves the log sets `follow`, as a reader's own scroll does.
            list.follow = list.atYEnd
        }
    }
}
