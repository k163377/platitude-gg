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
    /// What every name in the strip is capped at right now, and the length a name stops being eased at — both settled
    /// in the same pass, off the same run (`TabStrip.settleTitleCap`).
    property real titleCap: 0
    property real titleEaseW: 0
    /// The strip's shared arithmetic (`TabMetrics`): what a tab costs, the seat its mark stands in, and how a short
    /// name is eased. One object rather than a copy of each number, so the strip and the tab cannot disagree.
    required property var metrics
    /// The air this name is eased with, half of what it falls short by (`TabMetrics.titleEase`). Read off the label's
    /// own hint, which is the name at its natural width — the cap is a maximum on the item and does not move it.
    readonly property real titleEase:
        tabItem.metrics.titleEase(tabTitle.implicitWidth, tabItem.titleCap, tabItem.titleEaseW)
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
    /// This tab has become the one in front, or has stopped being it. The strip's stand-in is drawn off whichever item
    /// answers true (`TabPin`), and it is pushed rather than looked up: the item for a row the model has only just
    /// gained arrives with the next layout, so a strip that went looking the moment the front changed would find
    /// nothing standing there (`TabStrip.middleClickTab` carries the same note).
    signal frontChanged(bool front)

    // The name, the two margins, and the half of the easing that falls outside the row (`tabContent` carries the other
    // half in its spacing). Exact fit: anything the layout cannot hand out lands on the right margin, where nobody
    // wrote it down. Rounded up so this and `settleTitleCap` agree on what the tab costs, or the strip scrolls by the
    // fractions they disagree about.
    width: Math.ceil(tabContent.implicitWidth) + tabItem.metrics.tabPadW + tabItem.titleEase / 2
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
    // The repository in full, under the hand (デザイン規約 §hover のツールチップ「タブも同じで、hover が必ずリポジトリの
    // フルパスを言う」). Read off `pointed` like the mark, so the one property the real hover writes is what puts the
    // words out — and the headless run reaches them the same way it reaches the mark (`TabProbe.pointAtTab`).
    //
    // Nothing new comes out under a hand that is carrying: by then the hand is doing something else, and a box opening
    // beside a tab in motion is not there to be read (同§「掴んだ手の下では新しく出さない」). Only the new one — a tip
    // already standing is the shared instance's to take down, and it keeps one up while the pointer is on the target
    // it came out of (`SharedToolTip.wanted`), which a tab under a carrying hand still is (P3-確認事項).
    ToolTip.visible: tabItem.pointed && !tabItem.held
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: tabItem.repo_path
    onCurrentChanged: tabItem.frontChanged(tabItem.current)
    // The strip's first tab comes up already in front, and a property that was true from the start never announces
    // itself — the stand-in would have nothing to draw from until the reader moved to some other tab and back.
    Component.onCompleted: if (tabItem.current) tabItem.frontChanged(true)
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
        // A tab is a dense row, and its step is the dense one (デザイン規約 §余白「高密度な行の内側のみ 4」;
        // 2026-08-23 ユーザー指示 = the `spaceSm` step it had was read as too much air on both sides of the name).
        //
        // The mark's two sides are seated off its ink rather than off its box (`TabStrip.markGap`), so all three gaps
        // in a tab are the one step: the name from the near edge, the mark from the name, the far edge from the mark.
        // A spacing of nothing is what the wider step could afford — there the box's own air was already the whole gap
        // — and it is what left the mark sitting nearer both its neighbours than the name sat to the tab's edge.
        //
        // A short name's easing goes on either side of the **name** rather than at the tab's two edges: the mark keeps
        // its own step off the far edge whatever the name does, so what opens up is the room the name is set in
        // (2026-08-23 ユーザー指示 — the seat every short name was padded out to made a row of equal blanks).
        anchors.leftMargin: Theme.spaceXs + tabItem.titleEase / 2
        anchors.rightMargin: tabItem.metrics.markGap
        spacing: tabItem.metrics.markGap + tabItem.titleEase / 2
        Label {
            id: tabTitle
            text: tabItem.title
            elide: Text.ElideRight
            // The cap the whole strip shares; capping the hint is what narrows the tab.
            Layout.maximumWidth: tabItem.titleCap
            // The other half of the easing: a short name is set with its letters a little apart, so the air it is
            // given belongs to the word rather than standing beside it. Off the letter count, never off the width —
            // the width is what the air is computed from (`TabMetrics.titleTracking`).
            font.letterSpacing: tabItem.metrics.titleTracking(tabItem.title.length)
            Layout.fillHeight: true
            verticalAlignment: Text.AlignVCenter
            font.weight: tabItem.current ? Font.DemiBold : Font.Normal
            color: Theme.textPrimary
        }
        // Shown on the tab in front and under the pointer (デザイン規約 §タブの所作). Dimmed rather than dropped: an item the
        // layout has stopped seeing takes its width with it, and the tab would change size under the hand that came to
        // close it.
        CloseToolButton {
            id: closeMark
            Layout.alignment: Qt.AlignVCenter
            // Narrower than the `iconLg` seat this mark stands in everywhere else (デザイン規約 §寸法). A seat is air the
            // layout cannot see past: the `iconLg` one carried `(iconLg − iconSm) / 2` on each side, so the name and
            // the tab's own edge were held further out than the margins beside them said, and at this step no margin
            // could take it back without pushing the seat over the tab beside it. Cut to the mark's own box, the air
            // left over is small enough for `markGap` to spend the rest and land the ink a whole step from both.
            //
            // Only the width comes in. The seat stays `iconLg` tall so a hand coming down the strip still lands on the
            // mark, and the wash is inset back to a box on the ink — 広げるのは判定だけ (§当たり判定; 手本 `TabStrip`'s `+`).
            implicitWidth: tabItem.metrics.markSeat
            topInset: (Theme.iconLg - tabItem.metrics.markSeat) / 2
            bottomInset: closeMark.topInset
            opacity: tabItem.current || tabItem.pointed ? 1 : 0
            onClicked: tabItem.tabsModel.closeTab(tabItem.tab_id)
        }
    }
}
