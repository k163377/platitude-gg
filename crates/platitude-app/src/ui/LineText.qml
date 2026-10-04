import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One line a pane draws to be read, in a field the reader can take away (デザイン規約 §右のペインの字は掴める) — the
// one-line half of `CardText`. It cuts its width by clipping, not eliding: `TextEdit` has no `elide`, and an elided
// field would hand over `Yuki Tana…`. The whole value stays in the field and a mark on the ground says it was cut
// (デザイン規約 §hover のツールチップ「切ったのは見えている量であって、持ち帰れる量ではない」); a cut line is read in full
// in the card its row opens.
Item {
    id: line

    /// The value in full — including whatever the width is hiding.
    property string text: ""
    /// The line as rich-text markup where one word wears its own colour (`Words.nameInSentence`); empty otherwise.
    /// `text` stays the plain sentence either way — callers measure and read it back. Only the tail cut takes markup:
    /// a middle cut spells the head from `text`, which would come out in the wrong colour.
    property string markup: ""
    property color color: Theme.textPrimary
    property real pixelSize: Theme.fontMd
    property int weight: Font.Normal
    /// The mono family (a hash — デザイン規約 §git 用語のコード表記) instead of the UI one. A bool, not a family: a
    /// caller's family would skip Theme's per-OS fallback.
    property bool mono: false
    /// Added to the space between words. Negative only in `CodeChip`, where a mono space would split a command and its
    /// flag into two words on one ground.
    property real wordSpacing: 0
    /// The opaque ground the cut mark is drawn on, over the glyphs it cuts.
    property color ground: Theme.bgBase
    /// Whatever is laid over `ground` where this line stands (a row's hover wash). The mark's patch wears both
    /// layers, or it reads as a box around the `…`.
    property color groundOverlay: "transparent"
    /// A press on the words starts a selection here. Off only where a control stands over the words (the hash plate,
    /// the parent link); the selection is then driven in from beside (`SweepRoom`, `anchorFrom`).
    property bool grabbable: true
    /// Which end goes when the value is too wide: `"end"` (names, stamps — the head tells them apart), `"start"`, or
    /// `"middle"` (paths told apart by both ends — `DiffPaneHeader`); the same choice as `CutName.cutAt`. A middle cut
    /// is held like `"start"`, with the head painted onto the mark's ground beside the `…`, so the copy stays whole.
    property string cutAt: "end"

    /// The value is wider than the line — what `Text.truncated` says on a Label, and what a headless run reads.
    readonly property bool clipped: Math.ceil(ruler.implicitWidth) > line.width + 0.5
    /// The cut is taken from the head (or the middle): the words are held against the far edge, the mark at the near
    /// one.
    readonly property bool cutsHead: (line.cutAt === "start" || line.cutAt === "middle") && line.clipped
    /// `…`, or for a middle cut the head and `…`, split where `Text.ElideMiddle` would at this width.
    readonly property string markText: (line.cutAt === "middle" && line.clipped
                                        ? line.headOf(line.text, middleRuler.elidedText) : "") + "…"
    /// Where an end cut's mark starts: the far edge of the last character standing whole before it. Cut by the pixel,
    /// a character stands half under the mark and reads as another (`12` as `1` and a stroke). One binding over what
    /// moves the answer, since `positionAt` is a call (rules/app-ui.md「メソッドはバインディングが依存を取らない」).
    readonly property real markAt: {
        const room = Math.max(0, line.width - markLabel.implicitWidth)
        if (!line.clipped || line.cutsHead || field.text === "" || field.contentWidth <= 0)
            return room
        // Asked on the line's first row: below the line a field answers with its last character.
        let at = field.positionAt(room, 0)
        const edge = field.positionToRectangle(at).x
        // `positionAt` answers the nearer edge, which can be the one past the room. Under a pixel past it is the
        // character's own air and it stands: a holder that sized the line for a count of characters rounds its sum
        // apart from the mark's width here, and comes out a fraction short of the last one (`HashPlate.hashWidth`
        // under a face of fractional advances).
        if (edge < room + 1 || at <= 0)
            return Math.min(edge, room)
        // Back to the edge before it — past every position inside a character of several code units, each of which
        // answers that character's far edge.
        let back = edge
        while (at > 0 && back >= edge)
            back = field.positionToRectangle(--at).x
        return Math.min(back, room)
    }
    /// How far a head cut draws the words back from the far edge, so the first character past the mark stands whole:
    /// held hard against that edge, whatever character the mark's end falls in shows its back half. The same binding
    /// as `markAt`, read off where the words would stand unmoved (not `field.x`, which this feeds).
    readonly property real tailSlack: {
        if (!line.cutsHead || field.text === "" || field.contentWidth <= 0)
            return 0
        const from = markLabel.implicitWidth - (line.width - field.width)
        const at = field.positionAt(from, 0)
        const edge = field.positionToRectangle(at).x
        // `positionAt` answers the nearer edge, which can be the one the mark still covers — by under a pixel, the
        // character's own air, and it stands (as `markAt`).
        const next = edge > from - 1 || at >= field.length ? edge : field.positionToRectangle(at + 1).x
        return Math.max(0, next - from)
    }
    /// How wide the mark is: what a holder that cuts the value back to a word of its own choosing leaves past that word
    /// (`CommitAuthorRow.dateDayRoom`).
    readonly property alias markWidth: markLabel.implicitWidth
    /// What is selected right now, for a run that has no pointer to drag with.
    readonly property alias selected: field.selectedText
    /// A value a `SweepPad` walking a card can land a sweep on.
    readonly property bool sweepable: true
    /// The keyboard is here, which `Ctrl+C` needs and a selection alone does not say.
    readonly property alias hasCaret: field.activeFocus
    /// The output side of `grabbable`, for runs.
    readonly property alias grabs: field.selectByMouse
    /// Where the caret ended up, for a run that has to say why a drag came away with nothing.
    readonly property alias caretPos: field.cursorPosition

    /// Selects the whole value, for automation and `Ctrl+A`; takes the caret first, as `Ctrl+A` does
    /// (rules-refs/app-ui.md「`persistentSelection` を使わない」).
    function selectAll() {
        field.forceActiveFocus()
        field.selectAll()
    }
    function deselect() {
        field.deselect()
    }
    /// The head of an elided value — searched, since the value itself may hold a `…`.
    function headOf(whole, elided) {
        for (let i = elided.indexOf("…"); i >= 0; i = elided.indexOf("…", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return elided.substring(0, i)
        }
        return ""
    }

    // ---- driven from outside ----------------------------------------
    // For words a control stands over: the press's owner hands the gesture down in its own coordinates.
    /// Where the gesture started, in characters.
    property int grabAnchor: 0
    /// A point in `item`'s coordinates, in the field's own. The height is clamped: a one-line field asked above or
    /// below its box answers with the line's ends. The x is the field's, not this item's — a field cutting its head
    /// hangs off the near edge (negative `field.x`).
    function onLine(item, x, y) {
        const p = line.mapFromItem(item, x, y)
        return Qt.point(p.x - field.x, Math.max(0, Math.min(line.height - 1, p.y - field.y)))
    }
    function anchorFrom(item, x, y) {
        const p = line.onLine(item, x, y)
        // Taking the caret clears the selection of whatever field held it (no `persistentSelection`).
        field.forceActiveFocus()
        line.grabAnchor = field.positionAt(p.x, p.y)
        field.select(line.grabAnchor, line.grabAnchor)
    }
    function extendFrom(item, x, y) {
        const p = line.onLine(item, x, y)
        field.select(line.grabAnchor, field.positionAt(p.x, p.y))
    }

    /// Measured off `ruler`, since a squeezed field answers with its box
    /// (rules-refs/app-ui.md「自然幅は隠した `Text` の ruler で測る」), and rounded up, since a layout hands out the
    /// whole pixel below (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
    implicitWidth: Math.ceil(ruler.implicitWidth)
    implicitHeight: field.implicitHeight
    // The field's own, for a holder that sets words of two faces on one line (`CommitAuthorRow.firstBase` /
    // `secondBase`).
    baselineOffset: field.y + field.baselineOffset
    clip: line.clipped

    Text {
        id: ruler
        visible: false
        text: line.markup !== "" ? line.markup : line.text
        textFormat: line.markup !== "" ? Text.RichText : Text.PlainText
        font: field.font
        wrapMode: Text.NoWrap
    }
    /// Where a middle elide would split at this width; no text in the other modes, so nothing is laid out twice.
    TextMetrics {
        id: middleRuler
        font: field.font
        elide: Text.ElideMiddle
        elideWidth: line.width
        text: line.cutAt === "middle" ? line.text : ""
    }

    TextEdit {
        id: field
        // Cutting the head: as wide as the words, held against the far edge less what stands the first character past
        // the mark whole (`tailSlack`); the item clips what hangs off.
        width: line.cutsHead ? Math.max(line.width, Math.ceil(ruler.implicitWidth)) : line.width
        x: line.cutsHead ? line.width - field.width - line.tailSlack : 0
        // The line in the middle of the box, where a `Label` of the same face stands. A field files the face's leading
        // under its line (Hiragino Sans: 7px at `fontMd`), so beside a label the words stand half of it high, and an
        // underline at the box's foot hangs all of it under them. Whole pixels; nothing where the face has no leading,
        // and nothing for a `markup` line, whose ruler is rich text and carries the leading as the field does.
        y: Math.floor((field.implicitHeight - ruler.implicitHeight) / 2)
        height: line.height
        text: line.markup !== "" ? line.markup : line.text
        // Pinned: `AutoText` guesses, and a branch called `<b>` would vanish. `markup` is escaped by the caller that
        // built it.
        textFormat: line.markup !== "" ? TextEdit.RichText : TextEdit.PlainText
        color: line.color
        font.family: line.mono ? Theme.monoFamily : Theme.uiFamily
        font.pixelSize: line.pixelSize
        font.weight: line.weight
        font.wordSpacing: line.wordSpacing
        wrapMode: Text.NoWrap
        readOnly: true
        selectByMouse: line.grabbable
        // Where the words take no press, the caret comes only with a sweep's selection — a press on a value is no
        // reason to take the arrows from the graph.
        activeFocusOnPress: line.grabbable
        // Off by default on a read-only field, taking `Ctrl+A` with it — the only way to reach a tail the width cut.
        // On even without `grabbable`: `Ctrl+C` has to reach a swept selection.
        selectByKeyboard: true
        // No `persistentSelection` — one selection in the window, camera included
        // (rules-refs/app-ui.md「`persistentSelection` を使わない」).
        selectionColor: Theme.accent
        selectedTextColor: Theme.textOnAccent
        padding: 0
        textMargin: 0
    }

    // The cut mark, on an opaque patch over the glyphs it cuts, at the end the cut was taken from (the shape `CardText`
    // draws for its height cap). An end cut's patch runs from the last whole character to the line's end (`markAt`).
    Rectangle {
        visible: line.clipped
        color: line.ground
        width: line.cutsHead ? markLabel.implicitWidth : line.width - line.markAt
        height: line.height
        x: line.cutsHead ? 0 : line.markAt
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
