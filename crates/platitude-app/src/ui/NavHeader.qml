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
    /// What the fold was left at; what is on show is `showsRows`.
    property bool expanded: true
    /// Whether this section can be closed from here — false on the one the folded rail opens beside itself, which is
    /// all that is on screen.
    property bool foldable: true
    property bool showTagToggle: false
    property bool tagsShown: true
    /// The `+` that writes a remote down. Live at zero, unlike the rest of an empty band: a repository with no remote
    /// is where it is needed (デザイン規約 §左メニューの所作).
    property bool showAddRemote: false
    /// Held while the pane's write doors are (`SidebarPane.doorsHeld`) — the one thing on the band that writes.
    /// Greyed, not hidden (規約 §無効).
    property bool addHeld: false
    signal toggled()
    signal tagsToggled(bool shown)
    signal addRemoteRequested()

    /// No rows, nothing to open: the band keeps its place and goes unavailable, as the folded rail's cells do
    /// (規約 §無効).
    readonly property bool empty: header.count === 0
    /// Whether the rows are on show. The list reads this rather than `expanded`, so the fold a reader left survives a
    /// repository with none of that section.
    readonly property bool showsRows: header.expanded && !header.empty
    /// Whether the band answers a click — for the pointer and the smoke hook alike (`tap`).
    readonly property bool takesClick: header.foldable && !header.empty

    /// A click landed on the band; PGG_AUTO_ACT=nav-close comes in here too, so it meets the same refusal.
    function tap() {
        if (header.takesClick)
            header.toggled()
    }

    /// The eye pressed (PGG_AUTO_ACT=tags-eye), through the button's own wiring: `toggle()` flips the tick without
    /// raising `toggled`, so both are called. Refused where there is no eye to press (zero tags).
    function tapTags() {
        if (header.showTagToggle && !header.empty) {
            tagEye.toggle()
            tagEye.toggled()
        }
    }

    Layout.fillWidth: true
    implicitHeight: Theme.rowHeight
    color: Theme.bgElevated
    // A HoverHandler: it also fires over the tag toggle, so the row highlight covers the header's full clickable
    // surface.
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
        anchors.leftMargin: Theme.spaceXs
        anchors.rightMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        // An `iconMd` chevron in an `iconSm` seat: the ink grows a step over a row's fold arrow (`NameCell`), the
        // caption does not move, and the strokes still fit the seat (デザイン規約 §左メニューの所作「畳みの山形は 2 階級ある」).
        Item {
            visible: header.foldable
            Layout.preferredWidth: Theme.iconSm
            Layout.preferredHeight: Theme.iconSm
            NavIcon {
                anchors.centerIn: parent
                width: Theme.iconMd
                height: Theme.iconMd
                kind: "chevron"
                rotation: header.showsRows ? 90 : 0
                tint: header.empty ? Theme.textMuted : Theme.textSecondary
            }
        }
        NavIcon {
            kind: header.iconKind
            // Unavailable greys the mark with the words (規約 §無効); `*Dim` would say something about the kind.
            tint: header.empty ? Theme.textMuted : header.iconTint
            width: Theme.iconMd
            height: Theme.iconMd
        }
        Label {
            text: header.caption
            color: header.empty ? Theme.textMuted : Theme.textSecondary
            font.pixelSize: Theme.fontMd
            font.weight: Theme.fontWeightStrong
        }
        // At the caption's own step, in its own label: what says it is a count is the weight and the colour
        // (デザイン規約 §タイポグラフィ).
        Label {
            text: "(" + header.count + ")"
            color: Theme.textMuted
            font.pixelSize: Theme.fontMd
        }
        Item { Layout.fillWidth: true }
        // Whether the graph draws tags — an eye, since a flag would read as a tag itself. On and off told apart by the
        // tint (§暗く落とした段).
        HoverToolButton {
            id: tagEye
            // Gone at zero tags, as on the folded rail's TAGS cell.
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
        // Writing a remote down, in the eye's seat. It keeps the section's tint on an empty band: it is the one thing
        // there that can be pressed (規約 §無効).
        HoverToolButton {
            visible: header.showAddRemote
            enabled: !header.addHeld
            // The usual icon-button seat, the mark a step down on the band's `iconMd` grid (デザイン規約 §寸法) — on the
            // full `iconLg` grid the cross outweighs every other `+`. Written as padding: a `Control` stretches its
            // `contentItem`, so a width on the icon is overwritten.
            padding: (Theme.iconLg - Theme.iconMd) / 2
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            contentItem: NavIcon {
                kind: "plus"
                // Coloured here: an own `contentItem` does not go through the palette (規約 §無効).
                tint: header.addHeld ? Theme.textMuted : header.iconTint
            }
            // The dialog it opens is named by the `…`, the way the chooser's own row names it (デザイン規約 §長押し の語彙).
            tip: Words.addRemote
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
