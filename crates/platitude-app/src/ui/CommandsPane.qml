pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The git commands this tab ran, oldest first. Hidden until asked for, and raised on its own when something the user
// asked for fails — until the error surfaces are built, this is where a failure is read in full.
//
// **Its band is the left menu's last row, carried on across the window** (デザイン規約 §git が言ったことを読む場所): the
// same mark at the same step in the same column, the same name after it, and the panel's own controls filling the run
// the pane's foot had nothing in. Pressing that mark is what took the row down here, and pressing it again puts it
// back at the foot of the pane.
//
// Nothing is written to disk and nothing survives the tab: the model keeps the last few hundred rows and drops the
// rest.
Rectangle {
    id: pane

    required property var commandsModel
    /// The page this panel belongs to: the seat in its band reads the log and the error line off it, the same as the
    /// seat at the foot of the pane does while the panel is down.
    required property var curPage
    /// A failure that never became a command row (a background read that gave up). Shown in the header, cleared by
    /// clicking it — and by `Clear`, which takes everything the panel says at once.
    property string errorText: ""

    /// A reader was sent here from somewhere else in the window, and this says so until they do something next
    /// (`RepoPage.commandsAttention`, デザイン規約 §hover のツールチップ). The same mark the right pane wears for the
    /// same reason — its own frame in `accent` (`MessageEditor.attention`).
    property bool attention: false

    signal closeRequested()
    signal errorCleared()
    signal copyRequested(string text)

    color: Theme.bgBase

    /// Automation: the colour the `>_` in this band came out to (`PGG_AUTO_ACT=commands-clear`).
    readonly property alias markColor: seat.markColor

    /// The panel opens on its newest row, wherever it was raised from — the row that has just been added is the one
    /// somebody came here to read. The page raises it; where it is looking when it comes up is this panel's own
    /// answer. **Coming up is being built**: the page builds the panel into its seat as the log is raised and takes
    /// it down as the log is shut (`RepoPage.commandsSeat`), so completion and coming on screen are one
    /// event — a `visible` that moved would be a seat wired some other way.
    function cameUp() {
        pane.showLatest()
        pane.tellTheZone()
        // The keys this panel answers are the list's, and a panel nobody has clicked in has not been given them
        // (Ctrl+C over a log that is on screen is what a reader would expect to work).
        list.forceActiveFocus()
    }
    Component.onCompleted: pane.cameUp()

    /// The machine's offset from UTC, which is the one thing about the clock the model cannot work out for itself
    /// (`CommandsModel.setZoneMinutes`). Said again every time the panel comes up (`RepoPage`'s model says it
    /// first), so a session carried across a change of offset stamps what arrives
    /// afterwards with the new one.
    function tellTheZone() {
        pane.commandsModel.setZoneMinutes(new Date().getTimezoneOffset())
    }
    // …and a failure that arrives with the panel already up puts it back at the end as well, over a reader who had
    // scrolled away from it. The rows follow on their own while nobody has (`list.follow`), so this is only about the
    // reader who has: a failure is the one row worth taking them back to, which is the same call the page makes when
    // it raises the panel for one.
    Connections {
        target: pane.commandsModel
        function onFailure() {
            if (pane.visible)
                pane.showLatest()
        }
    }

    /// Puts the newest row back in view — what the panel opens on.
    function showLatest() {
        list.follow = true
        list.positionViewAtEnd()
    }

    /// What `Clear` empties: the rows and the line in the header both. The mark is red for either of them, so a Clear
    /// that left the line standing left the mark red over an empty panel, with nothing on screen left to explain it.
    ///
    /// And then the panel goes down with them. A panel raised by a failure is read once; the
    /// press that says "I am done with this" is the same press that empties it, and what stays behind otherwise is a
    /// panel saying `Nothing yet` over the graph it pushed out of the way. Reopening is the `>_`, which goes back to
    /// the foot of the left menu with the panel.
    function clearPanel() {
        pane.commandsModel.clear()
        pane.errorCleared()
        pane.closeRequested()
    }

    /// What Ctrl+C puts on the clipboard: the text the reader dragged over
    /// (デザイン規約 §git が言ったことを読む場所). Nothing goes out for a drag that took nothing — an empty clipboard is
    /// worse than the one the reader already had — and saying so is what lets the key fall through to whoever else
    /// wants it.
    function copySelection() {
        const text = pane.commandsModel.selectionText()
        if (text === "")
            return false
        pane.copyRequested(text)
        return true
    }

    /// Automation: the hand that picks text, without a pointer behind it (verify-ui). It enters the same three
    /// functions the `MouseArea`'s own handlers call, so a run cannot pass while the handlers do something else.
    function pickText(fromRow, fromAt, toRow, toAt) {
        textPick.pressText(fromRow, fromAt)
        textPick.dragText(toRow, toAt)
        textPick.releaseText()
    }
    /// Automation: the same hand started on the ground under the last row — the one place inside this panel's own
    /// frame that no row is under (`PGG_AUTO_ACT=commands-sweep`). It goes in through the pixels, because the
    /// pixels are the whole of the claim.
    function sweepGround(fx, fy) {
        return textPick.sweepFromGround(fx, fy)
    }
    /// Whether there is any of that ground to sweep from — a log that fills its panel has none, and a run that swept
    /// one would prove nothing — and where it begins, which is the geometry a run watches settle before it sweeps.
    readonly property bool hasGround: textPick.hasGround
    readonly property real groundTop: textPick.groundTop
    /// Automation: how many rows are wearing a rectangle of the selection, and whether every row that exists is
    /// (`PGG_AUTO_ACT=commands-select`).
    ///
    /// **Counted on the paint.** A row can hold its whole share of the selection and draw nothing — which
    /// is exactly what a wash asked of the row's layout does when it is asked at the wrong moment (measured: the
    /// runs were right, the rectangles were empty, and the run went green because the picture of an unwashed log
    /// and the picture of a log nobody dragged over are the same picture). So this counts the rectangles.
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

    // The mark, over everything this panel draws. A frame (the right pane's box recolours one it already had):
    // this panel has no frame of its own, and the split it stands in is the whole
    // of what separates it from the pane above.
    //
    // **It reaches up over that separator.** Drawn inside the panel's own top edge the line comes out under the
    // split's, and the two read as a hairline pair (measured).
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: -Theme.splitterWidth
        visible: pane.attention
        color: "transparent"
        border.color: Theme.accent
        border.width: Theme.borderWidth
        // Over the header band and the rows both, and over nothing that can be pressed: the frame is a line on the
        // panel's own edges, where none of its controls are.
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
            // The seat carries the pane's inset at the near end, so the mark and the name stand in the
            // columns they stood in at the foot of the list.
            RowLayout {
                anchors.fill: parent
                anchors.rightMargin: Theme.spaceMd
                // The sections' own step, since this band opens with one of their rows: at `spaceSm` the count would
                // sit twice as far from the name here as `(2)` does from `BRANCHES`.
                spacing: Theme.spaceXs

                // The row this band is: the mark, and the name it has been carrying all along.
                CommandsToggle {
                    id: seat
                    Layout.fillHeight: true
                    captioned: true
                    ruled: false
                    curPage: pane.curPage
                }
                Label {
                    text: "(" + list.count + ")"
                    font.pixelSize: Theme.fontMd
                    color: Theme.textMuted
                }
                // Why the panel is up, in git's own words. One line here; the row that failed keeps the whole of it.
                //
                // git's messages run to several lines, and eliding does not make text one line — it trims the last one.
                // Without a cap the band grows to fit them and the message is painted over the rows below it.
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
                    // The panel's own band is shorter than a form row, so this box keeps the band's height
                    // (`AppCheckBox` opens at the control height).
                    implicitHeight: Theme.iconLg
                    checked: pane.commandsModel.backgroundReads
                    // The reads a repository page makes on a timer are nobody's doing and would bury the rest, so they
                    // are off until asked for — and only from here on.
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
            // Nothing above the first row: the band is already the edge, and a sliver of ground under it reads as the
            // log hanging off its own header — 4 pixels of it stood between the band's
            // rule and the first row, which a failure's lit ground and red edge draw for anyone to see (observed).
            // The graph keeps a top margin because it has no band over it (`GraphList`).
            bottomMargin: Theme.spaceXs
            // The middle button is the pane's: the hand that picks the text is laid over these rows, and the one that
            // sends them has to stand over that (below).
            ownsHand: false

            /// Whether new rows pull the view along. Reading further up stops that until the end is reached again — the
            /// same rule the graph follows: the ground stays put under a reader.
            property bool follow: true
            onMovementEnded: list.follow = list.atYEnd
            onCountChanged: if (list.follow) list.positionViewAtEnd()

            // The one key this panel answers. `StandardKey`, so the platform's idea
            // of copy is what is matched — the same way the diff answers it (規約 §diff の中身をコピーする).
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
                // Children of a ListView are adopted by its contentItem, which is 0×0 while the list is empty — pin the
                // label to the view itself so "empty" is said in its middle.
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
    /// One column of the mono font the rows are drawn in, measured. **It is the two spacings the
    /// row is built with**: the space between `git` and what follows it, and the one between the two
    /// words about how a command went (`CommandRowDelegate`). Where a column's characters fall is a
    /// question for that column's own layout (`lineRuler`).
    readonly property real charW: charMeasure.implicitWidth / charMeasure.text.length
    Text {
        id: charMeasure
        visible: false
        text: "0000000000"
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
    }
    /// Where a place in one of a row's three columns is drawn, and which place a point along it is over — asked of
    /// the column itself, laid out. One for the panel: the answer belongs to the column, so a
    /// ruler that has just been handed one answers about it in the same statement (`LineRuler`).
    LineRuler {
        id: lineRuler
        // The rows' own two. Every column of this log is the characters it holds, set in a plain `Label` at the
        // window's own size — a ruler reading the same column as markup, or at another size, is measuring text this
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
    /// Automation only: the log's hand, started and drifted without a pointer (a middle button cannot be injected),
    /// and the list it sends, for the run to read where it went.
    readonly property alias hand: hand
    readonly property alias view: list
    // **Declared after the hand that picks the text, so it stands over it** — the list's own hand would sit under that
    // one, where the click that ends a gesture lands on the text picker and starts a selection instead
    // (`AppListView.ownsHand`). Over the rows' own frame: the column this panel is laid out in fills it, so the list's
    // x and y are already in these coordinates (`CommandsTextSelect` stands on the same frame).
    MiddleAutoScroll {
        id: hand
        x: list.x
        y: list.y
        width: list.width
        height: list.height
        // Standing while the log has somewhere to go, as every list's own hand does (`AppListView`).
        visible: list.ScrollBar.vertical.visible
        onDrifted: dy => {
            list.contentY = list.clampY(list.contentY + dy)
            // Where the hand leaves the log is where the reader parked it: up the log, new rows stop pulling the view
            // along, and back at the end they pull it again (`follow`).
            list.follow = list.atYEnd
        }
    }
}
