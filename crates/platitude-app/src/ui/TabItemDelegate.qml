pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One tab in the strip: the repository's name, the mark that closes it, and the two washes that say which tab is in
// front and which one the pointer is on (デザイン規約 §タブの所作).
Rectangle {
    id: tabItem
    required property int index
    required property int tab_id
    required property string title
    /// The worktree this tab stands in: the path the hover puts out, and — for a linked worktree — its folder name,
    /// drawn after the title (デザイン規約 §タブの所作). The name is empty in the repository's own worktree, which draws
    /// no run and wears the band's ordinary blue.
    required property string worktree_path
    required property string worktree_name
    /// The strip's own model: the one question a tab asks of it, and the one thing the mark does to it.
    required property var tabsModel
    /// What every name in the strip is capped at right now, the shortest one is cut to, the length a name stops being
    /// eased at, the room every tab keeps for its mark, and the run a name goes quiet over where the mark stands on
    /// it — all settled in the same pass, off the same run (`TabStrip.settleTitleCap`).
    property real titleCap: 0
    property real titleMinW: 0
    property real titleEaseW: 0
    property real markRoom: 0
    property real fadeW: 0
    /// The ground the band paints behind this tab (`TabStrip.bandColor`) — what the name goes quiet into on every tab
    /// but the one in front, which paints its own.
    required property color bandColor
    /// The strip's shared arithmetic (`TabMetrics`): what a tab costs, the seat its mark stands in, and how a short
    /// name is eased. One object, so the strip and the tab cannot disagree.
    required property var metrics
    /// The name and the worktree's run (`TabTreeMark`) uncut — one number against the strip's one cap, which the tab
    /// splits between the two (`nameSplit`).
    readonly property real nameNatW: Math.ceil(tabTitle.implicitWidth) + treeRun.naturalWidth
    /// How it is spent: the name first, the worktree's run out of what is left, and that run gone whole before a
    /// letter of the name is cut (`TabShare.splitName`).
    readonly property var nameSplit: tabItem.metrics.share.splitName(
        Math.ceil(tabTitle.implicitWidth), treeRun.naturalWidth, treeRun.floorWidth, tabItem.titleCap)
    readonly property real titleW: tabItem.nameSplit.titleW
    readonly property real treeW: tabItem.nameSplit.treeW
    /// The air this name is eased with (`TabMetrics.titleEase`), read off both runs whole — the cap is a maximum on the
    /// item and does not move it.
    readonly property real titleEase:
        tabItem.metrics.titleEase(tabItem.nameNatW, tabItem.titleCap, tabItem.titleEaseW)
    /// How much of that air goes on the mark's side (`TabMetrics.easeRight`), and whether the name still runs on under
    /// the mark (for the fade).
    readonly property real easeRight: tabItem.metrics.easeRight(tabItem.titleEase, tabItem.markRoom)
    readonly property bool nameUnderMark: tabItem.markRoom + tabItem.easeRight < tabItem.metrics.markRoomFull
    property real stripHeight: 0
    readonly property bool current: tabItem.tabsModel.currentIndex === tabItem.index
    /// Whether the pointer is on this tab — the one property both the real hover and the smoke hook write, so the
    /// headless run proves what the wash and the mark answer.
    property bool pointed: false
    /// Automation: whether the mark is out, read off the mark itself — the output side, so a cut binding cannot pass.
    readonly property real markShown: closeMark.opacity
    /// Automation: how far this tab is drawn from its row, read off the transform as `markShown` is off the mark
    /// (`PGG_AUTO_ACT=tab-hold`).
    readonly property real shiftShown: heldShift.x
    /// Automation: whether any of this name is still drawn, read off the label's halves (the handed name is always
    /// whole). A tab crushed to its mark looks in a picture like a short name in a crowded strip
    /// (`TabProbe.tabNamesCrushed`).
    readonly property bool nameKept:
        tabItem.title === "" || tabTitle.headText !== "" || tabTitle.tailText !== ""
    /// Whether this is the tab in hand, and where its left edge has been carried to — both settled by the strip's
    /// `TabCarry`, since the order changes under a drag.
    property bool held: false
    property real heldX: 0
    // The tab in hand takes its tip back, a box already out included (`TipAsk.drop`) — at the drag threshold, where
    // `held` rises. A press alone changes nothing: Qt hands out no hover while a press holds the grab, so the window's
    // `HandWatch` stays where the press landed (`tst_carriedtabtip.qml`).
    onHeldChanged: if (tabItem.held) pathTip.drop()
    /// Pressed with a button the tab answers. Which button means what is the strip's to say, since the same rule is
    /// what the middle-click hook comes through (`TabStrip.pressTab`).
    signal tabPressed(int button)
    /// Taken up (`grabX` = where inside the tab the hand took hold), dragged and set down. The drag reports in scene
    /// coordinates: by then the tab is drawn away from its own place.
    signal tabTaken(real grabX)
    signal tabDragged(real sceneX)
    signal tabDropped()
    /// This tab has become the one in front, or stopped being it. Pushed, because the strip's stand-in (`TabPin`) draws
    /// off whichever item answers true, and the item for a row just gained arrives only with the next layout (as
    /// `TabStrip.middleClickTab` notes).
    signal frontChanged(bool front)

    // Name, worktree run, near step, mark room and easing. The name is rounded up so this and `settleTitleCap` agree on
    // what a tab costs, or the strip scrolls by the fractions they disagree about.
    width: tabItem.titleW + tabItem.treeW
        + tabItem.metrics.tabPadL + tabItem.markRoom + tabItem.titleEase
    height: tabItem.stripHeight
    // Over the tabs it is carried past, which it covers until it has taken half of one.
    z: tabItem.held ? 1 : 0
    // The tab in front paints its own ground, which says which worktree it stands in (デザイン規約 §タブの所作): the
    // ground and the underline are what say "in front", so their hue doubles as "where it stands", and only the tab in
    // front wears it — the worktree's run (`treeRun`) stands on every tab. `bgHereTree` is as dark as the band, so in
    // a linked worktree "in front" rests on the underline and `fontWeightStrong`; the ground adds the hue alone.
    color: tabItem.current ? tabItem.groundColor : "transparent"
    /// What the tab in front is painted in, and the line along its bottom edge (デザイン規約 §ref の種別). Read by the
    /// fade over the mark and by the stand-in as well, so a strip that has scrolled says one thing (`TabPin`): a
    /// stand-in in another colour is the band drawing one tab twice, and a fade on another ground brings the name
    /// back out of a colour the tab has not got.
    readonly property color groundColor: tabItem.worktree_name === "" ? Theme.bgSelected : Theme.bgHereTree
    readonly property color ruleColor: tabItem.worktree_name === "" ? Theme.accent : Theme.textHereTree
    // Drawn where the hand has it, as a transform: the view owns a delegate's place and rewrites it at every layout,
    // and an offset from that place keeps the tab under the hand across the very moves it is causing.
    transform: Translate {
        id: heldShift
        x: tabItem.held ? tabItem.heldX - tabItem.x : 0
    }
    // The full path under the hand, always — of the worktree the tab stands in now (user decision — ask before
    // changing): the name already says the repository, so which of its worktrees is what only the path can tell. The
    // path alone: what a tab does is the gesture table's to say (デザイン規約 §タブの所作). Read off `pointed` like the
    // mark so the headless run reaches it the same way (`TabProbe.pointAtTab`). Nothing opens under a carrying hand,
    // and the box already out goes as the tab is taken up (`TabCarry`): that hand is busy carrying, and a box
    // beside a moving tab is there for nobody to read. The ask falling here is no hand walking into the box, which the
    // seat is told by the drop above. `TabPin` puts out the same path; `repo_path` is the name's and the tab's identity
    // and is never shown. Headless: `tab-name`.
    readonly property TipAsk pathTip: TipAsk {
        id: pathTip
        target: tabItem
        asked: tabItem.pointed && !tabItem.held
        words: tabItem.worktree_path
    }
    onCurrentChanged: tabItem.frontChanged(tabItem.current)
    // A tab born in front never announces `current` changing, and the stand-in would have nothing to draw from.
    Component.onCompleted: if (tabItem.current) tabItem.frontChanged(true)
    // Which tab the hand is on — the one tab besides the front one that shows its `✕` (`closeMark`). A handler:
    // the `✕` takes hover of its own, and a tab losing hover there drops the mark from under the hand
    // (rules-refs/app-ui.md「行の hover は `HoverHandler`」).
    HoverHandler {
        id: tabHover
        onHoveredChanged: tabItem.pointed = tabHover.hovered
    }
    // The whole tab answers the middle button; the `✕` does not accept it, so a press on the mark falls through to the
    // same gesture. **The target for closing is the whole tab, the `✕` a mark standing in it**: a gesture aimed by the
    // tab's width that made the hand hit a 16px mark would leave nothing but two buttons. It closes on the release, as
    // a click: a press carried off the tab before letting go does nothing — no question is asked, so a way back stays.
    MouseArea {
        id: tabMouse
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
        /// Where the press landed in the scene — this item moves under the hand — and whether the hand has since
        /// carried the tab off.
        property real pressSceneX: 0
        property bool carrying: false
        /// Where inside the tab the hand took hold, at the press, so the tab travels exactly as far as the hand did.
        property real grabX: 0
        function letGo() {
            if (!tabMouse.carrying)
                return
            tabMouse.carrying = false
            tabItem.tabDropped()
        }
        // A press moves to the tab (a close waits for the release — above): the drag that may follow carries the tab
        // already in front. Carrying a tab not in front would leave only the seat's colour to say which one is in hand.
        onPressed: mouse => {
            if (mouse.button !== Qt.LeftButton)
                return
            tabItem.tabPressed(mouse.button)
            tabMouse.pressSceneX = tabMouse.mapToItem(null, mouse.x, 0).x
            tabMouse.grabX = mouse.x
            tabMouse.carrying = false
        }
        onPositionChanged: mouse => {
            // Only the left button carries; a middle-button drag comes through here too.
            if (!(mouse.buttons & Qt.LeftButton))
                return
            // The pointer's true scene x even while this item rides the `Translate` it is moving (measured in a
            // throwaway qmltestrunner scene; no headless verb reaches a real pointer).
            const sceneX = tabMouse.mapToItem(null, mouse.x, 0).x
            if (!tabMouse.carrying) {
                // The platform's own threshold, the one the view would have stolen the press at (`TabStrip`): below
                // it the hand is holding still. `drag.threshold` equals `Qt.styleHints.startDragDistance` with no
                // `drag.target` written (measured as above).
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
    // The name, a dense row's step in from the near edge (デザイン規約 §余白「高密度な行の内側のみ 4」), with a short
    // name's easing on either side of the name — the mark keeps its own step off the far edge. Cut in the middle
    // through `CutName` (user decision — ask before changing): a repository is named by its last folder, and a name
    // grown beside a namesake (`tab_name`) is told apart at both ends — the added part is exactly where the two
    // differ, the end is the folder itself. A bare `elide` leaves its remainder at the far edge, so capped names would
    // stop at different distances from their marks (規約 §寸法「余りを切れ目へ入れる」). The whole path is the hover's.
    // **Every name is `textPrimary`**: which tab is in front is said by its seat (ground, underline, weight), so the
    // letters need not say it; the `✕` is a step quieter (`textSecondary`).
    CutName {
        id: tabTitle
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: tabItem.metrics.tabPadL + tabItem.titleEase - tabItem.easeRight
        // What the split left the name (`nameSplit`), not what the margins leave: the worktree's run stands between
        // this and the mark.
        width: tabItem.titleW
        text: tabItem.title
        // The other half of the easing: a short name's letters set a little apart. Off the letter count, since the
        // air is computed from the width (`TabMetrics.titleTracking`).
        letterSpacing: tabItem.metrics.titleTracking(tabItem.title.length)
        weight: tabItem.current ? Theme.fontWeightStrong : Font.Normal
        color: Theme.textPrimary
    }
    // The linked worktree this tab stands in, after its name, on every such tab in front or not (デザイン規約 §タブの所作).
    TabTreeMark {
        id: treeRun
        anchors.left: tabTitle.right
        anchors.verticalCenter: parent.verticalCenter
        width: tabItem.treeW
        visible: tabItem.treeW > 0
        name: tabItem.worktree_name
        gap: tabItem.metrics.treeRunGap
        minNameW: tabItem.titleMinW
    }
    // The name going quiet where the mark stands over it (`TabTitleFade`), in the tab's own ground with the
    // hover wash folded in. Declared between the name and the underline, which goes over everything.
    TabTitleFade {
        id: titleFade
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: tabItem.metrics.markRoomFull + tabItem.fadeW
        // The mark's centre and the disc its wash paints: the same circle, or the name comes back on an edge the wash
        // has not got.
        markX: titleFade.width - tabItem.metrics.markGap - tabItem.metrics.markSeat / 2
        markR: tabItem.metrics.markSeat / 2
        rampW: tabItem.fadeW
        ground: tabItem.current
            ? tabItem.groundColor
            : (tabItem.pointed ? Qt.tint(tabItem.bandColor, Theme.bgHover) : tabItem.bandColor)
        opacity: closeMark.opacity
        // Nothing to quieten while the name stops a whole step short of the mark, as a short one does by spending its
        // eased air on that side (`easeRight`).
        visible: tabItem.nameUnderMark
    }
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 2 * Theme.borderWidth
        color: tabItem.ruleColor
        visible: tabItem.current
    }
    // Shown on the tab in front and under the pointer only: a mark on every tab has every seat no hand came to asking
    // to be pressed, and the eye counts the marks before it reads the names it came for. By opacity: out of the layout
    // its seat would go too, and the tab would narrow under the hand that came to close it. Against the tab's far edge
    // by anchors, seated off its ink (`TabMetrics.markGap`): the room in front of it is the first thing a crowded strip
    // takes back (`TabStrip.settleTitleCap`), so in a row with the name the mark would move by what that room gives
    // back and stand at a different distance from each tab's edge. `markRoom` moves the name's far margin only. "Under
    // the pointer" is `tabHover`; `tabMouse` declares no `hoverEnabled` — once a press holds the grab, its moves bring
    // the threshold and the carry. No verb guards this wiring: `tab-mark` writes `pointed` straight
    // (`TabProbe.pointAtTab`) and stays green with it cut; hover cannot be injected, so only a throwaway qmltestrunner
    // scene measures it.
    CloseToolButton {
        id: closeMark
        anchors.right: parent.right
        anchors.rightMargin: tabItem.metrics.markGap
        anchors.verticalCenter: parent.verticalCenter
        // Width held to the mark's own box (デザイン規約 §寸法): the usual `iconLg` seat would hang over the next tab
        // and eat its hover. The height stays `iconLg` so a hand coming down the strip lands on it, and the wash is
        // inset back to the ink (§当たり判定, as `TabStrip`'s `+`).
        implicitWidth: tabItem.metrics.markSeat
        // Drawn at the seat it is measured at (`TabMetrics.closeMark`): a step above the other closing marks.
        markSize: tabItem.metrics.markSeat
        topInset: (Theme.iconLg - tabItem.metrics.markSeat) / 2
        bottomInset: closeMark.topInset
        // Written out: the shared button binds `leftInset: topInset`, which here would narrow the wash below the ink.
        leftInset: 0
        rightInset: 0
        opacity: tabItem.current || tabItem.pointed ? 1 : 0
        onClicked: tabItem.tabsModel.closeTab(tabItem.tab_id)
    }
}
