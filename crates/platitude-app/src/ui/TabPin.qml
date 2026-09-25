pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The tab in front always stands in the strip: while its row is scrolled off the run, this stand-in rides the edge the
// row went out of, and steps aside once the row is whole on screen (デザイン規約 §タブの所作). The same shape as
// `HeadPinRow` and `GraphHeadPin`.
//
// Declare it beside the list, not in it: a Flickable's declared children travel with the scroll.
Rectangle {
    id: tabPin

    required property var tabsModel
    /// The strip's shared arithmetic and what it handed out among the names (`TabStrip.settleTitleCap`) — the same
    /// values the rows are drawn from.
    required property var metrics
    property real titleCap: 0
    property real titleMinW: 0
    property real titleEaseW: 0
    property real markRoom: 0
    property real fadeW: 0
    /// The tab in front, as the strip's list built it (`TabStrip.frontTab`); everything drawn here is read off it.
    /// Null while no tab is open and for the frame between a tab closing and its neighbour coming forward.
    property Item frontTab: null
    /// The run the tabs are drawn in: where it begins in the strip, how wide it is, and how far it has scrolled.
    required property real runX
    required property real runWidth
    required property real runOffset
    /// Whether a tab is being carried: no stand-in then (デザイン規約 §タブの所作「掴んでいる間は本物だけ」).
    property bool carrying: false
    property real stripHeight: 0

    /// Hover, written down as the tabs do (`TabItemDelegate.pointed`) so a headless run can write it and photograph the
    /// tip (verify-ui スキル §hover の絵の撮り方).
    property bool pointed: false

    /// Pressed: the strip travels to the row this stands for (デザイン規約 §タブの所作).
    signal activated()

    /// The tab in front's own place and width, never re-measured: a pixel off, and the stand-in is a tab the strip does
    /// not have.
    readonly property real seatX: tabPin.frontTab ? tabPin.frontTab.x : 0
    readonly property real seatWidth: tabPin.frontTab ? tabPin.frontTab.width : 0
    /// Which edge the row went out of — and, between the two of them, whether it went out at all.
    readonly property bool rideLeft: tabPin.seatX < tabPin.runOffset
    readonly property bool rideRight: tabPin.seatX + tabPin.seatWidth > tabPin.runOffset + tabPin.runWidth
    readonly property bool frontWhole: tabPin.frontTab !== null && !tabPin.rideLeft && !tabPin.rideRight
    /// What is left of the run on the side this faces, and how far the dark holds over it: `fadeW`
    /// (`TabMetrics.fadeChars`), held inside the run as `x` is.
    readonly property real dissolveRoom: tabPin.rideLeft
        ? tabPin.runX + tabPin.runWidth - (tabPin.x + tabPin.width)
        : tabPin.x - tabPin.runX
    readonly property real dissolveW: Math.max(0, Math.min(tabPin.fadeW, tabPin.dissolveRoom))
    /// What the run beside the tab is taken down to: the window's floor (デザイン規約 §背景), and that floor at zero
    /// alpha — `transparent` is black and would ramp through a colour the strip has not got.
    readonly property color deepGround: Theme.bgBase
    readonly property color deepGone:
        Qt.rgba(tabPin.deepGround.r, tabPin.deepGround.g, tabPin.deepGround.b, 0)
    /// The linked copy the tab in front stands in; empty for the repository's own copy (no second run).
    readonly property string treeName: tabPin.frontTab ? tabPin.frontTab.copy_name : ""
    /// The two runs and the cap spent between them, as the rows settle them (`TabShare.splitName`).
    readonly property real nameNatW: Math.ceil(pinTitle.implicitWidth) + pinTree.naturalWidth
    readonly property var nameSplit: tabPin.metrics.share.splitName(
        Math.ceil(pinTitle.implicitWidth), pinTree.naturalWidth, pinTree.floorWidth, tabPin.titleCap)
    readonly property real titleW: tabPin.nameSplit.titleW
    readonly property real treeW: tabPin.nameSplit.treeW
    /// The air this name is eased with, as the rows are (`TabMetrics.titleEase`), off both runs together.
    readonly property real titleEase:
        tabPin.metrics.titleEase(tabPin.nameNatW, tabPin.titleCap, tabPin.titleEaseW)
    /// Where that air goes, and whether the name still runs under the mark — as the rows settle them
    /// (`TabItemDelegate`).
    readonly property real easeRight: tabPin.metrics.easeRight(tabPin.titleEase, tabPin.markRoom)
    readonly property bool nameUnderMark: tabPin.markRoom + tabPin.easeRight < tabPin.metrics.markRoomFull
    /// Automation: whether any of the name is still drawn, as the rows answer it (`TabItemDelegate.nameKept`).
    readonly property bool nameKept:
        pinTitle.text === "" || pinTitle.headText !== "" || pinTitle.tailText !== ""

    function closeFront() {
        if (tabPin.frontTab)
            tabPin.tabsModel.closeTab(tabPin.frontTab.tab_id)
    }

    visible: tabPin.frontTab !== null && !tabPin.carrying && (tabPin.rideLeft || tabPin.rideRight)
    width: tabPin.seatWidth
    height: tabPin.stripHeight
    // Held inside the run, since nothing here clips and the ☰ is past its edge. The `max` only bites for a tab wider
    // than the run, a layout already gone wrong.
    x: tabPin.rideLeft ? tabPin.runX
                       : Math.max(tabPin.runX, tabPin.runX + tabPin.runWidth - tabPin.width)
    // Dressed as the tab it stands for, and opaque: the strip runs underneath. No rule on its edge
    // (デザイン規約 §タブの所作「縦の境は地が言う」); what the strip does past that edge is `pinDissolve`.
    color: tabPin.frontTab ? tabPin.frontTab.groundColor : Theme.bgSelected
    // The working copy's full path, as the tab says it (デザイン規約 §hover のツールチップ).
    ToolTip.visible: tabPin.pointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: tabPin.frontTab ? tabPin.frontTab.copy_path : ""
    HoverHandler {
        id: pinHover
        onHoveredChanged: tabPin.pointed = pinHover.hovered
    }
    // Left travels to the row; middle closes the tab, as anywhere on a tab. The `✕` takes only the left button, so a
    // middle press on it falls through to here.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        onClicked: mouse => {
            if (mouse.button === Qt.MiddleButton)
                tabPin.closeFront()
            else
                tabPin.activated()
        }
    }
    // The run-facing edge: the tabs travelling under it sink into the window's floor over `dissolveW`
    // (デザイン規約 §タブの所作「隠すのは暗さ」). Drawn outside the tab's box — inside, the floor would be a hole in the
    // tab — which also leaves press and hover to the tab underneath.
    Rectangle {
        id: pinDissolve
        x: tabPin.rideLeft ? tabPin.width : -tabPin.dissolveW
        width: tabPin.dissolveW
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: tabPin.rideLeft ? tabPin.deepGround : tabPin.deepGone }
            GradientStop { position: 1; color: tabPin.rideLeft ? tabPin.deepGone : tabPin.deepGround }
        }
    }
    // The tab's own face, at its margins and cut where it cuts (`TabItemDelegate`).
    CutName {
        id: pinTitle
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: tabPin.metrics.tabPadL + tabPin.titleEase - tabPin.easeRight
        width: tabPin.titleW
        text: tabPin.frontTab ? tabPin.frontTab.title : ""
        letterSpacing: tabPin.metrics.titleTracking(pinTitle.text.length)
        weight: Font.DemiBold
        color: Theme.textPrimary
    }
    TabTreeMark {
        id: pinTree
        anchors.left: pinTitle.right
        anchors.verticalCenter: parent.verticalCenter
        width: tabPin.treeW
        visible: tabPin.treeW > 0
        name: tabPin.treeName
        metrics: tabPin.metrics
        minNameW: tabPin.titleMinW
    }
    // As in the rows; the ground behind the name is always this tab's own.
    TabTitleFade {
        id: pinFade
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: tabPin.metrics.markRoomFull + tabPin.fadeW
        markX: pinFade.width - tabPin.metrics.markGap - tabPin.metrics.markSeat / 2
        markR: tabPin.metrics.markSeat / 2
        rampW: tabPin.fadeW
        ground: tabPin.color
        visible: tabPin.nameUnderMark
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 2 * Theme.borderWidth
        color: tabPin.frontTab ? tabPin.frontTab.ruleColor : Theme.accent
    }
    // The tab in front always shows its mark (デザイン規約 §タブの所作).
    CloseToolButton {
        id: pinMark
        anchors.right: parent.right
        anchors.rightMargin: tabPin.metrics.markGap
        anchors.verticalCenter: parent.verticalCenter
        implicitWidth: tabPin.metrics.markSeat
        topInset: (Theme.iconLg - tabPin.metrics.markSeat) / 2
        bottomInset: pinMark.topInset
        // Nothing sideways, for the reason the rows' mark carries.
        leftInset: 0
        rightInset: 0
        onClicked: tabPin.closeFront()
    }
}
