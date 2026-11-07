pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The band over the graph while an entry of the discard log is on it: what the operation was and when,
// what each of its parts would bring back and as what, the press that brings it back, and the way out of the showing
// — the left panel only names the entry. The rebase plan's head is the model: a mode, named over its rows and
// underlined in the accent (`RebasePlanPane`).
//
// The presses say what the Windows Recycle Bin says for the same act: `Restore` for one thing, `Restore all` for every
// part of an entry whose parts each come back alone too. The branches one rebase moved come back together, under the
// one `Restore` (`byPart`); a single commit of theirs is a cherry-pick from the rows shown on the graph.
//
// One rebase can move any number of branches, and the band describes a graph it must not take the place of: it stands
// at `Theme.recoverBandMaxHeight` at the most, whatever the window, and the window's floor leaves the graph its rows
// under that (`PageLayout.panesMinHeight`). Past it the lines scroll; the heading and the date stay put, a list whose
// heading has scrolled away no longer saying what it lists (`DetailsMessageBlock`), and so do the controls. The heading
// is cut at `Theme.recoverBandHeadRows`, its whole under the hand: past those it is a rebase's branches, each said on a
// line of its own below, or long names in a graph at its narrowest.
//
// The band, its lines and its presses are user decisions — ask before changing.
Rectangle {
    id: band

    /// The entry on the graph, as the list words it (`RecoverEntries.entryOf`); null for none.
    property var entry: null
    /// The moment in full, then how long ago (`RecoverEntries.whenWords`).
    property string when: ""
    /// Per part, whether what it would bring back is on the graph: a part whose tip the graph has not walked to cannot
    /// come back until it is (`RepoPage.recoverReach`). Empty while the walk that would tell is still out: nothing is
    /// pressed, and nothing is said to be out of reach.
    property var reach: []
    /// The graph can walk further than it has, `step` commits at a press (`GraphTailFooter`).
    property bool canWalkFurther: false
    property int step: 0
    /// A write is out: the presses wait for it.
    property bool busy: false

    /// The press that brings it back — every part, or the one `part` (-1 for all).
    signal restoreAsked(int part)
    /// The showing is put down.
    signal dismissed()
    /// Walk the graph further, toward what the entry would bring back.
    signal furtherAsked()

    readonly property var parts: band.entry === null ? [] : band.entry.parts
    /// Each part can be brought back alone as well (`Discard::restores_by_part`).
    readonly property bool byPart: band.entry !== null && band.entry.byPart
    readonly property bool outOfReach: band.reach.some(inReach => !inReach)
    /// Every part can be pressed: the walk has told, and all of it is on the graph.
    readonly property bool allInReach: band.parts.length > 0 && band.reach.length === band.parts.length
                                       && !band.outOfReach
    /// What the ceiling leaves the parts' lines under everything that stays put, a line at the least: only a graph at
    /// its narrowest, wrapping all of that at once, comes near it.
    readonly property real partsRoom: Math.max(
        Theme.fontSmLine,
        Theme.recoverBandMaxHeight - 2 * Theme.spaceSm
        - heading.implicitHeight - Theme.spaceXs - whenLine.implicitHeight - Theme.spaceXs
        - (band.outOfReach ? Theme.spaceXs + reachRow.implicitHeight : 0))
    /// The entry across reads: a read makes a new object of the one on the graph (`RecoverEntries.entries`), and a
    /// reader's place in its lines stays through that.
    readonly property string entryKey: band.entry === null ? "" : band.entry.key

    /// The line under the title where the entry was another worktree's: when, then where.
    function whereWords(when, worktree) {
        return qsTr("%1 · in %2").arg(when).arg(worktree)
    }
    /// A part's line: what it took, then what the press makes of it.
    function partWords(taken, restore) {
        return qsTr("%1 — %2").arg(taken).arg(restore)
    }

    implicitHeight: bandRow.implicitHeight + 2 * Theme.spaceSm
    color: Theme.bgElevated
    // Another entry's lines start at their top: the band stays standing from pick to pick. The bar's step in flight
    // goes with the lines it was sending (rules-refs/app-ui.md「ホイールの 1 ノッチは送られる、跳ばない」).
    onEntryKeyChanged: {
        partsBar.stepGlide.halt()
        partsView.contentY = 0
    }

    RowLayout {
        id: bandRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Theme.spaceMd
        anchors.rightMargin: Theme.spaceMd
        anchors.topMargin: Theme.spaceSm
        spacing: Theme.spaceSm
        ColumnLayout {
            id: column
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: Theme.spaceXs
            // What was done, then when and where — the heading's two halves, as a commit's card gives subject and date.
            // Every name behind its kind's mark (デザイン規約 §ref の種別「名前の印」, `RecoverEntries.titleMarksOf`).
            CardText {
                id: heading
                Layout.fillWidth: true
                text: band.entry === null ? "" : band.entry.title
                marks: band.entry === null ? [] : band.entry.titleMarks
                // A heading: a long name is cut at the name ceiling, whole under the hand (デザイン規約 §寸法「名前の天井」).
                holdsNames: true
                capRows: Theme.recoverBandHeadRows
                tipsCap: true
                color: Theme.textPrimary
                pixelSize: Theme.fontMd
                weight: Theme.fontWeightStrong
            }
            CardText {
                id: whenLine
                Layout.fillWidth: true
                text: band.entry === null || band.entry.worktree === ""
                      ? band.when : band.whereWords(band.when, band.entry.worktree)
                marks: band.entry === null || band.entry.worktree === "" ? []
                       : Words.marksOf(worktree => band.whereWords(band.when, worktree),
                                       [[band.entry.worktree, band.entry.worktreeMark]])
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
            // What the press acts on: the lines it stands on the middle of (`restoreEntry`), the date not among them.
            ColumnLayout {
                id: lines
                Layout.fillWidth: true
                spacing: Theme.spaceXs
                Item {
                    id: partsSeat
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(partsColumn.implicitHeight, band.partsRoom)
                    Flickable {
                        id: partsView
                        anchors.fill: parent
                        // The floating bar stands in the gap before the press rather than over the line ends, where a
                        // part's own press is, and the words keep their width as it comes and goes: a width that moved
                        // with it could wrap it away again (規約 §QML 実装ルール のバーの選び方).
                        anchors.rightMargin: -bandRow.spacing
                        contentWidth: partsSeat.width
                        contentHeight: partsColumn.implicitHeight
                        // The content is narrower than the view by that gap, which would let a hand drag it sideways.
                        flickableDirection: Flickable.VerticalFlick
                        // Only while something is cut off: a press's wash reaches past the line it stands on, into the
                        // lines' spacing, and past the first and the last there is nothing to take it off.
                        clip: partsBar.visible
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: AutoScrollBar {
                            id: partsBar
                        }
                        // On the view's own frame, not in its content, which travels with the scroll.
                        MiddleAutoScroll {
                            parent: partsView
                            anchors.fill: parent
                            visible: partsBar.visible
                            onDrifted: dy => partsView.contentY =
                                Math.max(0, Math.min(partsView.contentY + dy,
                                                     partsView.contentHeight - partsView.height))
                        }
                        ColumnLayout {
                            id: partsColumn
                            width: partsSeat.width
                            spacing: Theme.spaceXs
                            // One line a part, a list's pitch apart: its mark in the left panel's tint, what it took,
                            // and what the press makes of it — and, where the parts come back alone, the press for that
                            // one.
                            Repeater {
                                model: band.parts
                                delegate: RowLayout {
                                    id: partLine
                                    required property var modelData
                                    required property int index
                                    Layout.fillWidth: true
                                    spacing: Theme.spaceXs
                                    // The head of the line's words, on their capitals' middle (デザイン規約 §アイコンの席
                                    // の文頭記号) — the words' top is the row's (they are its tallest item).
                                    RecoverMark {
                                        Layout.preferredWidth: implicitWidth
                                        Layout.preferredHeight: implicitHeight
                                        Layout.alignment: Qt.AlignTop
                                        Layout.topMargin: Math.round(partText.baselineOffset
                                                                     - WordFace.capitalMiddle(partText.pixelSize)
                                                                     - implicitHeight / 2)
                                        size: partText.pixelSize
                                        kind: partLine.modelData.mark
                                        tint: partLine.modelData.tint
                                        badged: partLine.modelData.badged
                                    }
                                    // Wrapped, never cut at the name ceiling (user decision — ask before changing):
                                    // the line says what the press brings back as, down to the name's tail, and the
                                    // band's ceiling answers for how tall that gets.
                                    CardText {
                                        id: partText
                                        Layout.fillWidth: true
                                        text: band.partWords(partLine.modelData.text, partLine.modelData.restore)
                                        // The restore half's names, seated after what the part took.
                                        marks: {
                                            const lead = Words.standSeats(
                                                band.partWords(partLine.modelData.text, Words.stand),
                                                partLine.modelData.restore)
                                            return lead.length === 0 ? []
                                                 : partLine.modelData.restoreMarks
                                                       .map(mark => [mark[0] + lead[0], mark[1], mark[2], mark[3]])
                                        }
                                        color: Theme.textPrimary
                                        pixelSize: Theme.fontSm
                                    }
                                    // The words' own height, the press on the middle of their lines, its word on theirs
                                    // in whole pixels where they are one line: a button's height would open the lines
                                    // apart, and centred on the line its word stands under the line's by half the
                                    // leading a field files under its line (2〜3px on macOS). The seat is centred in the
                                    // row as the words are, so its top is theirs.
                                    Item {
                                        visible: band.byPart
                                        Layout.preferredWidth: partRestore.implicitWidth
                                        Layout.preferredHeight: partText.implicitHeight
                                        HoverToolButton {
                                            id: partRestore
                                            y: Math.round(partText.baselineOffset - partRestore.baselineOffset
                                                          + (partText.implicitHeight - partText.lineHeight) / 2)
                                            // Down to the wash's own height (`HoverToolButton`), which the lines'
                                            // spacing holds.
                                            topPadding: 0
                                            bottomPadding: 0
                                            enabled: band.reach[partLine.index] === true && !band.busy
                                            text: qsTr("Restore")
                                            font.pixelSize: Theme.fontSm
                                            onClicked: band.restoreAsked(partLine.index)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Not yet on the graph: the press waits, and the way there is offered in the words the graph's own end
                // offers it with (`GraphTailFooter`). Out of the scroll: it says why the press is dead. The press
                // stands right after the words; a graph too narrow for both wraps the words rather than push the band's
                // controls off its column.
                RowLayout {
                    id: reachRow
                    visible: band.outOfReach
                    Layout.fillWidth: true
                    Layout.maximumWidth: implicitWidth
                    spacing: Theme.spaceSm
                    // On one baseline in whole pixels, as a part's press is: both hang from the top and the one whose
                    // word stands higher drops to the other's.
                    CardText {
                        id: reachWords
                        Layout.fillWidth: true
                        text: qsTr("Not loaded on the graph yet")
                        color: Theme.warning
                        pixelSize: Theme.fontSm
                        Layout.alignment: Qt.AlignTop
                        Layout.topMargin: walkFurther.visible
                                          ? Math.max(0, Math.round(walkFurther.baselineOffset
                                                                   - reachWords.baselineOffset))
                                          : 0
                    }
                    HoverToolButton {
                        id: walkFurther
                        Layout.alignment: Qt.AlignTop
                        Layout.topMargin: Math.max(0, Math.round(reachWords.baselineOffset
                                                                 - walkFurther.baselineOffset))
                        visible: band.canWalkFurther
                        // The step is a count: said as a number, the button's words stay the app's whatever the band
                        // hands it (a tool button leaves its words to the style, which guesses their format).
                        text: Words.loadFurther(Number(band.step))
                        font.pixelSize: Theme.fontSm
                        onClicked: band.furtherAsked()
                    }
                }
            }
        }
        // The controls' column (user decision — ask before changing): the `✕` up in the corner, as the question bar's
        // (`AskBar`), and the press on the middle of the lines it acts on, as that bar's stands beside its one line,
        // the heading and the date keeping it clear of the `✕`. Their right edges are one line. Centred on one line the
        // press reaches below the words, and this column sets the band's height.
        Item {
            id: controls
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: restoreEntry.implicitWidth
            implicitHeight: restoreEntry.y + restoreEntry.height
            ActionButton {
                id: restoreEntry
                anchors.right: parent.right
                y: Math.round(lines.y + (lines.height - restoreEntry.height) / 2)
                height: Theme.controlHeight
                enabled: band.allInReach && !band.busy
                text: band.byPart ? qsTr("Restore all") : qsTr("Restore")
                frameColor: Theme.accent
                onActivated: band.restoreAsked(-1)
            }
            // The way out of the showing, as the list's own `✕` is (`RecoverPane`): the mark's right edge on the
            // press's and its top on the heading's, the rest of its seat hanging past them.
            CloseToolButton {
                id: stopShowing
                x: controls.width - stopShowing.width + (stopShowing.width - stopShowing.markSize) / 2
                y: -(stopShowing.height - stopShowing.markSize) / 2
                Accessible.name: qsTr("Stop showing it")
                onClicked: band.dismissed()
            }
        }
    }
    // The mode's own underline, as the plan's head draws it.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Theme.borderWidth
        color: Theme.accent
    }
}
