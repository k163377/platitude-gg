pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The tab in front never leaves the strip: while its own row is scrolled off the run, this stand-in rides the edge the
// row went out of, and it steps aside the moment that row is whole on screen — so the strip never shows one tab twice
// (デザイン規約 §タブの所作). The sidebar answers the same question about the current branch in the same shape
// (`HeadPinRow`), and the graph does about the commit HEAD stands on (`GraphHeadPin`).
//
// Whoever uses this stands it beside the list rather than declaring it inside: a Flickable's declared children are
// taken by its content item and travel with the scroll, which is the one thing a stand-in may not do.
Rectangle {
    id: tabPin

    /// The strip's own model — the one thing the mark does goes through it.
    required property var tabsModel
    /// The strip's shared arithmetic and the run it handed out among the names (`TabStrip.settleTitleCap`). The same
    /// three the rows are drawn from, so the stand-in cannot be set in a measure the strip is not using.
    required property var metrics
    property real titleCap: 0
    property real titleEaseW: 0
    property real markRoom: 0
    property real fadeW: 0
    /// The tab in front, as the strip's list built it (`TabStrip.frontTab`). Everything drawn here is read off that
    /// item — where it sits, how wide it came out, what it is called, what it stands on — so the stand-in cannot say
    /// anything the tab is not saying. Null while no tab is open, and null again for the frame between a tab closing
    /// and its neighbour coming forward.
    property Item frontTab: null
    /// The run the tabs are drawn in: where it begins in the strip, how wide it is, and how far it has travelled.
    required property real runX
    required property real runWidth
    required property real runOffset
    /// Whether a tab is under a hand. Nothing stands in for one: a carry draws its tab against the edge of the run
    /// whatever row it has got to (`TabCarry.carryTab`), so the row's own seat stops saying where the reader can see
    /// it — and the tab in hand is the tab in front (デザイン規約 §タブの所作「移るのは押した時」).
    property bool carrying: false
    property real stripHeight: 0

    /// Where the pointer's arrival is written down, in the shape the tabs already use (`TabItemDelegate.pointed`).
    /// The tip below reads this rather than the handler, so the seat a headless run would write to reach it is cut
    /// before there is a verb that wants it — hover is the one input no run can make, and a tip wired to a handler
    /// has to be rewired before it can ever be photographed (verify-ui スキル §hover の絵の撮り方).
    property bool pointed: false

    /// Pressed: the strip is asked to travel to the row this stands for. The one thing a press on a stand-in can mean
    /// — the same answer the graph's own gives (`GraphHeadPin`; デザイン規約 §タブの所作).
    signal activated()

    /// Where the tab in front sits in the strip's content and how wide it came out. Nothing is measured again here:
    /// a stand-in a pixel off the tab it stands for is a tab the strip does not have.
    readonly property real seatX: tabPin.frontTab ? tabPin.frontTab.x : 0
    readonly property real seatWidth: tabPin.frontTab ? tabPin.frontTab.width : 0
    /// Which edge the row went out of — and, between the two of them, whether it went out at all.
    readonly property bool rideLeft: tabPin.seatX < tabPin.runOffset
    readonly property bool rideRight: tabPin.seatX + tabPin.seatWidth > tabPin.runOffset + tabPin.runWidth
    readonly property bool frontWhole: tabPin.frontTab !== null && !tabPin.rideLeft && !tabPin.rideRight
    /// What is left of the run on the side this faces into, and how far the dark holds over it. **The three characters
    /// the name is already given under the mark** (`TabMetrics.fadeChars`): the strip measures its dissolves in
    /// letters, and a band holding two lengths of one reads as two materials (デザイン規約 §タブの所作).
    /// Never past the far end of the run — the ☰ and the `+` stand the other side of those, and nothing here clips.
    readonly property real dissolveRoom: tabPin.rideLeft
        ? tabPin.runX + tabPin.runWidth - (tabPin.x + tabPin.width)
        : tabPin.x - tabPin.runX
    readonly property real dissolveW: Math.max(0, Math.min(tabPin.fadeW, tabPin.dissolveRoom))
    /// What the run beside the tab is taken down to: the window's own floor, a step under the band the tabs are drawn
    /// on (デザイン規約 §背景). And that same floor at none of itself, which is what it lets go to — **not
    /// `transparent`**, which is a transparent black and takes the ramp out through a colour the strip has not got
    /// (`TabTitleFade`; §色).
    readonly property color deepGround: Theme.bgBase
    readonly property color deepGone:
        Qt.rgba(tabPin.deepGround.r, tabPin.deepGround.g, tabPin.deepGround.b, 0)
    /// The air this name is eased with, the same half-of-the-shortfall the rows are given
    /// (`TabMetrics.titleEase`). Read off the label's own hint, which is the name at its natural width.
    readonly property real titleEase:
        tabPin.metrics.titleEase(pinTitle.implicitWidth, tabPin.titleCap, tabPin.titleEaseW)
    /// Where that air is set down, and whether the name still runs on under the mark once it has been spent — both as
    /// the rows settle them (`TabItemDelegate`).
    readonly property real easeRight: tabPin.metrics.easeRight(tabPin.titleEase, tabPin.markRoom)
    readonly property bool nameUnderMark: tabPin.markRoom + tabPin.easeRight < tabPin.metrics.markRoomFull
    /// Automation: whether any of the name is still drawn, as the rows answer it (`TabItemDelegate.nameKept`). The
    /// stand-in is the one tab a run photographs in close-up, and a name cut away to the mark is the one way it can
    /// say something the tab it stands for is not saying while still being its width.
    readonly property bool nameKept:
        pinTitle.text === "" || pinTitle.headText !== "" || pinTitle.tailText !== ""

    /// The tab this stands for, closed. The whole of it is the target, the way the whole of a tab is
    /// (デザイン規約 §タブの所作「閉じる的はタブ全体」).
    function closeFront() {
        if (tabPin.frontTab)
            tabPin.tabsModel.closeTab(tabPin.frontTab.tab_id)
    }

    visible: tabPin.frontTab !== null && !tabPin.carrying && (tabPin.rideLeft || tabPin.rideRight)
    width: tabPin.seatWidth
    height: tabPin.stripHeight
    // Never before the run begins: the ☰ is the other side of that edge, and nothing in this file clips. The strip
    // hands the run out among the names before it lets one tab past the width of it (`TabStrip.settleTitleCap`), so
    // this only holds for a layout that has already gone wrong — and it goes wrong towards the edge the stand-in
    // takes when the row leaves the other way, rather than over a mark that has nothing to do with tabs.
    x: tabPin.rideLeft ? tabPin.runX
                       : Math.max(tabPin.runX, tabPin.runX + tabPin.runWidth - tabPin.width)
    // Dressed as the tab it stands for: the tab in front's own ground, its underline below, its weight in the name.
    // Opaque, because the strip runs underneath. **No line down the side it runs under** — the sidebar's stand-in
    // draws one because its ground and its rows' are a step apart, and here the two are different things entirely
    // (a filled tab against transparent ones). A rule in the band would be the strongest ink in it for as long as the
    // strip is scrolled, which is the reason the graph's stand-in has none either (規約 §グラフの中で HEAD を見失わない).
    // It ends with the tab, and what the strip does on the other side of that edge is `pinDissolve` below.
    color: Theme.bgSelected
    // The repository in full, as the tab it stands for says it (デザイン規約 §hover のツールチップ). Read off `pointed`
    // for the reason the tabs read it, and it is the stand-in's own: a tip belongs to the thing under the hand.
    ToolTip.visible: tabPin.pointed
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: tabPin.frontTab ? tabPin.frontTab.repo_path : ""
    HoverHandler {
        id: pinHover
        onHoveredChanged: tabPin.pointed = pinHover.hovered
    }
    // The left button takes the reader to the row; the middle one closes the tab, as it does anywhere on a tab. The
    // `✕` does not accept the middle button, so a press on the mark falls through to the same gesture.
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
    // Where the strip goes under it. An opaque edge cuts whichever tab is beneath it in two — a name broken mid-word,
    // or the two pixels of one that read as a tab with no padding at all — and this tab's ground cannot say otherwise:
    // a filled tab against transparent ones says which of them is on top, not that the one underneath carries on.
    // **So the run beside the tab is taken down**: the floor stands against the tab's edge and lets go over three
    // characters, so a tab travelling under this one sinks past the strip's ground into the dark and comes back out of
    // it (デザイン規約 §タブの所作). The graph's stand-in lays the same thing downwards, and a step in the tab
    // underneath would say "it ended here" (§省略の表し方).
    //
    // **No blue leaves the tab — not as a spread and not as a ramp.** The whole of a tab is what a press is aimed at
    // (§タブの所作「閉じる的はタブ全体」), and the tab's own ground is what says where that is, so blue on the far side
    // of the edge puts the target's edge where the reader cannot find it. What is outside is dark, and only dark.
    // **The dark is a token rather than a new one** (`bgBase`, the floor everything in the window stands on;
    // §暗く落とした段「新しい暗さを発明しない」).
    //
    // **Only the edge that faces the run.** The other one stands on the list's own clip, and no tab is drawn past it.
    // **Outside the tab's box, never into it**: the face is drawn at the width the strip handed that tab
    // (§タブの所作「幅を測り直さない」), and the floor laid inside it would be a hole in the tab. That is also what
    // keeps it clear of the quiet under the mark (`pinFade`) — one hides this tab's own tail, the other the tabs behind
    // it, and they meet at the edge rather than lie over each other. Outside the box is outside the press and the hover
    // as well (both are the item's own), so what is under the dissolve is a tab of the strip's, answering for itself.
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
    // The tab's own face, at the tab's own margins and cut where the tab cuts (`TabItemDelegate` carries what each part
    // of it is for) — a stand-in whose name broke in another place would be a tab the strip has not got.
    CutName {
        id: pinTitle
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: tabPin.metrics.tabPadL + tabPin.titleEase - tabPin.easeRight
        anchors.rightMargin: tabPin.markRoom + tabPin.easeRight
        text: tabPin.frontTab ? tabPin.frontTab.title : ""
        letterSpacing: tabPin.metrics.titleTracking(pinTitle.text.length)
        weight: Font.DemiBold
        color: Theme.textPrimary
    }
    // The name goes quiet under the mark on the same terms the rows do. One ground rather than the rows' two: this one
    // is always the tab in front, so what is behind its name is its own.
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
        color: Theme.accent
    }
    // The tab in front always has its mark out (デザイン規約 §タブの所作), and the room it stands in is part of what that
    // tab costs — a stand-in drawn without it would be the same width with a hole at the end of it.
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
