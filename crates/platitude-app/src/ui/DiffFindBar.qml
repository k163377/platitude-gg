pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Ctrl+F over a diff: a strip under the DIFF band, the pane's full width, that the rows step down for
// (デザイン規約 §diff の中を探す). The box takes lines (Ctrl+Enter) and holds its three options and the new-line mark at
// its right end; the count, the arrows and the `✕` keep to the strip's right end. One grade under the band's own tools
// throughout: seats `iconLg` with `iconSm` marks, words in the box's `fontMd`.
Rectangle {
    id: bar

    property bool open: false
    /// Writable so a headless run can fill the box — keystrokes cannot be injected.
    property alias query: field.text
    property bool matchCase: false
    property bool wholeWord: false
    property bool regex: false
    /// A line is standing — the model says what counts as one. Only then is there anything to count, `0 / 0` included.
    property bool searching: false
    property int matches: 0
    property int atMatch: 0
    /// Rows on screen: the widest the count can get, and the width its place is held at so the caret does not slide
    /// as the hits narrow (as the graph's card, §コミットを探す).
    property int loaded: 0
    /// Why the line could not be read as a pattern, `""` while it could: the frame says so, the reason waits in the
    /// tooltip (規約 §可否・警告の出し場所).
    property string error: ""
    readonly property bool refused: bar.error !== ""
    onErrorChanged: {
        if (bar.error !== "")
            box.shownError = bar.error
    }
    /// Automation: the refusal's tip is actually up, on this box.
    readonly property bool tipShown: refusalTip.shown
    /// Automation: whether the caret is in the box.
    readonly property bool boxFocused: field.activeFocus
    /// Automation: how many lines the box holds.
    readonly property int lines: field.lineCount
    /// Automation: how far the box has been sent sideways to keep the caret in view, and whether its lines run past
    /// its width at all.
    readonly property real boxScrolled: fieldView.contentX
    readonly property bool boxOverflows: fieldView.contentWidth > fieldView.width + 0.5

    signal dismissed()
    /// Enter and Shift+Enter: on to the next hit, back to the previous.
    signal nextRequested()
    signal previousRequested()
    /// An option was flipped: the same line reads differently.
    signal optionsChanged()

    /// Brings the strip up with the caret in the box: `seed` (the lines under the hand's selection) in it where there
    /// is one, else the last query, selected. The seed goes in before the strip comes up, so one search runs — the
    /// strip's, over the seed (a line put into a strip that is down searches nothing).
    function raise(seed) {
        if (seed !== "")
            field.text = seed
        bar.open = true
        field.forceActiveFocus()
        field.selectAll()
    }
    function dismiss() {
        if (!bar.open)
            return
        bar.open = false
        bar.dismissed()
    }
    /// Closes without `dismissed`: the press that landed elsewhere owns the keyboard (as `FindBar.dropAway`).
    function dropAway() {
        bar.open = false
    }
    /// Whether a press at `scenePos` (scene coordinates) landed on this strip. `null` — a headless press with no
    /// place — is never on it.
    function holds(scenePos) {
        if (!scenePos)
            return false
        const p = bar.mapFromItem(null, scenePos)
        return p.x >= 0 && p.y >= 0 && p.x < bar.width && p.y < bar.height
    }
    function toggleCase() {
        bar.matchCase = !bar.matchCase
        bar.optionsChanged()
    }
    function toggleWord() {
        bar.wholeWord = !bar.wholeWord
        bar.optionsChanged()
    }
    function toggleRegex() {
        bar.regex = !bar.regex
        bar.optionsChanged()
    }
    /// Automation: the caret at the end of the lines, where typing them would have left it — an assignment to the
    /// text leaves it at the start.
    function caretToEnd() {
        field.cursorPosition = field.length
    }

    /// The air over and under a one-line box inside the band's height.
    readonly property real inset: (Theme.headerHeight - Theme.iconLg) / 2
    readonly property real boxMinWidth: 160
    /// One line of the box, paddings included — what the box is tall with one line in it, and the marks' seats'
    /// side, so they stand the same above and below the letters on every face.
    readonly property real lineHeight: (field.implicitHeight - field.topPadding - field.bottomPadding)
                                       / Math.max(1, field.lineCount) + field.topPadding + field.bottomPadding

    visible: bar.open
    color: Theme.bgElevated
    // The band's height with one line in the box; the box's growth grows the strip by the same.
    implicitHeight: box.implicitHeight + 2 * bar.inset
    BandRule { z: 1 }

    RowLayout {
        id: findRow
        anchors.fill: parent
        // The box's frame stands where a hunk heading's words start (`spaceXs` in from the pane's edge).
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceSm
        // The box, in the frame every box in the app wears (`SlimField`'s), grown by lines. At its right end the
        // new-line mark (IntelliJ's "Enter a new line"), then the options (VS Code's place for them).
        Rectangle {
            id: box
            Layout.fillWidth: true
            // A search box's fixed width (デザイン規約 §レイアウト初期値), here its floor; and four of them its
            // ceiling — past that a line is read sideways, and the controls would stand a pane away from the box.
            Layout.minimumWidth: bar.boxMinWidth
            Layout.maximumWidth: 4 * bar.boxMinWidth
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: bar.inset
            implicitHeight: field.implicitHeight + 2 * Theme.borderWidth
            color: Theme.bgBase
            radius: Theme.radiusSm
            border.width: Theme.borderWidth
            border.color: bar.refused ? Theme.warning
                        : field.activeFocus || menuSeat.holding ? Theme.borderFocus : Theme.borderDefault
            // The reason the line was refused (規約 §可否・警告の出し場所). That tip answers what is typed, not a hand:
            // it stands on the box's left edge until the typing changes it, wherever the hand goes
            // (`TipAsk.typed`, 規約 §hover のツールチップ「入力の拒否への補足は一律」).
            readonly property bool tipTyped: bar.refused && bar.open
            /// The reason as last said, kept through the tip's own fall: the error clears as the strip closes, and a
            /// box going down with no words in it is a flash.
            property string shownError: ""
            readonly property TipAsk refusalTip: TipAsk {
                id: refusalTip
                target: box
                asked: box.tipTyped
                typed: true
                words: box.shownError
            }
            // The lines run sideways, never wrapped — a typed line is one line of the text. Past the box's width the
            // view follows the caret, as a one-line field's does (規約 §diff を横へ送る「行は端まで在る」); up and down
            // the box grows instead, so nothing is ever out of sight that way.
            Flickable {
                id: fieldView
                anchors.left: parent.left
                anchors.right: inside.left
                anchors.top: parent.top
                anchors.margins: Theme.borderWidth
                height: field.implicitHeight
                clip: true
                flickableDirection: Flickable.HorizontalFlick
                boundsBehavior: Flickable.StopAtBounds
                TextArea.flickable: TextArea {
                    id: field
                    placeholderText: qsTr("Find in diff")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontMd
                    // Every box in the app holds its words `spaceXs` off its frame (デザイン規約 §余白).
                    leftPadding: Theme.spaceXs
                    rightPadding: Theme.spaceXs
                    topPadding: Math.max(0, (Theme.iconLg - 2 * Theme.borderWidth - fm.height) / 2)
                    bottomPadding: topPadding
                    background: null
                    wrapMode: TextEdit.NoWrap
                    // Enter steps through the hits and Ctrl+Enter breaks the line (VS Code, Zed); the options flip on
                    // Alt+C / W / R (VS Code; IntelliJ's C / W). Escape takes the strip down.
                    Keys.onPressed: event => {
                        if (event.key === Qt.Key_Escape) {
                            bar.dismiss()
                            event.accepted = true
                        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                            if (event.modifiers & Qt.ControlModifier)
                                field.insert(field.cursorPosition, "\n")
                            else if (event.modifiers & Qt.ShiftModifier)
                                bar.previousRequested()
                            else
                                bar.nextRequested()
                            event.accepted = true
                        } else if (event.modifiers & Qt.AltModifier) {
                            if (event.key === Qt.Key_C)
                                bar.toggleCase()
                            else if (event.key === Qt.Key_W)
                                bar.toggleWord()
                            else if (event.key === Qt.Key_R)
                                bar.toggleRegex()
                            else
                                return
                            event.accepted = true
                        }
                    }
                    FontMetrics { id: fm; font: field.font }
                    // The product's right-click menu, not the style's, and the menu key's (`FieldMenuSeat`).
                    ContextMenu.menu: null
                    ContextMenu.onRequested: menuSeat.offer()
                    Keys.onMenuPressed: event => event.accepted = menuSeat.offer()
                    FieldMenuSeat {
                        id: menuSeat
                        editor: field
                    }
                }
            }
            Row {
                id: inside
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: Theme.borderWidth
                // A step of air before the frame, so the last mark's own frame never meets the box's.
                anchors.rightMargin: Theme.spaceXs
                spacing: 0
                FindOptionGlyph {
                    mark: "newline"
                    side: bar.lineHeight
                    tip: qsTr("New line (Ctrl+Enter)")
                    onClicked: {
                        field.insert(field.cursorPosition, "\n")
                        field.forceActiveFocus()
                    }
                }
                FindOptionGlyph {
                    mark: "case"
                    side: bar.lineHeight
                    on: bar.matchCase
                    tip: qsTr("Match case (Alt+C)")
                    onClicked: bar.toggleCase()
                }
                FindOptionGlyph {
                    mark: "word"
                    side: bar.lineHeight
                    on: bar.wholeWord
                    tip: qsTr("Whole word (Alt+W)")
                    onClicked: bar.toggleWord()
                }
                FindOptionGlyph {
                    mark: "regex"
                    side: bar.lineHeight
                    on: bar.regex
                    tip: qsTr("Regular expression (Alt+R)")
                    onClicked: bar.toggleRegex()
                }
            }
        }
        // The controls keep to the right end, whatever the box's width.
        Item { Layout.fillWidth: true }
        // Emptied by its text, never hidden: a Layout skips an invisible item and its spacing with it. Held at its
        // widest (`loaded`) and right-aligned, so the numbers grow away from the box.
        Label {
            id: count
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: (Theme.headerHeight - implicitHeight) / 2
            Layout.preferredWidth: widest.implicitWidth
            horizontalAlignment: Text.AlignRight
            text: bar.searching ? qsTr("%1 / %2").arg(bar.atMatch).arg(bar.matches) : ""
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMd
            Label {
                id: widest
                visible: false
                font: count.font
                text: qsTr("%1 / %2").arg(bar.loaded).arg(bar.loaded)
            }
        }
        Row {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: bar.inset
            spacing: 0
            HoverToolButton {
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                padding: 0
                tip: qsTr("Previous match (Shift+Enter)")
                onClicked: bar.previousRequested()
                contentItem: Item {
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconSm
                        height: Theme.iconSm
                        kind: "chevron"
                        rotation: -90
                        tint: Theme.textSecondary
                    }
                }
            }
            HoverToolButton {
                implicitWidth: Theme.iconLg
                implicitHeight: Theme.iconLg
                padding: 0
                tip: qsTr("Next match (Enter)")
                onClicked: bar.nextRequested()
                contentItem: Item {
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconSm
                        height: Theme.iconSm
                        kind: "chevron"
                        rotation: 90
                        tint: Theme.textSecondary
                    }
                }
            }
        }
        CloseToolButton {
            Layout.alignment: Qt.AlignTop
            Layout.topMargin: bar.inset
            Accessible.name: qsTr("Close the find bar")
            onClicked: bar.dismiss()
        }
    }
}
