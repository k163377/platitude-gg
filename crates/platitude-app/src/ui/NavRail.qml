pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The sidebar folded down to its spine: one cell per section, the icon
// saying what is in it and the number under it how much. The pointer
// resting on a cell opens that one section beside the rail; a click puts
// the whole list back with that section expanded in it.
//
// The cells only report the pointer. The list that opens is the
// sidebar's, because it has to outlive the hover that raised it — the
// pointer walking into it comes off the cell on the way (the same reason
// the graph's chips hand their popup to the page).
Rectangle {
    id: rail

    required property var branchesModel
    required property var remotesModel
    required property var worktreesModel
    required property var stashesModel
    required property var tagsModel
    /// Whether the graph is showing tags. The rail is the only place the
    /// answer can be while the list is folded — the switch itself is in
    /// the TAGS section, which is a hover away.
    required property bool tagsShown

    /// The section the sidebar has open beside the rail: its cell keeps
    /// the hover look while the pointer is down in the list.
    property string openKind: ""

    /// The pointer came to rest on a cell. `top` is where that cell sits,
    /// so the section can open level with the icon it belongs to.
    signal peekRequested(string kind, real top)
    /// It left a cell without arriving on another.
    signal peekLeft()
    /// Put the list back. `kind` is the section that was asked for, which
    /// is the one that has to be open in it.
    signal unfoldRequested(string kind)

    // A square cell: the icon on its 20 grid with the count's line under
    // it, spaceXs above and below. Not a token yet — the value is being
    // looked at in place before it takes a row in 規約 §寸法.
    readonly property int cellSize: 44

    // The sections in the order the open sidebar stacks them, wearing the
    // tints their headers wear (NavHeader).
    readonly property var sections: [
        { kind: "branch", icon: "branch", tint: Theme.accent },
        { kind: "remote", icon: "remote", tint: Theme.textSecondary },
        { kind: "worktree", icon: "tree", tint: Theme.success },
        { kind: "stash", icon: "stash", tint: Theme.textSecondary },
        { kind: "tag", icon: "tag", tint: Theme.refTag }
    ]

    /// A section's icon and tint, so the one the sidebar opens beside the
    /// rail wears the same mark as the cell that opened it. An unnamed
    /// section answers with the first one's look rather than nothing —
    /// the caller is a binding that outlives the choice.
    function sectionOf(kind) {
        for (let i = 0; i < rail.sections.length; i++) {
            if (rail.sections[i].kind === kind)
                return rail.sections[i]
        }
        return rail.sections[0]
    }

    function modelOf(kind) {
        return kind === "branch" ? rail.branchesModel
             : kind === "remote" ? rail.remotesModel
             : kind === "worktree" ? rail.worktreesModel
             : kind === "stash" ? rail.stashesModel : rail.tagsModel
    }

    /// Where a section's cell sits, for anything that has to line up with
    /// it without the pointer having been there (the smoke hook).
    function topOf(kind) {
        for (let i = 0; i < rail.sections.length; i++) {
            if (rail.sections[i].kind === kind)
                return Theme.headerHeight + i * rail.cellSize
        }
        return Theme.headerHeight
    }

    implicitWidth: rail.cellSize
    color: Theme.bgSurface

    Column {
        anchors.fill: parent
        spacing: 0

        // The pane's header band, which folded is the fold control and
        // nothing else — the same block that sits at the end of the band
        // when the list is open, holding the whole width because there is
        // no filter beside it to divide it from. The panes either side
        // still start where this ends (規約 §QML 実装ルール).
        FoldBlock {
            width: parent.width
            height: Theme.headerHeight
            mark: "»"
            tip: qsTr("Unfold the list")
            divided: false
            // No section asked for: whatever was open stays open.
            onActivated: rail.unfoldRequested("")
            // The hairline the filter's underline leaves behind, so the
            // band reads as the same band it was before it folded.
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: Theme.borderSubtle
            }
        }

        Repeater {
            model: rail.sections
            delegate: Rectangle {
                id: cell
                required property var modelData
                readonly property var sectionModel: rail.modelOf(cell.modelData.kind)
                readonly property bool open: rail.openKind === cell.modelData.kind
                // Tags are the one section that can be kept out of the
                // graph, so its cell is the one that says whether they
                // are in it.
                readonly property bool taggable: cell.modelData.kind === "tag"
                readonly property bool offGraph: cell.taggable && !rail.tagsShown

                width: rail.cellSize
                height: rail.cellSize
                // The open section keeps the hover wash while the pointer
                // is away in its list: what is on screen has to say which
                // cell put it there.
                color: cellHover.hovered || cell.open ? Theme.bgHover : "transparent"

                Column {
                    anchors.centerIn: parent
                    spacing: 0
                    NavIcon {
                        id: sectionIcon
                        anchors.horizontalCenter: parent.horizontalCenter
                        kind: cell.modelData.icon
                        // Off the graph is a state, and a state is said by
                        // dropping the mark a step, not by greying it —
                        // grey text is what unavailable looks like
                        // (デザイン規約 §暗く落とした段).
                        tint: cell.offGraph ? Theme.refTagDim
                                            : cell.modelData.tint
                        width: Theme.iconLg
                        height: Theme.iconLg
                        // The eye the TAGS header carries, worn as a mark
                        // on the corner: folded, this is the only place
                        // "are tags in the graph" can be answered. It is
                        // open or it is not there — a mark that is always
                        // up says nothing, and two steps of colour is not
                        // a state anyone reads at this size. Smaller than
                        // the section's own mark, because it is about the
                        // mark rather than beside it.
                        NavIcon {
                            visible: cell.taggable && rail.tagsShown
                            kind: "eye"
                            tint: sectionIcon.tint
                            width: Theme.iconSm
                            height: Theme.iconSm
                            anchors.horizontalCenter: parent.right
                            anchors.verticalCenter: parent.top
                        }
                    }
                    // How many are in there. It stands in for the caption
                    // as well as the count, so it takes the caption's
                    // colour rather than the dimmer one a count wears when
                    // a word is already carrying the section (NavHeader).
                    Label {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: cell.sectionModel.total
                        color: Theme.textSecondary
                        font.pixelSize: Theme.fontSm
                        lineHeightMode: Text.FixedHeight
                        lineHeight: Theme.fontSmLine
                    }
                }
                // Passive, so that walking from a cell into the list it
                // opened does not have the list's own hover taken away.
                HoverHandler {
                    id: cellHover
                    onHoveredChanged: {
                        if (cellHover.hovered)
                            rail.peekRequested(cell.modelData.kind, cell.y)
                        else
                            rail.peekLeft()
                    }
                }
                TapHandler {
                    onTapped: rail.unfoldRequested(cell.modelData.kind)
                }
            }
        }
    }
}
