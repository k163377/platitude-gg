import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line a pane draws to be read, in a field the reader can take away with them: pressed, dragged over and copied
// like any other text in this window (デザイン規約 §右のペインの字は掴める).
//
// The line half of the pair `CardText` is the other half of. That one wraps and caps its height for a card; this one
// is a single line standing in a row, so **what it cuts is its width** — and it cuts the same way, by clipping:
// `TextEdit` has no `elide`, and a field whose text had been elided into it would hand the reader
// `Yuki Tana…` when they dragged over it. The whole value stays in the field, what does not fit is clipped, and the
// mark on the pane's own ground says so (規約 §hover のツールチップ「切ったのは見えている量であって、持ち帰れる量ではない」).
//
// **A cut line is read in the card the row already opens** (the author's, the co-authors'), which is where a name
// that did not fit is shown in full and wraps. Nothing here tries to scroll the tail into view: this is a value on
// a row.
Item {
    id: line

    /// The value in full — including whatever the width is hiding.
    property string text: ""
    /// The same line spelled as the markup rich text reads, where one word in it wears a colour of its own — a ref
    /// name inside the sentence (`Words.nameInSentence`). Empty everywhere else, and the field takes `text` instead.
    /// **`text` stays the plain sentence either way**, since that is what a caller measuring or reading the line back
    /// is handed. The ruler measures whichever of the two is drawn, or a width would be bid for the tags as well.
    ///
    /// **Only the tail cut takes markup.** A middle cut spells the head onto the mark's own ground out of `text`, and
    /// the plain head of a line whose drawn half is coloured would come away in the wrong colour.
    property string markup: ""
    property color color: Theme.textPrimary
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// Spelled in git's own family. A hash is git's spelling and wears the mono one (規約 §git 用語のコード表記);
    /// everything else is the window's. The two the design has, chosen by name — a family that came from a caller
    /// would skip the per-OS fallback Theme resolves (規約 §QML 実装ルール).
    property bool mono: false
    /// What is added to the space between words. **Negative where a command and its flag are one thing said**
    /// (`CodeChip` — a mono space is wider than the air a chip's ground keeps at its own ends, and left alone the
    /// two drift apart until the chip reads as two words on one ground). Zero everywhere else, which is every value
    /// that is one word or a sentence.
    property real wordSpacing: 0
    /// What the cut mark is drawn on. The pane's own ground by default — the mark is drawn over the last glyphs it
    /// cuts, and needs something opaque under it.
    property color ground: Theme.bgBase
    /// And whatever is laid over that ground where this line stands — a row's hover wash, and nothing else so far.
    /// **The mark has to wear the same two layers the row does**: it is drawn on an opaque patch, so a patch painted
    /// in the resting colour under a row wearing an overlay reads as a box around the `…`. Transparent wherever the
    /// ground does not move.
    property color groundOverlay: "transparent"
    /// Whether a press on these words starts a selection here — the ordinary way any text in a window is grabbed, and
    /// Qt's own machinery for it.
    ///
    /// **Off only where a control stands over the words** (the hash plate, the parent link): there the press is the
    /// control's the whole way across and stays that way, and the selection is driven in from beside it instead
    /// (`SweepRoom`, 規約 §右のペインの字は掴める). Everywhere else it is on — a value that answers its own press
    /// needs none of that machinery, and a reader who does hit the words gets the drag Qt gives them.
    property bool grabbable: true
    /// Which end the width takes when the value is wider than the row it stands in. `"end"` drops the tail, which is
    /// right for a name or a stamp — the head is what tells two of them apart. `"start"` drops the head and keeps the
    /// tail on screen. `"middle"` drops the middle, which is right for a path told apart by both of its ends: the
    /// leaf names the file, and the first folders say which tree it is in (`DiffPaneHeader`). The same choice
    /// `CutName` spells with its own `cutAt`, said here for a field.
    ///
    /// **The middle cut is drawn.** The field is held against the far edge exactly as `"start"` holds it, and the
    /// head is painted onto the mark's own ground beside the `…` — so it is a mark that happens to spell the head.
    /// That is what keeps the copy whole where three Texts would hand over a value with its middle missing
    /// (規約 §右のペインの字は掴める).
    ///
    /// **The whole value is in the field either way** — only what is on screen moves. That is the difference between
    /// this and an elide, and it is the whole reason these are fields: a reader who drags gets the whole value
    /// (規約 §右のペインの字は掴める).
    property string cutAt: "end"

    /// The width ran out and the tail is not on screen. The output side, and what a headless run reads in place of a
    /// mark it cannot see — the answer `Text.truncated` gives on a Label.
    readonly property bool clipped: Math.ceil(ruler.implicitWidth) > line.width + 0.5
    /// The half the width took came off the head, so the words are held against the far edge and the mark stands at
    /// the near one. Only ever true while there is something to cut. A middle cut is held the same way — what the
    /// mark spells is the only difference between the two.
    readonly property bool cutsHead: (line.cutAt === "start" || line.cutAt === "middle") && line.clipped
    /// What the mark spells: `…` on its own, or the head and then the `…` where the cut is taken out of the middle.
    /// The split is the one `Text.ElideMiddle` makes at this width, read off a ruler — the field below is untouched,
    /// so this is what the band paints over the value.
    readonly property string markText: (line.cutAt === "middle" && line.clipped
                                        ? line.headOf(line.text, middleRuler.elidedText) : "") + "…"
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText
    /// This is a value a sweep from the air around it can land on. The pane's own row names its values one by one
    /// (`SweepRoom`), but a card holding one of these — the credit line beside a hover card's date — is swept by a pad
    /// that walks the card looking for exactly this (`SweepPad`, 規約 §hover のツールチップ).
    readonly property bool sweepable: true
    /// Whether the keyboard is here — which is what `Ctrl+C` needs, and the half a selection alone does not say. The
    /// caret arrives with a selection and only with one (`anchorFrom`).
    readonly property alias hasCaret: field.activeFocus
    /// Whether the words answer their own press — the output side of `grabbable`, so a run reads what the field
    /// actually is.
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
    /// The head of an elided value. **Looked for** — `…` is a character a value may hold of its own — so the answer
    /// is the first mark that leaves a real head and a real tail of the value behind it.
    function headOf(whole, elided) {
        for (let i = elided.indexOf("…"); i >= 0; i = elided.indexOf("…", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return elided.substring(0, i)
        }
        return ""
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
    /// drag and came away with nothing (observed).
    ///
    /// **The answer is in the field's coordinates** — they are the same box as this item's only while the head is on
    /// screen. A field cutting its head hangs off the near edge (`field.x` is negative), and a column read at this
    /// item's x would be the column that many pixels further into the value than the one under the pointer.
    function onLine(item, x, y) {
        const p = line.mapFromItem(item, x, y)
        return Qt.point(p.x - field.x, Math.max(0, Math.min(line.height - 1, p.y)))
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

    /// The width the value wants — measured off a ruler, because a field asked for its implicit width while it is
    /// being squeezed answers about the box it was given (規約 §QML 実装ルール). Rounded up: a layout hands an item the
    /// whole pixel below a fractional width, and a value asking for 79.28 given 79 cuts itself against a row meant
    /// to hold it (app-ui.md §自然幅の上限は切り上げる).
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: field.implicitHeight
    // Only ever the width, and only when the row could not give it (see the mark below).
    clip: line.clipped

    Text {
        id: ruler
        visible: false
        text: line.markup !== "" ? line.markup : line.text
        textFormat: line.markup !== "" ? Text.RichText : Text.PlainText
        font: field.font
        wrapMode: Text.NoWrap
    }
    /// The second ruler, and only ever asked about a middle cut: where the elide would put its mark at this width.
    /// Given no text in the other two modes so a value that is never cut in the middle is never laid out twice.
    TextMetrics {
        id: middleRuler
        font: field.font
        elide: Text.ElideMiddle
        elideWidth: line.width
        text: line.cutAt === "middle" ? line.text : ""
    }

    TextEdit {
        id: field
        // As wide as the words when the head is the half being dropped, and held against the far edge so the tail is
        // what the row shows. The item above clips what hangs off (`clip: line.clipped`), and the whole value stays
        // in here to be selected — which is the one thing an elide could not do.
        width: line.cutsHead ? Math.max(line.width, Math.ceil(ruler.implicitWidth)) : line.width
        x: line.cutsHead ? line.width - field.width : 0
        height: line.height
        text: line.markup !== "" ? line.markup : line.text
        // Pinned. What these fields carry is git's own words as often as this application's, and `AutoText` decides
        // by guessing whether a string looks like markup — a branch called `<b>` would vanish. The one line that is
        // markup says so because a caller built it and escaped everything that went into it.
        textFormat: line.markup !== "" ? TextEdit.RichText : TextEdit.PlainText
        color: line.color
        font.family: line.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: line.pixelSize
        font.weight: line.weight
        font.wordSpacing: line.wordSpacing
        // One line: the value is a name, a stamp or a hash, and none of them is a paragraph. What does not fit is cut
        // by the item above; a second line would move the row it stands in.
        wrapMode: Text.NoWrap
        // Read only: the pane is a view of a commit, and nothing here is edited from it.
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
        // **There is one selection in this window**, and it belongs to whatever the caret is in: a field with
        // `persistentSelection` keeping its own after the caret left would leave the pane lit in two or three places
        // at once, which is what the pane did the first day it had fields (observed — three values washed blue in
        // one shot). Keeping it was for the camera, and the camera never needed it: a field that has not taken focus
        // answers `selectedText` after `selectAll()` all the same (measured, qmltestrunner `tst_twofields`).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        // A field of its own width, with no room around it: the row's spacing is the row's.
        padding: 0
        textMargin: 0
    }

    // What the width left behind, on the pane's own ground: the glyphs it cuts through are drawn under this, and a
    // mark read through a name is not a mark. The same shape `CardText` draws for the height it caps. It stands at
    // the end the cut was taken from, so the mark is always on the side the value runs off.
    Rectangle {
        visible: line.clipped
        color: line.ground
        width: markLabel.implicitWidth
        height: line.height
        x: line.cutsHead ? 0 : line.width - width
        // The second of the row's two layers, in the order the row paints them.
        Rectangle {
            anchors.fill: parent
            color: line.groundOverlay
        }
        Label {
            id: markLabel
            anchors.fill: parent
            verticalAlignment: Text.AlignVCenter
            text: line.markText
            color: line.color
            font: field.font
        }
    }
}
