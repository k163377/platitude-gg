pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// One tab in the strip: the repository's name, the mark that closes it, and the two washes that say which tab is in
// front and which one the pointer is on (デザイン規約 §タブの所作).
Rectangle {
    id: tabItem
    required property int index
    required property int tab_id
    required property string title
    required property string repo_path
    /// The strip's own model: the one question a tab asks of it, and the one thing the mark does to it.
    required property var tabsModel
    /// What every name in the strip is capped at right now (`TabStrip.settleTitleCap`).
    property real titleCap: 0
    /// What a tab costs in air before its name has a letter in it, and the air the mark keeps inside its own box. Both
    /// settled once for the whole strip (`TabStrip`), so this and the cap agree on what a tab costs.
    property real padW: 0
    property int markAir: 0
    /// The strip's height, which every tab is drawn at.
    property real stripHeight: 0
    readonly property bool current: tabItem.tabsModel.currentIndex === tabItem.index
    /// Whether the pointer is on this tab. The real hover and the smoke hook write this one property — hover is the
    /// input that cannot be injected, so the wash and the mark have to be answering a single question or the headless
    /// run proves nothing about either.
    property bool pointed: false
    /// Automation: whether the mark is out on this tab. Read off the mark itself — reporting what was asked of it would
    /// go on passing after the binding that draws it had come apart.
    readonly property real markShown: closeMark.opacity
    /// Pressed with a button the tab answers. Which button means what is the strip's to say, since the same rule is
    /// what the middle-click hook comes through (`TabStrip.pressTab`).
    signal tabPressed(int button)

    // The mark's seat is given only the air it has not already taken (`spaceSm − markAir`) — otherwise both are spent
    // twice and the gap inside the tab reads wider than the tab's own margins (実測 13px between name and mark against
    // 9px to the edge). Exact fit: anything the layout cannot hand out lands on the right margin, where nobody wrote it
    // down. Rounded up so this and `settleTitleCap` agree on what the tab costs, or the strip scrolls by the fractions
    // they disagree about.
    width: Math.ceil(tabContent.implicitWidth) + tabItem.padW
    height: tabItem.stripHeight
    color: tabItem.current ? Theme.bgSelected : "transparent"
    // The whole tab answers the middle button; the `✕` does not accept it, so a press on the mark falls through to the
    // same gesture.
    MouseArea {
        id: tabMouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        onClicked: mouse => tabItem.tabPressed(mouse.button)
        onContainsMouseChanged: tabItem.pointed = containsMouse
    }
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: tabItem.pointed && !tabItem.current
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 2 * Theme.borderWidth
        color: Theme.accent
        visible: tabItem.current
    }
    RowLayout {
        id: tabContent
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceSm
        anchors.rightMargin: Theme.spaceSm - tabItem.markAir
        spacing: 0
        Label {
            text: tabItem.title
            elide: Text.ElideRight
            // The cap the whole strip shares; capping the hint is what narrows the tab.
            Layout.maximumWidth: tabItem.titleCap
            Layout.fillHeight: true
            verticalAlignment: Text.AlignVCenter
            font.weight: tabItem.current ? Font.DemiBold : Font.Normal
            color: Theme.textPrimary
        }
        // Shown on the tab in front and under the pointer (デザイン規約 §タブの所作). Dimmed rather than dropped: an item the
        // layout has stopped seeing takes its width with it, and the tab would change size under the hand that came to
        // close it.
        HoverToolButton {
            id: closeMark
            padding: 0
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.iconLg
            implicitHeight: Theme.iconLg
            opacity: tabItem.current || tabItem.pointed ? 1 : 0
            contentItem: Item {
                NavIcon {
                    anchors.centerIn: parent
                    width: Theme.iconSm
                    height: Theme.iconSm
                    kind: "close"
                    tint: Theme.textSecondary
                }
            }
            onClicked: tabItem.tabsModel.closeTab(tabItem.tab_id)
        }
    }
}
