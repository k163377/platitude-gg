import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A name in a column narrower than itself, cut so the column keeps both of its edges
// (規約 §寸法「列に並ぶ名前の省略は、余りを切れ目へ入れる」). `Text.ElideMiddle` alone draws what fits from the left, so the
// unused width piles up at the right edge, differently on every row; and it can stop up to two characters short, as
// it gives one back to each end at a time. So the cut is filled out here a character at a time, and the leftover
// (under one character) goes into the cut, beside the mark.
//
// A sentence (`cutAt: "end"`, a commit's subject) keeps only its head, filled the same way, with the mark pinned to
// the column's right edge (規約 §寸法「文の省略は末尾で、印は列の右辺に貼る」).
//
// The name in full stays the row's hover (規約 §hover のツールチップ).
Item {
    id: cut

    /// The name in full.
    property string text: ""
    property color color: Theme.textPrimary
    property int weight: Font.Normal
    property real pixelSize: Theme.fontMd
    /// Letter spacing (the tab strip's tracking, `TabMetrics.titleTracking`). Set it here, not on the labels: the
    /// rulers measure through this font, so spacing set past them draws wider than it measured.
    property real letterSpacing: 0
    /// Where the mark falls: `middle` for a name (told apart by both of its ends), `end` for a sentence (read from the
    /// left, and nothing after the cut is worth keeping).
    property string cutAt: "middle"
    /// Whether the pieces are drawn. False where something else draws the same name here (`NameCell.whole`); the cut
    /// is still worked out, so the column keeps its width and `cutting` keeps answering.
    property bool inked: true

    /// The two halves that are drawn, and whether the mark stands between them — `cutting` is how a caller asks
    /// whether the name was cut (`Text.truncated` means nothing here). Written by `relayout()`: a binding that writes
    /// to the ruler it reads would retrigger itself forever.
    property string headText: ""
    property string tailText: ""
    property bool cutting: false
    /// Where the ink ends — the whole column once there is a cut in the name, the name's own width while there is not.
    /// What a mark on the name's shoulder is placed against.
    readonly property real inkWidth: cut.cutting ? cut.width : headLabel.implicitWidth

    function relayout() {
        const whole = cut.text
        // The whole name goes onto `measure` here, before any branch, so `implicitWidth` never lags the text (a
        // binding would answer `textChanged` in an unwritten order). The fit test below reuses it, and an unchanged
        // text (a column drag) costs no layout.
        measure.font = headLabel.font
        measure.text = whole
        // Read after that write: a caller sizing the box off `implicitWidth` (`TabItemDelegate`) re-enters this
        // function from those two lines, and a width read earlier would lay a cut for a column that is gone back over
        // the nested answer (rules-refs/app-ui.md「`CutName` の箱は定規へ書いた後で読む」).
        const box = cut.width
        // No name or no column yet (a delegate is built before layout): the anchored labels would otherwise draw the
        // whole name over the row's neighbours.
        if (whole === "" || box <= 0) {
            cut.headText = ""
            cut.tailText = ""
            cut.cutting = false
            return
        }
        if (measure.advanceWidth <= box) {
            cut.headText = whole
            cut.tailText = ""
            cut.cutting = false
            return
        }
        // Start from the elide's answer: it fits, and it breaks only where text may break (never inside a surrogate
        // pair or a cluster).
        probe.elideWidth = box
        probe.text = whole
        const elided = probe.elidedText
        const at = cut.cutPoint(whole, elided)
        const toEnd = cut.cutAt === "end"
        let head = at >= 0 ? at : 0
        let tail = !toEnd && at >= 0 ? elided.length - at - 1 : 0
        // Fill the column, the tail first: a generated name is told apart by its end
        // (規約 §git 用語のコード表記「ref の省略は中央」). A cut at the end grows only the head.
        const room = box - cut.inkOf("…")
        let headInk = cut.inkOf(whole.substring(0, head))
        let tailInk = cut.inkOf(whole.substring(whole.length - tail))
        for (;;) {
            let grew = false
            const back = toEnd ? 0 : cut.stepBack(whole, whole.length - tail)
            if (back > 0 && head + tail + back <= whole.length) {
                const ink = cut.inkOf(whole.substring(whole.length - tail - back))
                if (headInk + ink <= room) {
                    tail += back
                    tailInk = ink
                    grew = true
                }
            }
            const on = cut.stepOn(whole, head)
            if (on > 0 && head + tail + on <= whole.length) {
                const ink = cut.inkOf(whole.substring(0, head + on))
                if (ink + tailInk <= room) {
                    head += on
                    headInk = ink
                    grew = true
                }
            }
            if (!grew)
                break
        }
        cut.headText = whole.substring(0, head)
        cut.tailText = tail > 0 ? whole.substring(whole.length - tail) : ""
        cut.cutting = true
    }

    /// The advance of a piece of the name. Only `advanceWidth` equals the labels' `implicitWidth`; TextMetrics' other
    /// measures do not (`BandWidest`).
    function inkOf(part) {
        probe.text = part
        return probe.advanceWidth
    }

    /// One character on from `at`, and one back — in code points so a surrogate pair is never halved, and past
    /// anything with no advance of its own, so a combining accent stays with its letter.
    function stepOn(s, at) {
        let n = 0
        while (at + n < s.length) {
            const c = s.charCodeAt(at + n)
            n += c >= 0xD800 && c <= 0xDBFF && at + n + 1 < s.length ? 2 : 1
            if (cut.inkOf(s.substring(at, at + n)) > 0)
                break
        }
        return n
    }
    function stepBack(s, at) {
        let n = 0
        while (at - n > 0) {
            const c = s.charCodeAt(at - n - 1)
            n += c >= 0xDC00 && c <= 0xDFFF && at - n - 1 > 0 ? 2 : 1
            if (cut.inkOf(s.substring(at - n, at)) > 0)
                break
        }
        return n
    }
    /// Where the elide put its mark — searched for, since a name may hold its own `…` (a stash is named by its
    /// message): the first mark with a real head and tail of the name around it.
    function cutPoint(whole, elided) {
        for (let i = elided.indexOf("…"); i >= 0; i = elided.indexOf("…", i + 1)) {
            if (whole.startsWith(elided.substring(0, i)) && whole.endsWith(elided.substring(i + 1)))
                return i
        }
        return -1
    }

    // Rounded up once here for every caller (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
    implicitWidth: Math.ceil(measure.advanceWidth)
    implicitHeight: markLabel.implicitHeight
    onTextChanged: cut.relayout()
    onWidthChanged: cut.relayout()
    Component.onCompleted: cut.relayout()

    /// Two rulers, both written by `relayout()`: `measure` holds the whole name (the fit test and `implicitWidth`),
    /// `probe` the pieces — set a dozen times per relayout, so nothing binds to it.
    TextMetrics {
        id: measure
    }
    TextMetrics {
        id: probe
        font: headLabel.font
        elide: cut.cutAt === "end" ? Text.ElideRight : Text.ElideMiddle
    }

    // The three pieces, centred vertically: callers pass boxes taller than the line
    // (rules-refs/app-ui.md「`CutName` は渡された箱の縦中央に置く」).
    Label {
        id: headLabel
        visible: cut.inked
        anchors.fill: parent
        verticalAlignment: Text.AlignVCenter
        text: cut.headText
        color: cut.color
        font.pixelSize: cut.pixelSize
        font.weight: cut.weight
        font.letterSpacing: cut.letterSpacing
        // The family arrives with the window and the size with the caller; either changes what fits.
        onFontChanged: cut.relayout()
    }
    Label {
        id: tailLabel
        anchors.fill: parent
        horizontalAlignment: Text.AlignRight
        verticalAlignment: Text.AlignVCenter
        visible: cut.inked && cut.tailText !== ""
        text: cut.tailText
        color: cut.color
        font: headLabel.font
    }
    Label {
        id: markLabel
        y: 0
        height: cut.height
        visible: cut.inked && cut.cutting
        verticalAlignment: Text.AlignVCenter
        text: "…"
        color: cut.color
        font: headLabel.font
        // In the middle of the cut, the sub-character leftover split either side of the mark, and never left of the
        // head's end (a hair-wide measure would draw the mark over it). A cut at the end pins it to the right edge.
        x: cut.cutAt === "end"
           ? cut.width - markLabel.implicitWidth
           : Math.max(headLabel.implicitWidth,
                      (headLabel.implicitWidth + cut.width - tailLabel.implicitWidth - markLabel.implicitWidth) / 2)
    }
}
