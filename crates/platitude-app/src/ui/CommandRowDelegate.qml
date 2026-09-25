pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One git invocation in the command log: the clock, the command, and how it went, with git's own words under a
// failure. The three are tab-separated columns of one line (デザイン規約 §git が言ったことを読む場所); `git` and its
// arguments are one column, drawn in two colours a character apart.
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
    /// Where the reader's selection falls on this row (`CommandsModel.sel` — `{clock, command, outcome, whole}`): the
    /// runs of each column, and whether the line was taken whole, which brings git's words with it. Nothing on a row
    /// the selection does not reach.
    required property var sel

    /// One measured column of the mono font (`CommandsPane.charW`), used for two spacings only: after `git`, and
    /// between the two words about how it went. Where a column's characters fall is the `ruler`'s to answer.
    required property real charW
    /// Where the selection lives, and where this row's three columns are cut from.
    required property var commandsModel
    /// Asked where a run of a column is drawn (`LineRuler`); the same ruler `CommandsTextSelect` reads a press against.
    required property var ruler

    readonly property bool failed: row.state === "failed"
    readonly property bool showsOutput: row.failed && row.output !== ""

    // ---- where this row's columns are drawn ---------------------------------------------------------------------
    /// Column edges, which `CommandsTextSelect` reads to decide which column a press landed in.
    readonly property real clockX: clock_.x
    readonly property real clockEnd: clock_.x + clock_.implicitWidth
    readonly property real cmdX: program.x
    readonly property real cmdEnd: Math.min(argsText.x + argsText.implicitWidth, outcome.x)
    readonly property real outX: outcome.x
    readonly property real outEnd: outcome.x + outcome.width
    /// How tall the command's own line is — below it is the block of words a failure carries.
    readonly property real lineHeight: line.height

    height: Theme.rowHeight + (row.showsOutput ? outputText.implicitHeight + 2 * Theme.spaceXs : 0)
    color: row.failed || rowHover.containsMouse ? Theme.bgElevated : "transparent"

    // A failure's edge, findable while scrolling past at speed.
    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 2 * Theme.borderWidth
        color: Theme.danger
        visible: row.failed
    }

    // Hover only, for the lit ground and the full command: presses belong to `CommandsTextSelect`, laid over the list.
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
    readonly property bool tookLine: row.sel ? row.sel.whole : false
    /// The three columns as `CommandsModel` numbers them (`AT_CLOCK` / `AT_CMD` / `AT_OUT` of
    /// `models/commands/selection.rs`; the odd numbers are the gaps). `CommandsTextSelect` spells the same five, so a
    /// renumbering has to find both.
    readonly property var washCols: [0, 2, 4]

    /// How far into each column the wash reaches, in that column's coordinates — `{ x, w }`, or null — taken from the
    /// column's own layout (`LineRuler`).
    ///
    /// Pushed, not bound (as `DiffLineCell.emphRects` is): asking the ruler writes to it, and a binding that writes
    /// while evaluated is a binding loop that holds the previous row's wash.
    property var washSpans: [null, null, null]
    function settleWash() {
        // `sel` is read again here: a change handler runs before the bindings derived from its property, so those
        // still hold the previous selection (rules-refs/app-ui.md「`onXChanged` は、同じ `x` から導かれる」).
        const wash = row.sel
        const runs = wash ? [wash.clock, wash.command, wash.outcome] : []
        const out = [null, null, null]
        for (let at = 0; at < 3; at++) {
            if (!runs[at] || runs[at].length === 0)
                continue
            const text = row.commandsModel.columnText(row.index, row.washCols[at])
            out[at] = row.bounds(row.ruler.rectsOf(text, false, runs[at]))
        }
        row.washSpans = out
    }
    /// The bounding box of a run's pieces: a bidirectional column comes back as several, and this log's wash is one
    /// rectangle (`washEdge`), so it cannot draw them apart the way the diff does.
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
    // Every input of the wash; a delegate reused for another row (`reuseItems`) arrives through these same handlers.
    onSelChanged: row.settleWash()
    onClockChanged: row.settleWash()
    onArgsChanged: row.settleWash()
    onResultChanged: row.settleWash()
    onDurationChanged: row.settleWash()
    Component.onCompleted: row.settleWash()

    /// The wash is one rectangle from the first column the selection reaches to the last, so it covers the gaps too —
    /// a tab is a character of the line, and a terminal washes it like one.
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
    // Both read only `washSpans`; `settleWash` is the one reader of `sel`.
    readonly property real washX: row.washEdge(true)
    // Clamped to the row: a command too long for its column is elided, and the wash would run on past it.
    readonly property real washRight: Math.min(row.washEdge(false), row.width - Theme.spaceMd)

    Rectangle {
        x: row.washX
        y: 0
        width: Math.max(0, row.washRight - row.washX)
        height: line.height
        color: Theme.bgSelected
    }
    // git's words come with a command line taken whole, so they are washed with it.
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
            // Named on all five labels: the ruler is set in this format (`row.ruler`), and an `AutoText` guess from
            // what a command carries would draw a line the ruler has never seen.
            textFormat: Text.PlainText
            color: Theme.textMuted
            font.family: Theme.monoFamily
            font.pixelSize: Theme.fontSm
        }
        // `git` never varies, so it is muted; what changes from row to row is the part after it.
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
            // One space after `git`: the ruler is handed `git <args>` as one string (`CommandsModel.columnText`), so
            // a selection lands on the pixels the copy names. The width is written, since a right anchor would move
            // the x as well.
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
                // Exit 0 is not written, only the time it took.
                text: row.state === "running" ? qsTr("running…") : row.duration
                textFormat: Text.PlainText
                color: row.state === "running" ? Theme.accent : Theme.textMuted
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontSm
            }
        }
    }

    // Indented to the command. Positioned, not anchored: `program` is inside `line`, neither parent nor sibling.
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
