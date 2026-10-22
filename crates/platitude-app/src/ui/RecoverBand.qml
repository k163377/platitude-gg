pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The band over the graph while an entry of the discard log is on it (破棄記録仕様.md): what the operation was and when,
// what each of its parts would bring back and as what, the press that brings it back, and the way out of the showing
// — the left panel only names the entry. The rebase plan's head is the model: a mode, named over its rows and
// underlined in the accent (`RebasePlanPane`).
//
// The presses say what the Windows Recycle Bin says for the same act: `Restore` for one thing, `Restore all` for every
// part of an entry whose parts each come back alone too. The branches one rebase moved come back together, under the
// one `Restore` (`byPart`).
Rectangle {
    id: band

    /// The entry on the graph, as the list words it (`RecoverEntries.entryOf`); null for none.
    property var entry: null
    /// The moment in full, then how long ago (`RecoverPane.whenWords`).
    property string when: ""
    /// Per part, whether what it would bring back is on the graph: a part whose tip the graph has not walked to cannot
    /// come back until it is (破棄記録仕様.md §4). Empty while the walk that would tell is still out: nothing is
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

    /// The line under the title where the entry was another worktree's: when, then where.
    function whereWords(when, worktree) {
        return qsTr("%1 · in %2").arg(when).arg(worktree)
    }
    /// A part's line: what it took, then what the press makes of it.
    function partWords(taken, restore) {
        return qsTr("%1 — %2").arg(taken).arg(restore)
    }

    implicitHeight: column.implicitHeight + 2 * Theme.spaceSm
    color: Theme.bgElevated

    RowLayout {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: Theme.spaceMd
        anchors.topMargin: Theme.spaceSm
        spacing: Theme.spaceSm
        ColumnLayout {
            id: column
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            // What was done, then when and where — the heading's two halves, as a commit's card gives subject and date.
            // Every name behind its kind's mark (デザイン規約 §ref の種別「名前の印」, `RecoverEntries.titleMarksOf`).
            CardText {
                Layout.fillWidth: true
                text: band.entry === null ? "" : band.entry.title
                marks: band.entry === null ? [] : band.entry.titleMarks
                // A heading: a long name is cut at the name ceiling, whole under the hand (デザイン規約 §寸法「名前の天井」).
                holdsNames: true
                color: Theme.textPrimary
                pixelSize: Theme.fontMd
                weight: Theme.fontWeightStrong
            }
            CardText {
                Layout.fillWidth: true
                text: band.entry === null || band.entry.worktree === ""
                      ? band.when : band.whereWords(band.when, band.entry.worktree)
                marks: band.entry === null || band.entry.worktree === "" ? []
                       : Words.marksOf(worktree => band.whereWords(band.when, worktree),
                                       [[band.entry.worktree, band.entry.worktreeMark]])
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
            // One line a part, a list's pitch apart: its mark in the left panel's tint, what it took, and what the press
            // makes of it — and, where the parts come back alone, the press for that one.
            Repeater {
                model: band.parts
                delegate: RowLayout {
                    id: partLine
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    spacing: Theme.spaceXs
                    // The head of the line's words, on their capitals' middle (デザイン規約 §アイコンの席 の文頭記号) —
                    // the words' top is the row's (they are its tallest item).
                    RecoverMark {
                        Layout.preferredWidth: implicitWidth
                        Layout.preferredHeight: implicitHeight
                        Layout.alignment: Qt.AlignTop
                        Layout.topMargin: Math.round(partText.baselineOffset
                                                     - WordFace.capitalMiddle(partText.pixelSize) - implicitHeight / 2)
                        size: partText.pixelSize
                        kind: partLine.modelData.mark
                        tint: partLine.modelData.tint
                        badged: partLine.modelData.badged
                    }
                    CardText {
                        id: partText
                        Layout.fillWidth: true
                        text: band.partWords(partLine.modelData.text, partLine.modelData.restore)
                        // The restore half's names, seated after what the part took.
                        marks: {
                            const lead = Words.standSeats(band.partWords(partLine.modelData.text, Words.stand),
                                                          partLine.modelData.restore)
                            return lead.length === 0 ? []
                                 : partLine.modelData.restoreMarks
                                       .map(mark => [mark[0] + lead[0], mark[1], mark[2], mark[3]])
                        }
                        color: Theme.textPrimary
                        pixelSize: Theme.fontSm
                    }
                    // The line's own height, the press on its words' baseline in whole pixels: a button's height
                    // would open the lines apart, and centred on the line its word stands under the line's by half the
                    // leading a field files under its line (2〜3px on macOS). The seat is centred in the row as the
                    // line is, so its top is the line's.
                    Item {
                        visible: band.byPart
                        Layout.preferredWidth: partRestore.implicitWidth
                        Layout.preferredHeight: partText.implicitHeight
                        HoverToolButton {
                            id: partRestore
                            y: Math.round(partText.baselineOffset - partRestore.baselineOffset)
                            // Down to the wash's own height (`HoverToolButton`), which the lines' spacing holds.
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
            // Not yet on the graph: the press waits, and the way there is offered in the words the graph's own end
            // offers it with (破棄記録仕様.md §4, `GraphTailFooter`).
            RowLayout {
                visible: band.outOfReach
                spacing: Theme.spaceSm
                // On one baseline in whole pixels, as a part's press is: both hang from the top and the one whose word
                // stands higher drops to the other's.
                CardText {
                    id: reachWords
                    text: qsTr("Not loaded on the graph yet")
                    color: Theme.warning
                    pixelSize: Theme.fontSm
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: walkFurther.visible
                                      ? Math.max(0, Math.round(walkFurther.baselineOffset - reachWords.baselineOffset)) : 0
                }
                HoverToolButton {
                    id: walkFurther
                    Layout.alignment: Qt.AlignTop
                    Layout.topMargin: Math.max(0, Math.round(reachWords.baselineOffset - walkFurther.baselineOffset))
                    visible: band.canWalkFurther
                    text: qsTr("Load %L1 more commits").arg(band.step)
                    font.pixelSize: Theme.fontSm
                    onClicked: band.furtherAsked()
                }
            }
        }
        // Centred down the band, as a notice's and a question's answer is (`NoticeBar`, `AskBar`).
        ActionButton {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredHeight: Theme.controlHeight
            enabled: band.allInReach && !band.busy
            text: band.byPart ? qsTr("Restore all") : qsTr("Restore")
            frameColor: Theme.accent
            onActivated: band.restoreAsked(-1)
        }
        // The way out of the showing, as the list's own `✕` is (`RecoverPane`).
        CloseToolButton {
            Layout.alignment: Qt.AlignVCenter
            Accessible.name: qsTr("Stop showing it")
            onClicked: band.dismissed()
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
