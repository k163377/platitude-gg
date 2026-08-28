import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line a pane draws to be read, in a field the reader can take away with them: pressed, dragged over and copied
// like any other text in this window (デザイン規約 §右のペインの字は掴める).
//
// The line half of the pair `CardText` is the other half of. That one wraps and caps its height for a card; this one
// is a single line standing in a row, so **what it cuts is its width** — and it cuts the same way, by clipping rather
// than eliding: `TextEdit` has no `elide`, and a field whose text had been elided into it would hand the reader
// `Yuki Tana…` when they dragged over it. The whole value stays in the field, what does not fit is clipped, and the
// mark on the pane's own ground says so (規約 §hover のツールチップ「切ったのは見えている量であって、持ち帰れる量ではない」).
//
// **A cut line is read in the card the row already opens** (the author's, the co-authors'), which is where a name that
// did not fit is shown in full and wraps. Nothing here tries to scroll the tail into view: this is a value on a row,
// not a box to read in.
Item {
    id: line

    /// The value in full — including whatever the width is hiding.
    property string text: ""
    property color color: Theme.textPrimary
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// Spelled in git's own family. A hash is git's spelling and wears the mono one (規約 §git 用語のコード表記);
    /// everything else is the window's. The two the design has, chosen by name rather than passed in — a family that
    /// came from a caller would skip the per-OS fallback Theme resolves (規約 §QML 実装ルール).
    property bool mono: false
    /// What the cut mark is drawn on. The pane's own ground by default — the mark is drawn over the last glyphs it
    /// cuts, and needs something opaque under it.
    property color ground: Theme.bgBase
    /// Whether a press on these words starts a selection here — the ordinary way any text in a window is grabbed, and
    /// Qt's own machinery for it.
    ///
    /// **Off only where a control stands over the words** (the hash plate, the parent link): there the press is the
    /// control's the whole way across and stays that way, and the selection is driven in from beside it instead
    /// (`SweepRoom`, 規約 §右のペインの字は掴める). Everywhere else it is on — a value that answers its own press
    /// needs none of that machinery, and a reader who does hit the words gets the drag Qt gives them.
    property bool grabbable: true

    /// The width ran out and the tail is not on screen. The output side, and what a headless run reads in place of a
    /// mark it cannot see — the same answer `Text.truncated` used to give from the Label this replaced.
    readonly property bool clipped: Math.ceil(ruler.implicitWidth) > line.width + 0.5
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText
    /// Whether the keyboard is here — which is what `Ctrl+C` needs, and the half a selection alone does not say. The
    /// caret arrives with a selection and only with one (`anchorFrom`).
    readonly property alias hasCaret: field.activeFocus
    /// Whether the words answer their own press — the output side of `grabbable`, so a run reads what the field is
    /// rather than what it was asked to be.
    readonly property alias grabs: field.selectByMouse
    /// Where the caret ended up, for a run that has to say why a drag came away with nothing.
    readonly property alias caretPos: field.cursorPosition

    /// Puts the whole value in the selection, for the automation and for `Ctrl+A` (see `selectByKeyboard`).
    ///
    /// **Takes the caret first**, which is what `Ctrl+A` means — the field has to hold the keyboard to be sent one —
    /// and is also what takes the selection off whatever field was holding one. Selecting without it would be a
    /// second way to be selected that no hand can reach, and a run that used it would never see two fields lit at
    /// once (which is the fault this pane shipped with).
    function selectAll() {
        field.forceActiveFocus()
        field.selectAll()
    }
    function deselect() {
        field.deselect()
    }

    // ---- driven from outside ----------------------------------------
    // For the words a control stands over: whoever owns the press hands the gesture down in its own coordinates and
    // the field turns it into characters. The anchor is set once, where the drag began, and every move reads from
    // there — the same shape the field's own drag has.
    /// Where the gesture started, in characters.
    property int grabAnchor: 0
    /// A point in another item's coordinates, brought into this field's own line. **The height is clamped**: a
    /// one-line field asked for a position above or below its box answers about the line rather than about the
    /// column, so a sweep that arrived from over or under the words picked the same character at both ends of its
    /// drag and came away with nothing (2026-08-28 — the fault behind「ドラッグしても青表示にならない」).
    function onLine(item, x, y) {
        const p = line.mapFromItem(item, x, y)
        return Qt.point(p.x, Math.max(0, Math.min(line.height - 1, p.y)))
    }
    function anchorFrom(item, x, y) {
        const p = line.onLine(item, x, y)
        // The caret comes here, which is also what takes the selection off whatever field was holding one: a field
        // drops its own the moment it loses focus (nothing here is persistent — one selection in the window).
        field.forceActiveFocus()
        line.grabAnchor = field.positionAt(p.x, p.y)
        field.select(line.grabAnchor, line.grabAnchor)
    }
    function extendFrom(item, x, y) {
        const p = line.onLine(item, x, y)
        field.select(line.grabAnchor, field.positionAt(p.x, p.y))
    }

    /// The width the value wants — measured off a ruler rather than off the field, because a field asked for its
    /// implicit width while it is being squeezed answers about the box it was given (規約 §QML 実装ルール). Rounded up:
    /// a layout hands an item the whole pixel below a fractional width, and a value asking for 79.28 given 79 cuts
    /// itself against a row meant to hold it (app-ui.md §自然幅の上限は切り上げる).
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: field.implicitHeight
    // Only ever the width, and only when the row could not give it (see the mark below).
    clip: line.clipped

    Text {
        id: ruler
        visible: false
        text: line.text
        font: field.font
        wrapMode: Text.NoWrap
    }

    TextEdit {
        id: field
        width: line.width
        height: line.height
        text: line.text
        color: line.color
        font.family: line.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: line.pixelSize
        font.weight: line.weight
        // One line: the value is a name, a stamp or a hash, and none of them is a paragraph. What does not fit is cut
        // by the item above rather than folded into a second line that would move the row it stands in.
        wrapMode: Text.NoWrap
        // Read, not written: the pane is a view of a commit, and nothing here is edited from it.
        readOnly: true
        selectByMouse: line.grabbable
        // The keyboard comes with a press only where the words answer one. Where they do not, it arrives with the
        // selection a sweep puts in and only with it — a press on a value is no reason to take the arrows away from
        // the graph.
        activeFocusOnPress: line.grabbable
        // `selectByKeyboard` is false by default on a read-only field, and `Ctrl+A` goes with it — the one gesture a
        // reader who wants the whole value reaches for first, and the only way to reach a tail the width has cut. It
        // stays on whether or not the words answer the pointer: where they do not, the caret arrives with the
        // selection a sweep puts in, and `Ctrl+C` has to reach it once it has.
        selectByKeyboard: true
        // **Never `persistentSelection`.** There is one selection in this window, and it belongs to whatever the caret
        // is in: a field that kept its own after the caret left would leave the pane lit in two or three places at
        // once, which is what the pane did the first day it had fields (2026-08-28 ユーザー報告 — three values washed
        // blue in one shot). Keeping it was for the camera, and the camera never needed it: a field that has not taken
        // focus answers `selectedText` after `selectAll()` all the same (実測 qmltestrunner `tst_twofields`).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        // A field of its own width, with no room around it: the row's spacing is the row's.
        padding: 0
        textMargin: 0
    }

    // What the width left behind, on the pane's own ground rather than over the words: the glyphs it cuts through are
    // drawn under this, and a mark read through a name is not a mark. The same shape `CardText` draws for the height
    // it caps.
    Rectangle {
        visible: line.clipped
        color: line.ground
        width: markLabel.implicitWidth
        height: line.height
        x: line.width - width
        Label {
            id: markLabel
            anchors.fill: parent
            verticalAlignment: Text.AlignVCenter
            text: "…"
            color: line.color
            font: field.font
        }
    }
}
