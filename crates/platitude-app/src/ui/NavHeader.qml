import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Fixed sidebar section header: fold arrow, tinted icon, caption and count, plus the one control that section
// carries at the end of the band — the tags-in-graph eye, or the `+` that writes a remote down.
Rectangle {
    id: header
    property string caption
    property string iconKind: "branch"
    property color iconTint: Theme.textSecondary
    property int count: 0
    /// What the fold was left at. What is actually on show is `showsRows` — an empty section has nothing to open onto.
    property bool expanded: true
    /// Whether this header's section can be closed from here. False on the one the folded rail opens beside itself:
    /// that list is already the only thing on screen, so an arrow offering to close it would be offering to leave
    /// nothing.
    property bool foldable: true
    property bool showTagToggle: false
    property bool tagsShown: true
    /// The `+` that writes a remote down. **Live at zero**, unlike everything else on an empty band: a repository with
    /// no remote is exactly the one where this is worth pressing, and the section it stands on cannot be opened to
    /// find another way in (デザイン規約 §左メニューの所作).
    property bool showAddRemote: false
    signal toggled()
    signal tagsToggled(bool shown)
    signal addRemoteRequested()

    /// Nothing in there, so there is nothing to open: a list of no rows offers no operation, and the number beside the
    /// caption has already said as much. The band keeps its place and goes unavailable — the same answer the folded
    /// rail's cells give (規約 §無効, NavRail).
    readonly property bool empty: header.count === 0
    /// Whether the rows under this band are on show: what the fold was left at, held against what is in it. Read by
    /// the list rather than `expanded`, so the fold a reader left a section at survives a repository that has none of
    /// that section — coming back to one that has them opens it the way it was left.
    readonly property bool showsRows: header.expanded && !header.empty
    /// Whether the band answers a click at all. One answer, for the pointer and for the smoke hook alike
    /// (`tap` — NavRail.tapAt).
    readonly property bool takesClick: header.foldable && !header.empty

    /// A click landed on the band. Named rather than left to the MouseArea, so PG_AUTO_ACT=nav-close reaches the same
    /// refusal a pointer does — clicks cannot be injected (verify-ui スキル).
    function tap() {
        if (header.takesClick)
            header.toggled()
    }

    /// The eye at the end of the band, pressed (PG_AUTO_ACT=tags-eye). Put in at the button rather than at this
    /// component's own signal, so what answers is the band's real wiring and not a second way in written for the run.
    /// **`toggle()` flips the tick without raising `toggled`** — the same seam `StashOptionsCard.clickStagedOnly`
    /// names, and a run that only flips it photographs a graph nobody asked to change. Refused where a pointer would
    /// find nothing to press: at zero tags the switch is not on the band at all.
    function tapTags() {
        if (header.showTagToggle && !header.empty) {
            tagEye.toggle()
            tagEye.toggled()
        }
    }

    Layout.fillWidth: true
    implicitHeight: Theme.rowHeight
    color: Theme.bgElevated
    // A HoverHandler rather than the MouseArea's hover: it also fires over the tag toggle, so the row highlight covers
    // the header's full clickable surface.
    HoverHandler {
        id: headerHover
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        // An empty band washes for nobody — unavailable does not answer the pointer (規約 §無効).
        visible: headerHover.hovered && header.takesClick
    }
    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        NavIcon {
            visible: header.foldable
            width: Theme.iconSm
            height: Theme.iconSm
            kind: "chevron"
            rotation: header.showsRows ? 90 : 0
            tint: header.empty ? Theme.textMuted : Theme.textSecondary
        }
        NavIcon {
            kind: header.iconKind
            // Grey is what unavailable looks like, and it takes the mark with the words (規約 §無効). The kind's own
            // tint is not dropped a step instead: `*Dim` says something about the kind, and what is being said here is
            // that there is nothing to press (デザイン規約 §暗く落とした段).
            tint: header.empty ? Theme.textMuted : header.iconTint
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: header.caption
            color: header.empty ? Theme.textMuted : Theme.textSecondary
            font.pixelSize: Theme.fontSm
            font.weight: Font.DemiBold
        }
        Label {
            text: "(" + header.count + ")"
            color: Theme.textMuted
            font.pixelSize: Theme.fontSm
        }
        Item { Layout.fillWidth: true }
        // Whether the graph is drawing tags. An eye rather than a flag: a flag is a mark on a commit, which is what a
        // tag already is — what this switches is whether they are looked at. Told apart by the tint, the way the panes'
        // tree/flat switches are, not by fading the whole control (§暗く落とした段).
        HoverToolButton {
            id: tagEye
            // With no tags there is nothing to keep out of the graph, so the switch has nothing to answer for — the
            // same seat the folded rail's TAGS cell leaves empty at zero (NavRail).
            visible: header.showTagToggle && !header.empty
            checkable: true
            checked: header.tagsShown
            padding: 0
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            contentItem: NavIcon {
                kind: header.tagsShown ? "eye" : "eye-off"
                tint: header.tagsShown ? Theme.refTag : Theme.refTagDim
            }
            tip: header.tagsShown ? qsTr("Hide tags in the graph") : qsTr("Show tags in the graph")
            onToggled: header.tagsToggled(checked)
        }
        // Writing a remote down, in the seat the TAGS band keeps its eye in — one control per section, at the end of
        // the band. It wears the section's own tint however dark the rest of the band has gone: greying it would say
        // it cannot be pressed, and at zero remotes it is the only thing here that can (規約 §無効).
        HoverToolButton {
            visible: header.showAddRemote
            // The seat stays the one every icon button in the window sits in; the mark inside it is one step down, so
            // it is drawn on the same grid as the mark at the head of this band (`iconMd` — デザイン規約 §寸法). A `Control`
            // stretches its `contentItem` over whatever the padding leaves, so the step is written as that padding and
            // not on the icon: a width put on the icon is overwritten on the next layout.
            //
            // Undropped, the cross was the only mark in the window drawn on the full `iconLg` grid — and the only one
            // that fills its box in both axes, so it carried a good half again the ink of the `+` on a WIP row
            // (measured: 14 against 12, against 8 on a diff line).
            padding: (Theme.iconLg - Theme.iconMd) / 2
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            contentItem: NavIcon {
                kind: "plus"
                tint: header.iconTint
            }
            // The dialog it opens is named by the `…`, the way the chooser's own row names it (デザイン規約 §長押し の語彙).
            tip: qsTr("Add remote…")
            onClicked: header.addRemoteRequested()
        }
    }
    MouseArea {
        anchors.fill: parent
        // Leave the control at the end of the band clickable.
        anchors.rightMargin: (header.showTagToggle && !header.empty) || header.showAddRemote ? Theme.spaceXl : 0
        enabled: header.takesClick
        onClicked: header.tap()
    }
}
