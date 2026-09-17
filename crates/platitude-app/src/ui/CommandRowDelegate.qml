pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One git invocation in the command log: the clock, the command, and how it went. A failure keeps git's own parting
// words under it — the whole point of the panel is that nothing is rephrased.
//
// **The three are columns of one line** (デザイン規約 §git が言ったことを読む場所): a drag over the log picks the line's own
// characters out of them, so each column is a run of the mono font at a place this row can name, and the two gaps
// between them are the tabs a copy comes out with. That is why the command's two halves are a character apart
// — `git` and what follows it are one string, drawn in two colours.
Rectangle {
    id: row

    required property int index
    required property string clock
    required property string args
    required property string full
    required property string state
    required property string result
    required property string duration
    required property string output
    /// Where the reader's own selection falls on this row (`CommandsModel.sel`): a run for each of the three columns
    /// and a last field saying whether the line was taken end to end, which is what brings git's words with it.
    /// Empty on a row the selection does not reach.
    required property string sel

    /// One measured column of the mono font (`CommandsPane.charW`). **Two spacings only**: the space
    /// between `git` and what follows it, and the one between the two words about how the command went. Where the
    /// characters of a column are drawn is that column's own layout to answer (`ruler`).
    required property real charW
    /// Where the selection lives — and where the three columns this row draws are cut from (`CommandsModel`, which
    /// holds the line and hands over a column at a time).
    required property var commandsModel
    /// One of those columns, laid out, asked where a run of it is drawn (`LineRuler`). The same ruler the hand over
    /// the rows reads a press against (`CommandsTextSelect`).
    required property var ruler

    readonly property bool failed: row.state === "failed"
    readonly property bool showsOutput: row.failed && row.output !== ""

    // ---- where this row's columns are drawn ---------------------------------------------------------------------
    /// The near edge of each column and the far edge of the two a press can run off the end of. The hand over the list
    /// reads these to decide which column a press landed in (`CommandsTextSelect`).
    readonly property real clockX: clock_.x
    readonly property real clockEnd: clock_.x + clock_.implicitWidth
    readonly property real cmdX: program.x
    readonly property real cmdEnd: Math.min(argsText.x + argsText.implicitWidth, outcome.x)
    readonly property real outX: outcome.x
    readonly property real outEnd: outcome.x + outcome.width
    /// How tall the command's own line is — below it is the block of words a failure carries.
    readonly property real lineHeight: line.height

    // Width comes from the view; the height grows with the output block.
    height: Theme.rowHeight + (row.showsOutput ? outputText.implicitHeight + 2 * Theme.spaceXs : 0)
    color: row.failed || rowHover.containsMouse ? Theme.bgElevated : "transparent"

    // What went wrong is carried by the edge as well as by the words, so a failure is findable while scrolling past at
    // speed.
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 2 * Theme.borderWidth
        color: Theme.danger
        visible: row.failed
    }

    // The pointer, for the lit ground and the one line the row has no room for. It takes no buttons: the presses over
    // this list belong to the hand that picks its text (`CommandsTextSelect`), which is laid over the whole of it.
    MouseArea {
        id: rowHover
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
        ToolTip.visible: containsMouse && row.state !== "running"
        ToolTip.delay: Metrics.tipDelayMs
        // The reproducible form is long; it is here so the log stays one line per command.
        ToolTip.text: row.full
    }

    // ---- the reader's own selection -----------------------------------------------------------------------------
    /// The three runs and the flag, taken apart once: `<clock>|<command>|<outcome>|<whole>`.
    readonly property var picked: row.sel === "" ? [] : row.sel.split("|")
    readonly property bool tookLine: row.picked.length === 4 && row.picked[3] === "1"
    /// The three columns as `CommandsModel` numbers them, in the order the runs above come in — `AT_CLOCK`,
    /// `AT_CMD`, `AT_OUT` of `models/commands/selection.rs`, whose odd numbers are the two gaps no run is drawn in.
    /// Written out: `CommandsTextSelect` names the same five for the hand, and a renumbering has
    /// to find both.
    readonly property var washCols: [0, 2, 4]

    /// How far into each of the three columns the wash reaches, in that column's own coordinates — `{ x, w }`, or
    /// null where the column holds none of the selection. Taken from the column's own layout: a place of it is drawn
    /// where the row drew it, and no count of characters finds that (`LineRuler`).
    ///
    /// **Pushed** (the same rule `DiffRowDelegate.emphRects` is written under). Asking the ruler means
    /// putting the column on it, and a binding that writes while it is being evaluated is a binding loop — Qt says so
    /// by name and then holds whatever it had, which is a wash left on the row before it. The column's coordinates
    /// are its own, so the three x the columns stand at stay bindings and this is asked again only when what a
    /// column holds changes.
    property var washSpans: [null, null, null]
    function settleWash() {
        // **`sel` is read again here.** A change handler runs *before* the bindings
        // that derive from the property it is about, so `picked` inside `onSelChanged` is still the row's previous
        // selection — measured on a throwaway `qmltestrunner` scene, and seen as a log that held a selection and
        // wore no wash at all.
        const runs = row.sel === "" ? [] : row.sel.split("|")
        const out = [null, null, null]
        for (let at = 0; at < 3; at++) {
            if (runs[at] === undefined || runs[at] === "")
                continue
            const text = row.commandsModel.columnText(row.index, row.washCols[at])
            out[at] = row.bounds(row.ruler.rectsOf(text, false, runs[at]))
        }
        row.washSpans = out
    }
    /// One rectangle over every piece of a run. A column that reads one way comes back as one piece and this is that
    /// piece; a column carrying a word that reads the other way comes back as several, and the wash over them is
    /// their bounding box — this log's wash is one rectangle by design (see `washEdge`), so it cannot draw the
    /// pieces apart the way the diff does.
    function bounds(rects) {
        if (rects.length === 0)
            return null
        let from = rects[0].x
        let to = rects[0].x + rects[0].w
        for (const rect of rects) {
            from = Math.min(from, rect.x)
            to = Math.max(to, rect.x + rect.w)
        }
        return { x: from, w: to - from }
    }
    // What a column holds, and what of it the selection covers. A delegate handed to another row (`reuseItems`)
    // arrives through these same handlers.
    onSelChanged: row.settleWash()
    onClockChanged: row.settleWash()
    onArgsChanged: row.settleWash()
    onResultChanged: row.settleWash()
    onDurationChanged: row.settleWash()
    Component.onCompleted: row.settleWash()

    /// The wash is one rectangle from the first column the selection reaches into to the last, which is what puts it
    /// over the gaps as well — a tab is a character of the line, and a terminal washes it like one.
    function washEdge(wantFirst) {
        const bases = [row.clockX, row.cmdX, row.outX]
        for (let i = 0; i < 3; i++) {
            const at = wantFirst ? i : 2 - i
            const span = row.washSpans[at]
            if (!span)
                continue
            return bases[at] + (wantFirst ? span.x : span.x + span.w)
        }
        return 0
    }
    // Both read only `washSpans` — a row wearing no span is a row with no wash, and
    // asking `sel` here again would be the second reader of a property this one already answers for.
    readonly property real washX: row.washEdge(true)
    // Clamped to the row: a command too long for its column is drawn elided, and a wash running on past the last
    // character anybody can see would be washing the panel.
    readonly property real washRight: Math.min(row.washEdge(false), row.width - Theme.spaceMd)

    Rectangle {
        x: row.washX
        y: 0
        width: Math.max(0, row.washRight - row.washX)
        height: line.height
        color: Theme.bgSelected
    }
    // git's words come with a command line taken whole, so they are washed with it (デザイン規約 §git が言ったことを読む場所).
    Rectangle {
        x: outputText.x
        y: outputText.y
        width: outputText.implicitWidth
        height: outputText.implicitHeight
        visible: row.showsOutput && row.tookLine
        color: Theme.bgSelected
    }

    Item {
        id: line
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.rowHeight

        Label {
            id: clock_
            x: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            text: row.clock
            // Every column of this log is the characters it holds, named on all five of them: the ruler that
            // places the wash and reads the press is set in this format (`row.ruler`), and Qt's `AutoText`
            // guess is a property of the text — a row whose format came from what a command happened to
            // carry would be drawn as a line the ruler has never seen.
            textFormat: Text.PlainText
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // The program never varies, so it is drawn: what changes from row to row is the part after it.
        Label {
            id: program
            x: row.clockEnd + Theme.spaceLg
            anchors.verticalCenter: parent.verticalCenter
            text: "git"
            textFormat: Text.PlainText
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Label {
            id: argsText
            // One character along from `git`: the two are one string with one space in it —
            // which is the string the ruler is handed for this column (`CommandsModel.column`), so a selection that
            // runs through them lands on the same pixels the copy names. **Its width is
            // written** — an anchor on the right decides the x as well, and this
            // one has to begin where the command's own column does (measured, the args went to the far edge).
            x: program.x + 4 * row.charW
            width: Math.max(0, outcome.x - Theme.spaceMd - x)
            anchors.verticalCenter: parent.verticalCenter
            text: row.args
            textFormat: Text.PlainText
            elide: Text.ElideRight
            color: Theme.textPrimary
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        Row {
            id: outcome
            anchors.right: parent.right
            anchors.rightMargin: Theme.spaceMd
            anchors.verticalCenter: parent.verticalCenter
            // The same one space the copy puts between them: two words about how it went.
            spacing: row.charW
            Label {
                text: row.result
                visible: text !== ""
                textFormat: Text.PlainText
                color: Theme.danger
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
            Label {
                // Exit 0 says nothing that the absence of a complaint has not already said, so only the time it took is
                // kept.
                text: row.state === "running" ? qsTr("running…") : row.duration
                textFormat: Text.PlainText
                color: row.state === "running" ? Theme.accent : Theme.textMuted
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // Indented to the command it belongs to. Positioned: the column it lines up with lives inside
    // `line`, which makes it neither parent nor sibling of this.
    Label {
        id: outputText
        visible: row.showsOutput
        x: program.x
        y: line.height + Theme.spaceXs
        width: row.width - program.x - Theme.spaceMd
        text: row.output
        wrapMode: Text.Wrap
        color: Theme.textSecondary
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontSm
        lineHeight: Theme.fontSmLine
        lineHeightMode: Text.FixedHeight
    }
}
