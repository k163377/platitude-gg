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
    ///
    /// Written by the handler below rather than by the MouseArea that fills the tab: hover goes to the topmost item
    /// that takes it, and the `✕` is a Control that takes its own, so the tab stopped being "under the hand" exactly
    /// when the hand arrived at the mark — which dropped the mark out from under it (rules-refs/app-ui.md 「行の
    /// hover を `MouseArea` で取らない」; 実測 qmltestrunner: `pointed=false mark=0` with the pointer in the middle
    /// of the `✕`).
    property bool pointed: false
    /// Automation: whether the mark is out on this tab. Read off the mark itself — reporting what was asked of it would
    /// go on passing after the binding that draws it had come apart.
    readonly property real markShown: closeMark.opacity
    /// Automation: how far this tab is drawn from the row it belongs to, read off the transform that carries it rather
    /// than off what was asked of it — the same reason `markShown` is read off the mark (`PG_AUTO_ACT=tab-hold`).
    readonly property real shiftShown: heldShift.x
    /// Whether this is the tab in hand, and where the hand has carried its left edge to. Both settled by the strip
    /// (`TabStrip.carryTo`): the order changes underneath a drag, so which row is being carried is not something a row
    /// can remember about itself.
    property bool held: false
    property real heldX: 0
    /// Pressed with a button the tab answers. Which button means what is the strip's to say, since the same rule is
    /// what the middle-click hook comes through (`TabStrip.pressTab`).
    signal tabPressed(int button)
    /// Taken up to be carried, `grabX` being where inside the tab the hand took hold, and set down again. Between the
    /// two the hand reports where it has got to, in **scene** coordinates: by then the tab is drawn somewhere its own
    /// place does not say, and mapping out through this item's transform is what makes the answer the pointer's.
    signal tabTaken(real grabX)
    signal tabDragged(real sceneX)
    signal tabDropped()

    // The mark's seat is given only the air it has not already taken (`spaceSm − markAir`) — otherwise both are spent
    // twice and the gap inside the tab reads wider than the tab's own margins (実測 13px between name and mark against
    // 9px to the edge). Exact fit: anything the layout cannot hand out lands on the right margin, where nobody wrote it
    // down. Rounded up so this and `settleTitleCap` agree on what the tab costs, or the strip scrolls by the fractions
    // they disagree about.
    width: Math.ceil(tabContent.implicitWidth) + tabItem.padW
    height: tabItem.stripHeight
    // Over the tabs it is being carried past: between one neighbour's half and the next one's, the tab in hand covers
    // the tab it has not displaced yet.
    z: tabItem.held ? 1 : 0
    color: tabItem.current ? Theme.bgSelected : "transparent"
    // Drawn where the hand has it rather than where the strip put it. A transform rather than an `x` of its own: the
    // view owns a delegate's place and writes it back at every layout, and this way the two never argue — the offset is
    // read from whatever place the row was given, so the tab stays under the hand across the very moves it is causing.
    transform: Translate {
        id: heldShift
        x: tabItem.held ? tabItem.heldX - tabItem.x : 0
    }
    // Which tab the hand is on (デザイン規約 §タブの所作「`✕` が出るのは前に居るタブと、手の下のタブだけ」). A handler
    // because handlers are passive: the mark, the wash and the tab go on answering the one pointer however many
    // children of this tab take hover of their own.
    HoverHandler {
        id: tabHover
        onHoveredChanged: tabItem.pointed = tabHover.hovered
    }
    // The whole tab answers the middle button; the `✕` does not accept it, so a press on the mark falls through to the
    // same gesture.
    MouseArea {
        id: tabMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        /// Where the press landed in the scene, and whether the hand has since carried the tab off. Both the threshold
        /// and the carrying are measured from the scene: this item moves under the hand, so its own coordinates say
        /// less the further the drag goes.
        property real pressSceneX: 0
        property bool carrying: false
        /// Where inside the tab the hand took hold. Taken at the press rather than at the threshold, so the tab travels
        /// exactly as far as the hand did and not four pixels less.
        property real grabX: 0
        function letGo() {
            if (!tabMouse.carrying)
                return
            tabMouse.carrying = false
            tabItem.tabDropped()
        }
        // Moving to the tab is what a press means: the drag that may follow carries the tab it is about, and a strip
        // that waited for the release would be carrying a tab it had not moved to (デザイン規約 §タブの所作).
        onPressed: mouse => {
            if (mouse.button !== Qt.LeftButton)
                return
            tabItem.tabPressed(mouse.button)
            tabMouse.pressSceneX = tabMouse.mapToItem(null, mouse.x, 0).x
            tabMouse.grabX = mouse.x
            tabMouse.carrying = false
        }
        onPositionChanged: mouse => {
            // Hover comes through here too, and a hand with nothing in it is not carrying anything.
            if (!(mouse.buttons & Qt.LeftButton))
                return
            const sceneX = tabMouse.mapToItem(null, mouse.x, 0).x
            if (!tabMouse.carrying) {
                // The platform's own threshold, the one the view would have stolen the press at (`TabStrip`). Below it
                // the hand is holding still, and a tab that jumped at the first stray pixel would be answering a
                // gesture nobody made.
                if (Math.abs(sceneX - tabMouse.pressSceneX) < tabMouse.drag.threshold)
                    return
                tabMouse.carrying = true
                tabItem.tabTaken(tabMouse.grabX)
            }
            tabItem.tabDragged(sceneX)
        }
        onReleased: tabMouse.letGo()
        onCanceled: tabMouse.letGo()
        onClicked: mouse => {
            if (mouse.button === Qt.MiddleButton)
                tabItem.tabPressed(mouse.button)
        }
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
