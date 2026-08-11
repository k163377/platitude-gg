pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Top toolbar: prominent tabs and the per-repository controls share
// one row. `curPage` is the active RepoPage; its transient state
// (operation badge, conflicts, errors, push shape) surfaces here.
Rectangle {
    id: topBar

    required property var tabsModel
    // The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether this band is the window's title bar. When it is, the band
    /// carries the window's own buttons, and tells the platform where
    /// its empty run sits — the gestures on that run (drag, snap, the
    /// double-click, the window menu) are the platform's own, because
    /// the hit test calls it caption (`AppBackend.setCaptionStrip`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the
    /// window's own buttons, or the run of band left to take hold of, or
    /// push either off the end — and none of that shows in a screenshot
    /// taken where the platform draws no buttons at all.
    readonly property real bandGrabRun: tabs.grabRun
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    /// What the strip itself came out to. A broken term in its width
    /// expression turns it NaN, and NaN draws as "no tabs at all" with
    /// nothing on stderr — this is the number that catches it.
    readonly property real bandTabsWidth: tabs.width
    /// Automation: the run the tabs were handed, and what they made of it.
    /// A picture cannot say which tabs gave way and which were left alone
    /// — every strip that fits looks like every other one — so the widths
    /// themselves are the answer (`PG_AUTO_ACT=tab-widths`).
    readonly property int bandTabCount: tabs.count
    readonly property real bandTabRun: tabs.runAvail
    readonly property real bandTabContent: tabs.contentWidth
    readonly property bool bandTabScrolls: tabs.contentWidth > tabs.width
    /// Automation: what the command log's mark came out to — the band's
    /// own reading of the log and the error line together, and the colour
    /// it painted from it. Asked here rather than of the page, because
    /// what is being checked is that the page's news reached this mark
    /// at all (`PG_AUTO_ACT=commands-clear`).
    readonly property bool commandsWrong: commandsToggle.wrong
    readonly property color commandsMarkColor: commandsMark.color
    /// The three things this repository can be in the middle of. All three
    /// can stand at once — a stopped operation that hit a conflict, on a
    /// machine that has never been told who is committing — and as words
    /// beside the tabs that was the widest this band ever got, so they are
    /// folded into one mark and opened as a card (`BandStateCard`).
    ///
    /// One expression each, read by the mark and by the card: written
    /// twice, the two could disagree about whether there is anything here
    /// to open.
    readonly property var stateWt: topBar.curPage !== null
                                   ? topBar.curPage.pageWt : null
    readonly property bool opBadgeShown: topBar.stateWt !== null
                                         && topBar.stateWt.opText !== ""
    readonly property bool conflictBadgeShown: topBar.stateWt !== null
                                               && topBar.stateWt.hasConflicts
    /// A save whose halves did not both land leaves an identity that *is*
    /// set and not the one that was asked for, so nothing else on screen
    /// would mention it.
    readonly property bool identityBadgeShown:
        AppBackend.identityState === "missing"
        || AppBackend.identityUnsaved
        || (topBar.curPage !== null
            && !topBar.curPage.pageTab.identityReady)
    readonly property bool stateShown: topBar.opBadgeShown
                                       || topBar.conflictBadgeShown
                                       || topBar.identityBadgeShown
    /// The narrowest a badge is drawn before the group gives up on words:
    /// its opening letters and the ellipsis that says the rest was cut.
    /// Counted in characters rather than pixels, for the reason the tab
    /// names are (規約 §ウィンドウの縁) — three characters cost a
    /// different number of pixels in each platform's UI font.
    readonly property int stateMinChars: 3
    readonly property real stateBadgeMinW:
        Math.ceil(stateFont.advanceWidth("…")
                  + topBar.stateMinChars * stateFont.averageCharacterWidth)
        + 2 * Theme.spaceXs
    FontMetrics {
        id: stateFont
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
    }

    /// The three badges at their natural width, measured off labels that
    /// are never drawn.
    ///
    /// The badges in the band cannot also be what the cap is measured
    /// from. A `RowLayout` that is not being laid out reports the width it
    /// had when it last was, and the row of badges goes away the moment
    /// the group folds — so a cap read from there makes the fold one that
    /// nothing comes back from (measured 2026-08-11: `cap=32` with a
    /// 1440-wide window, and no width would bring the words back).
    component BadgeWord: Label {
        visible: false
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
    }
    BadgeWord {
        id: mOpText
        text: topBar.stateWt !== null ? topBar.stateWt.opText : ""
    }
    BadgeWord {
        id: mOpAlso
        text: topBar.stateWt !== null ? topBar.stateWt.opAlso : ""
    }
    BadgeWord {
        id: mOpStep
        text: topBar.stateWt === null ? ""
              : qsTr("%1/%2").arg(topBar.stateWt.opStep)
                             .arg(topBar.stateWt.opSteps)
    }
    BadgeWord {
        id: mConflict
        text: qsTr("CONFLICTS")
    }
    BadgeWord {
        id: mIdentity
        text: qsTr("SET IDENTITY")
    }
    DotMark {
        id: mDot
        visible: false
    }
    readonly property bool stateHasAlso: topBar.stateWt !== null
                                         && topBar.stateWt.opAlso !== ""
    readonly property bool stateHasStep: topBar.stateWt !== null
                                         && topBar.stateWt.opSteps > 0
    /// Whole pixels, for the reason the tab names are settled in them
    /// (規約 §ウィンドウの縁): a word asks for a fractional width, a box
    /// is laid out on a whole one, and a ceiling summed from the fractions
    /// is a few pixels under what the same widths add up to when each is
    /// rounded — so the group is handed exactly its natural width and the
    /// share-out still finds itself short, and every word elides in a band
    /// with room to spare (measured 2026-08-11 on Linux: `cap=103` with
    /// `groupW=270`, which was the natural width).
    readonly property int opBadgeW:
        Math.ceil(mOpText.implicitWidth
                  + (topBar.stateHasAlso
                     ? 2 * Theme.spaceXs + mDot.implicitWidth
                       + mOpAlso.implicitWidth
                     : 0)
                  + (topBar.stateHasStep
                     ? Theme.spaceXs + mOpStep.implicitWidth : 0))
        + 2 * Theme.spaceXs
    readonly property int conflictBadgeW:
        Math.ceil(mConflict.implicitWidth) + 2 * Theme.spaceXs
    readonly property int identityBadgeW:
        Math.ceil(mIdentity.implicitWidth) + 2 * Theme.spaceXs

    /// Automation: which of the group's three shapes is on screen, what
    /// the badges were narrowed to, and what the card came back with. The
    /// conditions above are what asks for a state; these are what the band
    /// made of it (`PG_AUTO_ACT=badges` / `badges-hover`).
    readonly property bool stateWordsShown: badgeRow.visible
    readonly property bool stateMarkShown: stateToggle.visible
    readonly property int stateCapW: stateGroup.cap === Number.MAX_VALUE
                                     ? -1 : Math.round(stateGroup.cap)
    readonly property int stateGroupW: Math.round(stateGroup.width)
    readonly property bool stateCardOpen: stateCard.opened
    readonly property string stateCardRows: stateCard.rowsLaidOut()
    readonly property string stateCardSize: stateCard.laidOutSize
    /// Stands in for the pointer where headless cannot put one, so the
    /// card can be photographed (`badges-hover` / `identity-tip`). The
    /// real hover writes this same one property — hover is the input that
    /// cannot be injected, so the card has to be answering a single
    /// question or the headless run proves nothing about it.
    property bool statePointedAt: false

    /// Whether anything is asking for the card: the pointer on the mark,
    /// the pointer inside the card, or the hook standing in for either.
    readonly property bool stateLit: groupHover.hovered
                                     || topBar.statePointedAt
                                     || stateCard.pointerInside
    onStateLitChanged: topBar.settleStateCard()
    /// Opens the card, or starts the wait that closes it. The wait is
    /// `Metrics.hoverKeepMs` rather than `Qt.callLater`: the mark and the
    /// card change their hover in separate frames and in either order, and
    /// callLater runs in between (app-ui.md の 5 つの罠 (3)).
    function settleStateCard() {
        if (!topBar.stateLit) {
            stateSettle.restart()
            return
        }
        stateSettle.stop()
        if (!stateCard.opened && topBar.stateShown)
            stateCard.open()
    }
    Timer {
        id: stateSettle
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!topBar.stateLit)
                stateCard.close()
        }
    }

    signal openRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()
    /// The grab-run moved or changed size in this band's own layout.
    /// `Main` folds in the shifts this band cannot see from here (the
    /// maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()
    /// The run itself, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: grabArea

    /// The narrowest this band can be laid out at — one of the two numbers
    /// the window's floor is the larger of (`Main.floorWidth`).
    ///
    /// Read off the row's own preferred width rather than summed here: the
    /// strip is the only part of the band that gives, so what the row asks
    /// for with the strip at its floor *is* the floor, and a control added
    /// to the band later is counted without anybody remembering to. The
    /// margin is the row's own, which the fill does not cover.
    ///
    /// Read off the row's *minimum* rather than what it asks for: the
    /// state group asks for its words and gives way to a mark, so the row
    /// preferring the words is what lets the two of them share the band
    /// — but the window may still be dragged down to where the mark is
    /// all that is left. Both numbers come from the same row, and a
    /// control added to the band later is counted in either without
    /// anybody remembering to.
    readonly property real floorWidth:
        bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin

    /// One of the window's own buttons: the same cell the app menu sits in
    /// at the other end of the band, so the two ends are built alike and
    /// the outer one keeps its distance from the window's edge. The mark
    /// inside is a step down from the menu's — the platform's own three
    /// are smaller than an app's marks, and at `iconLg` they read as the
    /// loudest thing in the band.
    ///
    /// The close button is the one exception to the wash — red under the
    /// pointer is a convention old enough that departing from it would
    /// read as a bug, not as a house style.
    component WindowButton: Rectangle {
        id: winBtn
        property string kind: ""
        property bool danger: false
        signal triggered()

        width: Theme.railWidth
        height: parent ? parent.height : Theme.toolbarHeight
        color: !winBtnMouse.containsMouse ? "transparent"
               : winBtn.danger ? Theme.danger : Theme.bgHover
        NavIcon {
            anchors.centerIn: parent
            width: Theme.iconMd
            height: Theme.iconMd
            kind: winBtn.kind
            tint: winBtnMouse.containsMouse && winBtn.danger
                  ? Theme.textOnAccent : Theme.textPrimary
        }
        MouseArea {
            id: winBtnMouse
            anchors.fill: parent
            hoverEnabled: true
            onClicked: winBtn.triggered()
        }
    }

    /// Automation: run the push button's hold to its end. Does nothing
    /// unless the button is in the shape that arms it.
    function completePushHold() {
        pushButton.completeHold()
        AppBackend.report("push_hold mode=" + pushButton.mode)
    }

    /// What a press on a tab does, in one place: the left button picks
    /// the tab up, the middle one closes it (デザイン規約 §タブの所作).
    /// The real press and the smoke hook both come through here, so the
    /// two buttons are told apart once rather than twice.
    function pressTab(index, id, button) {
        if (button === Qt.MiddleButton)
            topBar.tabsModel.closeTab(id)
        else
            topBar.tabsModel.setCurrentIndex(index)
    }

    /// Automation: the middle click, landed on the tab at `index`
    /// (`PG_AUTO_ACT=middle-close`).
    function middleClickTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (tab)
            topBar.pressTab(index, tab.tab_id, Qt.MiddleButton)
    }

    /// Automation: the strip as it stands now. A closed tab leaves
    /// nothing of itself behind, so which repositories are still open is
    /// what says the gesture took the tab it was aimed at — the paths
    /// rather than the titles, because every demo repository is called
    /// the same thing and a strip of one name proves nothing.
    function tabPaths() {
        let paths = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            if (tab)
                paths.push(tab.repo_path)
        }
        return paths.join(",")
    }

    /// Automation: the path a tab was opened with, spelled the way the
    /// strip has it (`PG_AUTO_ACT=open-again`). Asking for a repository
    /// that is already open is what the verb does, and this is where the
    /// only string certain to name one comes from.
    function tabPathAt(index) {
        const tab = tabs.itemAtIndex(index)
        return tab ? tab.repo_path : ""
    }

    /// Automation: every tab's width, in the order they sit in
    /// (`PG_AUTO_ACT=tab-widths`). Tabs left at their natural width and
    /// tabs holding the shared cap are what this run is being asked
    /// about, and one number each is what tells them apart.
    function tabWidths() {
        let widths = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            widths.push(tab ? Math.round(tab.width) : 0)
        }
        return widths.join(",")
    }

    /// Automation: the pointer, set down on the tab at `index`
    /// (`PG_AUTO_ACT=tab-mark`). The `✕` comes out on the tab in front
    /// and on the tab under the hand, and the second of those is the
    /// half no headless run can reach any other way.
    function pointAtTab(index) {
        const tab = tabs.itemAtIndex(index)
        if (tab)
            tab.pointed = true
    }

    /// Automation: which tabs have their mark out, in strip order.
    function tabMarks() {
        let marks = []
        for (let i = 0; i < tabs.count; i++) {
            const tab = tabs.itemAtIndex(i)
            marks.push(tab ? tab.markShown : 0)
        }
        return marks.join(",")
    }

    /// The word both toolbar buttons are measured for. They sit side by
    /// side and change wording independently, so one box for the pair is
    /// what keeps either of them from shifting the other — and which of
    /// the wordings is the wider one is a question about the installed
    /// fonts, not about their spelling. Two of the four are commands and
    /// are set in the mono family (デザイン規約 §git 用語のコード表記),
    /// so the winner carries which family it was measured in.
    ///
    /// Settled once rather than bound: a binding that reads four text
    /// metrics and feeds two button widths is a loop as far as the engine
    /// is concerned, and the answer cannot change while the app runs.
    property string widestAction: ""
    property bool widestActionCode: false
    Component.onCompleted: {
        let best = fetchCodeWidest
        for (const m of [fetchWordWidest, pushCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // The longest each button can say, in each of the two voices it says
    // things in. `push` is inside `push -f`, and fetch says the command in
    // every shape but the stopped one, so three cover all six states —
    // push never leaves the command's voice (デザイン規約 §リモートへ送る).
    //
    // Labels rather than TextMetrics, and never drawn: the box these are
    // ranked for is a Label's, and TextMetrics reports a few pixels
    // tighter than one — enough that the pair could be ranked on one
    // measure and sized by another, and the loser could then be the wider
    // of the two (`ActionButton.widestText`).
    component Widest: Label {
        visible: false
        property bool code: false
        font.family: code ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: code ? -Theme.spaceXs : 0
        font.pixelSize: Theme.fontMd
    }
    Widest {
        id: fetchCodeWidest
        code: true
        text: "fetch"
    }
    Widest {
        id: fetchWordWidest
        text: qsTr("Resume")
    }
    Widest {
        id: pushCodeWidest
        code: true
        text: "push -f"
    }

    /// The longest a tab's name is ever drawn (デザイン規約 レイアウト初期値).
    readonly property int tabTitleMaxW: 180
    /// The shortest, in characters rather than in pixels: three of them
    /// and the ellipsis that says the rest was cut (同表). A count is what
    /// this is — three characters cost a different number of pixels in
    /// each of the three platforms' UI fonts, and at every scaling — so
    /// the length comes out of the font instead of out of the table.
    readonly property int tabTitleMinChars: 3
    readonly property int tabTitleMinW:
        Math.ceil(tabTitleFont.advanceWidth("…")
                  + topBar.tabTitleMinChars * tabTitleFont.averageCharacterWidth)
    /// What every tab's name is capped at right now — the strip's answer
    /// to how much room it was given (`settleTitleCap`).
    property real tabTitleCap: topBar.tabTitleMaxW
    /// What the strip would take with nothing cut (`settleTitleCap`), and
    /// the least it is ever laid out at.
    ///
    /// **Two tabs, not one** (2026-08-11 ユーザー指示): a window narrow
    /// enough to leave one tab is narrow enough to have lost what tabs are
    /// for, and the band gives up the state group's words before it gives
    /// up the second tab (規約 §ウィンドウの縁).
    property real tabsWantWidth: 0
    readonly property int tabStripFloorW:
        menuButton.width + plusButton.width + tabs.grabRun
        + 2 * (tabs.tabFixedW + topBar.tabTitleMinW)
    FontMetrics {
        id: tabTitleFont
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
    }

    /// Hands the run out among the tab names, the longest giving way last.
    ///
    /// Each name is granted its natural width while there is room; when
    /// there is not, what is left goes round again among the ones still
    /// asking for more. So a tab whose name already fits never moves, the
    /// ones that give way are the long ones, and they all come to rest on
    /// one width (デザイン規約 §ウィンドウの縁). Below `tabTitleMinW`
    /// nothing narrows any further and the strip scrolls instead.
    ///
    /// Settled by hand rather than bound: the widths are read off a list
    /// of items, and a binding cannot see one of those arrive.
    /// Whole pixels throughout: a name asks for a fractional width, an
    /// item is laid out on a whole one, and a strip sized off the
    /// fractions comes out a pixel over the run it was told to fit in —
    /// which is a strip that scrolls when nothing is out of room
    /// (実測 content=897 against run=896 before the rounding went in).
    function settleTitleCap() {
        let want = []
        for (let i = 0; i < titleMeasure.count; i++) {
            const label = titleMeasure.itemAt(i)
            if (label)
                want.push(Math.min(Math.ceil(label.implicitWidth),
                                   topBar.tabTitleMaxW))
        }
        // What the strip would take with no name cut. This is what the
        // row is asked for, so that the band's leftover is shared with
        // the state group in proportion to what each of them wants —
        // asking for the floor instead had the tabs down to three
        // characters beside two whole badges (reported 2026-08-11).
        // Measured off the same hidden labels the cap is, so it does not
        // move with the run it is about to be handed.
        topBar.tabsWantWidth =
            menuButton.width + plusButton.width + tabs.grabRun
            + want.reduce((sum, w) => sum + w, 0)
            + want.length * tabs.tabFixedW
        if (want.length === 0) {
            topBar.tabTitleCap = topBar.tabTitleMaxW
            return
        }
        want.sort((a, b) => a - b)
        let left = Math.floor(tabs.runAvail) - want.length * tabs.tabFixedW
        let cap = topBar.tabTitleMaxW
        for (let i = 0; i < want.length; i++) {
            const share = Math.floor(left / (want.length - i))
            if (want[i] > share) {
                cap = share
                break
            }
            left -= want[i]
        }
        topBar.tabTitleCap = Math.max(topBar.tabTitleMinW,
                                      Math.min(cap, topBar.tabTitleMaxW))
    }

    /// The names at their natural width, off screen. The strip's own
    /// labels are the ones being capped, so they cannot also be what the
    /// cap is measured from. These carry the font the strip draws in —
    /// the heavier weight the current tab is set in included, which is
    /// wider — so what comes back is the width the strip will ask for.
    Repeater {
        id: titleMeasure
        model: topBar.tabsModel
        onCountChanged: topBar.settleTitleCap()
        delegate: Label {
            required property int index
            required property string title

            visible: false
            text: title
            font.weight: topBar.tabsModel.currentIndex === index
                         ? Font.DemiBold : Font.Normal
            onImplicitWidthChanged: topBar.settleTitleCap()
            Component.onCompleted: topBar.settleTitleCap()
        }
    }

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        id: bandRow
        anchors.fill: parent
        // The band's own right edge is not the window's: the client area
        // reaches past what is drawn, so a row flush with it puts the
        // close button's last few pixels off screen and its wash reads as
        // clipped (measured: the cell ended 4.5px beyond the visible
        // edge). `spaceXs` lands it flush instead.
        anchors.rightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
        // The tighter step between this row's controls. A browser's band
        // carries the window's buttons and nothing else; this one carries
        // the command log, fetch and push as well, so the same spacing a
        // browser can afford would push the three off the end of what the
        // eye reads as one group.
        spacing: Theme.spaceXs
        // The strip: the app menu, the tabs, the way to open one more, and
        // whatever band is left over. Placed by hand rather than by a Row,
        // because the first three have to touch and the leftover has to be
        // measurable — it is both what the tabs may grow into and where
        // the window is taken hold of.
        //
        // No TabBar either: full control of the geometry is what puts the
        // selected tab's underline exactly on the band's bottom edge with
        // no styling leftovers beneath it.
        Item {
            id: tabStrip
            Layout.fillWidth: true
            Layout.fillHeight: true
            // What the strip costs when everything in it has given all it
            // can: the menu cell, one tab with its name cut to the three
            // characters the table allows, the way to open another, and
            // the run the window is taken hold of (デザイン規約 §ウィンドウ
            // の縁). It fills, so this is only what it asks for and not a
            // floor the row enforces — it is here so the band can say what
            // it costs, which is one half of the window's own floor
            // (`floorWidth`). Measured before there was one: at 320px the
            // close button was outside the window.
            implicitWidth: topBar.tabsWantWidth
            Layout.minimumWidth: topBar.tabStripFloorW
            // First to give and first to take. The band's order when it
            // runs short is: the tab names narrow together, then the state
            // group's words do, then the strip scrolls, then the group
            // becomes a mark (2026-08-11 ユーザー指示) — and a stretch
            // this much larger than the group's is what puts the strip at
            // the front of both queues. It takes the leftover too, since
            // the group stops at its own ceiling: room past the words is
            // grab run, which is the tabs' business.
            Layout.horizontalStretchFactor: 100
            // App menu (Claude-Desktop-style hamburger); most entries are
            // placeholders until their phases land.
            //
            // The head of the folded sidebar's column rather than a member
            // of the tab strip: `railWidth` wide, the band's full height,
            // and the same square wash the rail's cells wear, so this mark
            // and the section marks under it stand on one line. `iconLg`
            // rather than `iconMd` because the step follows the height of
            // the band the mark sits in — what `menu` actually draws is
            // 10px wide inside a 16px box (デザイン規約 §寸法).
            ToolButton {
                id: menuButton
                width: Theme.railWidth
                height: tabStrip.height
                padding: 0
                hoverEnabled: true
                Accessible.name: qsTr("Application menu")
                // Written out instead of borrowing HoverToolButton: an
                // open menu keeps the wash, which no hover of its own can
                // say — what is on screen has to say which mark put it
                // there (`NavRail`).
                background: Rectangle {
                    color: menuButton.hovered || appMenu.opened
                           ? Theme.bgHover : "transparent"
                }
                contentItem: Item {
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconLg
                        height: Theme.iconLg
                        kind: "menu"
                        tint: Theme.textPrimary
                    }
                }
                onClicked: appMenu.open()
                AppMenu {
                    id: appMenu
                    // A full-height cell ends where the band does, so the
                    // card would otherwise open on top of the divider.
                    y: menuButton.height + Theme.splitterWidth
                    AppMenuItem {
                        text: qsTr("Open repository…")
                        onTriggered: topBar.openRepositoryRequested()
                    }
                    AppMenuItem {
                        text: qsTr("Clone repository…")
                        enabled: false
                    }
                    AppMenuSeparator {}
                    // A local re-read (no network). The page re-reads
                    // itself on a tick while it is on screen, so this is
                    // here for where that cannot reach — a repository on a
                    // share that reads slowly, or a read that failed —
                    // rather than for everyday use, and it costs no
                    // toolbar room to keep.
                    AppMenuItem {
                        text: qsTr("Reload")
                        enabled: topBar.curPage !== null
                        onTriggered: topBar.curPage.pageTab.refreshAll()
                    }
                    AppMenuSeparator {}
                    AppMenuItem {
                        text: qsTr("Identity…")
                        onTriggered: topBar.identityEditRequested()
                    }
                    AppMenuItem {
                        text: qsTr("Settings…")
                        onTriggered: topBar.settingsRequested()
                    }
                    AppMenuItem {
                        text: qsTr("About Platitude GG")
                        enabled: false
                    }
                    AppMenuSeparator {}
                    AppMenuItem {
                        text: qsTr("Exit")
                        onTriggered: Qt.quit()
                    }
                }
            }
            // As wide as the tabs it holds, up to what the strip has left
            // once the menu and the `+` have their places. Past that it
            // scrolls: the alternative is the tabs pushing the repository's
            // own controls off the end of the band, and those are the ones
            // that have to stay where the hand expects them.
            //
            // The tabs narrow before it comes to that (`settleTitleCap`),
            // so the scrolling starts where they can give no more.
            ListView {
                id: tabs
                x: menuButton.width
                height: tabStrip.height
                /// Band the tabs may not grow into. Where this row is the
                /// title bar, the empty run past the last tab is the only
                /// place left to take hold of the window, and a run that
                /// can be squeezed to nothing by opening one more tab is
                /// not somewhere anyone would think to reach for. Two of
                /// the menu's cells is enough to read as a gap rather than
                /// as spacing.
                readonly property real grabRun: topBar.captionMerged
                                                ? 2 * Theme.railWidth : 0
                /// The run the tabs share out between them. Both the width
                /// below and the cap the names are narrowed to read this
                /// one expression — written twice, a value that can be set
                /// but does not take effect is what comes back.
                //
                // `tabs.grabRun` stays qualified: an unqualified name here
                // reads whatever id happens to share it — ids outrank the
                // enclosing object's own properties — and an Item minus a
                // number is NaN, which took the whole strip's width with
                // it once (no tabs drawn, nothing said why).
                readonly property real runAvail:
                    Math.max(0, tabStrip.width - menuButton.width
                                - plusButton.width - tabs.grabRun)
                onRunAvailChanged: {
                    topBar.settleTitleCap()
                    // The state group folds off this strip's width as well
                    // as its own, and the two do not always change in the
                    // same frame.
                    stateGroup.settleCap()
                }
                /// The air a tab is set in: `spaceSm` before the name, and
                /// after the mark what is left of `spaceSm` once the air
                /// the mark brings with it is taken off (デザイン規約 §余白;
                /// the delegate below carries the reasoning).
                readonly property int markAir: (Theme.iconLg - Theme.iconSm) / 2
                                               + Theme.spaceXs
                readonly property real tabPadW: Theme.spaceSm
                                                + (Theme.spaceSm - tabs.markAir)
                /// What a tab costs before its name has a single letter in
                /// it — that air, and the mark at its end.
                readonly property real tabFixedW: tabs.tabPadW + Theme.iconLg
                width: Math.max(0, Math.min(contentWidth, tabs.runAvail))
                orientation: ListView.Horizontal
                // Hard stop at the ends, as everywhere else that scrolls
                // (デザイン規約 §QML 実装ルール).
                boundsBehavior: Flickable.StopAtBounds
                clip: true
                // The strip scrolls by wheel only. Left interactive, the
                // view watches every press for a drag and steals the grab
                // at the platform's threshold — 4px on Windows — so a
                // click that lands with a little sideways motion cancels
                // the tab's own MouseArea instead of switching: no
                // switch, and the hover wash stays off until the pointer
                // moves again (reported as "the tab stopped taking the
                // first click"). Nothing here wants drag-to-pan, so the
                // watching is all the interactivity ever bought.
                interactive: false
                // Every delegate stays alive however far the strip is
                // scrolled: the automation walks the items for their
                // paths and ids (`tabPaths` / `middleClickTab`), and a
                // released delegate answers those walks with null. Tabs
                // are counted in ones, so keeping them all costs nothing.
                cacheBuffer: 65536
                // A wheel over a strip that runs sideways should move it
                // sideways, whichever way the wheel itself reports: a
                // plain wheel only ever says "vertical", and it is the
                // only wheel most people have.
                WheelHandler {
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onWheel: event => {
                        const step = event.angleDelta.x !== 0
                                     ? event.angleDelta.x : event.angleDelta.y
                        tabs.contentX = Math.max(
                            0, Math.min(tabs.contentWidth - tabs.width,
                                        tabs.contentX - step))
                    }
                }
                model: topBar.tabsModel
                delegate: Rectangle {
                    id: tabItem
                    required property int index
                    required property int tab_id
                    required property string title
                    required property string repo_path
                    readonly property bool current: topBar.tabsModel.currentIndex === index
                    /// Whether the pointer is on this tab. The real hover
                    /// and the smoke hook write this one property — hover
                    /// is the input that cannot be injected, so the wash
                    /// and the mark have to be answering a single question
                    /// or the headless run proves nothing about either.
                    property bool pointed: false
                    /// Automation: whether the mark is out on this tab.
                    /// Read off the mark itself — reporting what was asked
                    /// of it would go on passing after the binding that
                    /// draws it had come apart.
                    readonly property real markShown: closeMark.opacity
                    // Air the `✕` brings with it (`tabs.markAir`): the
                    // `iconSm` mark stands centred in an `iconLg` button,
                    // and the glyph is drawn inset again inside that
                    // (`NavIcon` "close" — diagonals read heavier, so it
                    // sits further in than the bars do).
                    //
                    // What the eye measures is ink, not boxes. So the mark's
                    // seat is given only the air it has not already taken
                    // (`spaceSm − markAir`), and the run to the name is left
                    // to the mark alone — otherwise both are spent twice and
                    // the gap inside the tab reads wider than the tab's own
                    // margins (実測 13px between name and mark against 9px to
                    // the edge). The width is the exact fit rather than
                    // `implicitWidth + 2 * spaceSm`: anything the layout
                    // cannot hand out lands past the last item, which is to
                    // say on the right margin, where nobody wrote it down.
                    // Rounded up for the same reason `settleTitleCap`
                    // works in whole pixels: the two have to agree on
                    // what this tab costs, or the strip scrolls by the
                    // fractions they disagree about.
                    width: Math.ceil(tabContent.implicitWidth) + tabs.tabPadW
                    height: tabs.height
                    color: current ? Theme.bgSelected : "transparent"
                    // The middle button is taken here rather than on the
                    // `✕`: the whole tab answers to it, so closing one
                    // never asks the hand to find a 16px target — and the
                    // `✕` sits on top of this area without accepting the
                    // middle button, so a press that lands on the mark
                    // falls through to the same gesture.
                    MouseArea {
                        id: tabMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                        onClicked: mouse => topBar.pressTab(tabItem.index,
                                                            tabItem.tab_id,
                                                            mouse.button)
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
                        anchors.rightMargin: Theme.spaceSm - tabs.markAir
                        spacing: 0
                        Label {
                            text: tabItem.title
                            elide: Text.ElideRight
                            // The cap the whole strip shares, so the tabs
                            // that give way give way together. Capping the
                            // hint is what narrows the tab: the row asks
                            // for what it is allowed, and the tab's width
                            // above is that plus its air.
                            Layout.maximumWidth: topBar.tabTitleCap
                            Layout.fillHeight: true
                            verticalAlignment: Text.AlignVCenter
                            // The name is what a tab is for, so it reads
                            // at full strength in every tab; which one is
                            // in front is said by the seat it sits in —
                            // the wash, the rule under it, and the weight.
                            font.weight: tabItem.current ? Font.DemiBold : Font.Normal
                            color: Theme.textPrimary
                        }
                        // Shown on the tab in front and under the pointer,
                        // and nowhere else — a row of marks is a row of
                        // things asking to be pressed, and only one tab at
                        // a time is being aimed at (デザイン規約 §タブの所作).
                        //
                        // Dimmed rather than dropped: an item the layout
                        // has stopped seeing takes its width with it, and
                        // the tab would then change size under the hand
                        // that came to close it. Nothing is reachable
                        // while it is out, either — being out is what
                        // "the pointer is elsewhere" means.
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
                                    // A step under the name in both size
                                    // and colour: what it closes is the
                                    // thing being read, and the mark is
                                    // the way out of it rather than the
                                    // point of it.
                                    tint: Theme.textSecondary
                                }
                            }
                            onClicked: topBar.tabsModel.closeTab(tabItem.tab_id)
                        }
                    }
                }
            }
            HoverToolButton {
                id: plusButton
                x: tabs.x + tabs.width
                anchors.verticalCenter: parent.verticalCenter
                text: "+"
                font.pixelSize: Theme.fontLg
                onClicked: topBar.openRepositoryRequested()
            }
            // The run of empty band past the last tab. Where this band is
            // the title bar, it is also the only place left to take hold
            // of the window — and it is a real title bar, not an imitation
            // of one: the hit test answers HTCAPTION for this rectangle
            // (`winframe::hit_test`), so a press here never reaches the
            // scene, and the drag, the snap, the double-click and the
            // right-click menu are all the platform's own. No handlers —
            // the scene's only job is saying where the run is.
            Item {
                id: grabArea
                x: plusButton.x + plusButton.width
                width: Math.max(0, tabStrip.width - x)
                height: tabStrip.height
                onXChanged: topBar.captionStripMoved()
                onWidthChanged: topBar.captionStripMoved()
                Component.onCompleted: topBar.captionStripMoved()
            }
        }

        // Whatever this repository is in the middle of. Three of these can
        // stand at once — a stopped operation that hit a conflict, on a
        // machine that has never been told who is committing — and as
        // whole words they take 340px of the row the tabs are for.
        //
        // So they give way in the two steps the tab names give way in
        // (規約 §ウィンドウの縁): while there is room the words stand
        // whole; when there is not they narrow *together* to one width,
        // each keeping its opening letters; and only when even those are
        // too short to tell apart does the group come down to a single
        // mark. The card the pointer opens carries the whole of it at
        // every step, so nothing is ever only in the band.
        Item {
            id: stateGroup

            /// Which colour the mark takes once the words are gone. The
            /// conflict is the one of the three that stops work, so it
            /// wins whenever it is among them (規約 §状態).
            readonly property color tint: topBar.conflictBadgeShown
                                          ? Theme.danger : Theme.warning
            /// What the badges standing would take with nothing narrowed,
            /// and what the mark costs on its own. The first is this
            /// group's ceiling and the second is what it asks for, so the
            /// row hands it whatever is left over between them — and the
            /// window's floor is costed at the mark (`TopBar.floorWidth`).
            readonly property real naturalWidth:
                (topBar.opBadgeShown ? topBar.opBadgeW + Theme.spaceXs : 0)
                + (topBar.conflictBadgeShown
                   ? topBar.conflictBadgeW + Theme.spaceXs : 0)
                + (topBar.identityBadgeShown
                   ? topBar.identityBadgeW + Theme.spaceXs : 0)
                - Theme.spaceXs
            readonly property real foldedWidth:
                stateMark.implicitWidth + 2 * fetchButton.padding
            /// What each badge's box is drawn at, once they have given way
            /// together, and whether the giving way has gone as far as it
            /// can. `Number.MAX_VALUE` is "nothing is narrowed".
            property real cap: Number.MAX_VALUE
            property bool folded: false

            /// Hands the run out among the badges, the widest giving way
            /// last — the same max-min share the tab names are settled
            /// with (`settleTitleCap`), and settled by hand for the same
            /// reason: the widths are read off a list of items, and a
            /// binding cannot see one of those arrive.
            function settleCap() {
                // Already whole (`TopBar.opBadgeW` and its two neighbours),
                // and the ceiling above is summed from the same three —
                // so a group handed its natural width has exactly what
                // this share-out is about to hand back out.
                let want = []
                if (topBar.opBadgeShown)
                    want.push(topBar.opBadgeW)
                if (topBar.conflictBadgeShown)
                    want.push(topBar.conflictBadgeW)
                if (topBar.identityBadgeShown)
                    want.push(topBar.identityBadgeW)
                if (want.length === 0) {
                    stateGroup.cap = Number.MAX_VALUE
                    stateGroup.folded = false
                    return
                }
                let left = Math.floor(stateGroup.width)
                           - (want.length - 1) * Theme.spaceXs
                want.sort((a, b) => a - b)
                let cap = Number.MAX_VALUE
                for (let i = 0; i < want.length; i++) {
                    const share = Math.floor(left / (want.length - i))
                    if (want[i] > share) {
                        cap = share
                        break
                    }
                    left -= want[i]
                }
                // Two ways the words stop being worth their room: the
                // opening letters would no longer tell one badge from
                // another, or the tabs beside them have narrowed as far
                // as they go and are about to start scrolling — at which
                // point what the band is short of is tabs, and this
                // group's whole width is the readiest thing to hand them
                // (2026-08-11 ユーザー指示: タブ等幅化 → この群の等幅化
                // → タブ横スク → 群を畳む → 床).
                stateGroup.folded = cap < topBar.stateBadgeMinW
                                    || topBar.tabTitleCap <= topBar.tabTitleMinW
                stateGroup.cap = cap
            }
            onWidthChanged: stateGroup.settleCap()

            visible: topBar.stateShown
            implicitHeight: fetchButton.implicitHeight
            // Asks for its words and will come down to the mark. The row
            // hands its leftover out in proportion to what each filling
            // item asked for, so **what is asked for is what decides who
            // gives way** — asking for the mark got this group a sliver
            // (104px of 794 at a 1440-wide window) and asking with a
            // stretch of 100 got it everything, and the tabs came down to
            // three characters beside two whole words (both measured
            // 2026-08-11, both reported). Asking for the words and no
            // more puts the two of them in proportion: the tabs narrow
            // and this group narrows, each by its own rule.
            implicitWidth: stateGroup.naturalWidth
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the
            // order). Asking for the words is what keeps the strip from
            // taking them; the small stretch is what makes the strip give
            // way first when there is not enough for both.
            Layout.horizontalStretchFactor: 1
            // The pointer anywhere on the group opens the card — over the
            // words as much as over the mark, since the words are narrowed
            // and the card is where the whole of them is. A handler rather
            // than an area: it is passive, so the identity badge under it
            // still takes its own press (app-ui.md).
            HoverHandler {
                id: groupHover
            }

            // The words, while there is room for them. Right-aligned: this
            // group grows and shrinks against the tabs on its left, and
            // what has to stay put is its edge with `>_`.
            Row {
                id: badgeRow
                visible: !stateGroup.folded
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.spaceXs

                /// One badge's box. The width is the natural one until the
                /// group hands down a cap, and the word inside elides into
                /// whatever that leaves — so what a narrowed badge keeps
                /// is its opening letters and the ellipsis.
                component Badge: Rectangle {
                    id: badge
                    property bool filled: false
                    /// What this badge is drawn at with nothing narrowed.
                    /// Handed in from the hidden measurement rather than
                    /// read off the row inside: that row is not laid out
                    /// while the group is folded (`BadgeWord`).
                    property real naturalW: 0
                    /// Whether this badge is also a way somewhere. Only
                    /// the identity one is, narrowed or not (規約 §identity).
                    property bool pressable: false
                    signal pressed()
                    default property alias content: badgeRowInner.data

                    implicitWidth: badge.naturalW
                    width: Math.min(badge.naturalW, stateGroup.cap)
                    height: Theme.iconLg
                    radius: Theme.radiusSm
                    color: badge.filled ? Theme.danger
                           : badge.pressable && badgeHover.hovered
                             ? Theme.bgHover : "transparent"
                    border.color: badge.filled ? "transparent" : Theme.warning
                    border.width: badge.filled ? 0 : Theme.borderWidth
                    RowLayout {
                        id: badgeRowInner
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceXs
                        anchors.rightMargin: Theme.spaceXs
                        spacing: Theme.spaceXs
                        // Handlers rather than a `MouseArea` and a wash of
                        // its own: an `Item` handed to a layout is given a
                        // seat in it, and the word beside it loses that
                        // much room (app-ui.md「`Layout` の子に `MouseArea`
                        // を置かない」— measured here as `SET IDENT…` in a
                        // window with 800px going spare).
                        HoverHandler {
                            id: badgeHover
                            enabled: badge.pressable
                        }
                        TapHandler {
                            enabled: badge.pressable
                            onTapped: badge.pressed()
                        }
                    }
                }

                Badge {
                    id: opBadge
                    visible: topBar.opBadgeShown
                    naturalW: topBar.opBadgeW
                    Label {
                        text: topBar.stateWt !== null ? topBar.stateWt.opText : ""
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                        // The one part of this badge that gives: the count
                        // and the second operation are a few characters
                        // each and mean nothing cut in half.
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        Layout.maximumWidth: implicitWidth
                    }
                    // Bisect runs alongside rather than instead, so it is
                    // the one thing that can share this badge. What goes
                    // between the two names is drawn, not typed — a middle
                    // dot would put a full-width cell in the badge
                    // (規約 §余白).
                    DotMark {
                        visible: topBar.stateWt !== null
                                 && topBar.stateWt.opAlso !== ""
                        tint: Theme.warning
                        Layout.alignment: Qt.AlignVCenter
                    }
                    Label {
                        visible: topBar.stateWt !== null
                                 && topBar.stateWt.opAlso !== ""
                        text: topBar.stateWt !== null ? topBar.stateWt.opAlso : ""
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                    // The count is the half a stopped rebase cannot say
                    // without it; a merge steps through nothing and has
                    // none.
                    Label {
                        visible: topBar.stateWt !== null
                                 && topBar.stateWt.opSteps > 0
                        text: topBar.stateWt === null ? ""
                              : qsTr("%1/%2").arg(topBar.stateWt.opStep)
                                             .arg(topBar.stateWt.opSteps)
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
                Badge {
                    id: conflictBadge
                    visible: topBar.conflictBadgeShown
                    naturalW: topBar.conflictBadgeW
                    filled: true
                    Label {
                        text: qsTr("CONFLICTS")
                        color: Theme.textOnAccent
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        Layout.maximumWidth: implicitWidth
                    }
                }
                Badge {
                    id: identityBadge
                    visible: topBar.identityBadgeShown
                    naturalW: topBar.identityBadgeW
                    // The one badge of the three that is also a way
                    // somewhere, narrowed or not.
                    pressable: true
                    onPressed: topBar.identityEditRequested()
                    Label {
                        text: qsTr("SET IDENTITY")
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                        Layout.maximumWidth: implicitWidth
                    }
                }
            }

            // …and the mark the group comes down to. `…` typed rather than
            // drawn, unlike the arrows and the middle dot: those are East
            // Asian Ambiguous and the CJK families hold them in a
            // full-width cell, but the ellipsis is what Qt spends on its
            // own eliding and it measured the same on both OSes (764c362).
            //
            // The same box the command log's mark takes, and for the same
            // reason: the padding that sets how big a target is here
            // belongs to the Fusion control beside it rather than to the
            // table (規約 §ウィンドウの縁「その 3 つは 1 つの箱の高さに
            // 揃える」). `Theme.buttonMinWidth` does not reach it — that
            // floor is for a box put round a *word*, and given it the mark
            // came out 80 wide with 26px of air at either end of three
            // dots (規約 §リポジトリが今どうなっているか).
            Rectangle {
                id: stateToggle
                visible: stateGroup.folded
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                implicitWidth: stateGroup.foldedWidth
                height: fetchButton.implicitHeight
                width: implicitWidth
                radius: Theme.radiusSm
                color: stateMouse.containsMouse ? Theme.bgHover : "transparent"
                // The frame is the badges', kept: this mark is out only
                // when something is already the matter, so unlike `>_` —
                // which wears one only when something went wrong — it
                // never stands without it.
                border.width: Theme.borderWidth
                border.color: stateGroup.tint
                Accessible.role: Accessible.Button
                Accessible.name: qsTr("What this repository is in the middle of")
                Label {
                    id: stateMark
                    anchors.centerIn: parent
                    text: "…"
                    font.pixelSize: Theme.fontMd
                    font.weight: Font.DemiBold
                    color: stateGroup.tint
                }
                MouseArea {
                    id: stateMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    // A press opens what the hover opens. A mark this
                    // small is aimed at as often as it is rested on, and a
                    // press that did nothing would read as a dead control.
                    onClicked: topBar.settleStateCard()
                }
            }

            BandStateCard {
                id: stateCard
                // Under the group and flush with its right-hand edge: the
                // group sits at the band's right-hand end, and a card
                // centred on it would open past the window.
                x: stateGroup.width - width
                y: stateGroup.height + Theme.spaceXs
                opText: topBar.stateWt !== null ? topBar.stateWt.opText : ""
                opAlso: topBar.stateWt !== null ? topBar.stateWt.opAlso : ""
                opStep: topBar.stateWt !== null ? topBar.stateWt.opStep : 0
                opSteps: topBar.stateWt !== null ? topBar.stateWt.opSteps : 0
                conflictCount: topBar.stateWt !== null
                               ? topBar.stateWt.conflictCount : 0
                identityUnsaved: AppBackend.identityUnsaved
                opShown: topBar.opBadgeShown
                conflictShown: topBar.conflictBadgeShown
                identityShown: topBar.identityBadgeShown
                onIdentityRequested: topBar.identityEditRequested()
            }
        }
        // The git commands this tab ran. Closed, this mark is the whole
        // of the feature on screen; it is also where the state of the
        // last one shows, so the toolbar says something failed without
        // spending a line on a message nobody can read in 320px.
        Rectangle {
            id: commandsToggle
            readonly property var log: topBar.curPage !== null
                                       ? topBar.curPage.pageCommands : null
            // Asked of the log rather than of the page: closing a tab
            // takes the page's models down while the page itself is
            // still standing, so `curPage !== null` is true for a beat
            // after there is nothing left to read off it.
            readonly property bool wrong:
                commandsToggle.log !== null
                && (commandsToggle.log.failed
                    || topBar.curPage.pageTab.lastError !== "")
            readonly property bool open: topBar.curPage !== null
                                         && topBar.curPage.commandsOpen

            visible: topBar.curPage !== null
            // Measured off the pair beside it rather than written to
            // tokens of its own. This is the first of the band's three
            // pressable boxes (デザイン規約 §ウィンドウの縁「並びは `>_` /
            // fetch / push」), and what sets how big a target is here is
            // the padding a Fusion `ToolButton` keeps around its content
            // — a number the theme does not have and cannot be matched
            // by choosing from the table. Sized from the table it came
            // out a step under both of its neighbours and read as the
            // odd one out (measured: 24x20 against their 92x32).
            //
            // The height is taken whole and the width is the mark plus
            // that padding: the three only sit on one line if the
            // heights are equal, while the width is the mark's own
            // business — there is no word here for the box to be
            // measured for.
            //
            // Not the seat the pair give their icon, either: that seat
            // is widened to hold an icon and the hold mark side by side,
            // and a button that is only ever clicked does not pay for a
            // pairing it cannot have (`ActionButton.besideWord` hands
            // the same air back for the same reason).
            //
            // `padding` rather than the two sides it settles to: those
            // carry the shared box's slack as well (`ActionButton.slack`),
            // so reading them would hand this mark the air that belongs to
            // a word it does not have — and would move it every time the
            // fetch button changed its wording.
            implicitWidth: commandsMark.implicitWidth
                           + 2 * fetchButton.padding
            implicitHeight: fetchButton.implicitHeight
            radius: Theme.radiusSm
            color: open ? Theme.bgSelected
                   : commandsMouse.containsMouse ? Theme.bgHover
                   : "transparent"
            border.width: commandsToggle.wrong ? Theme.borderWidth : 0
            border.color: Theme.danger
            Label {
                id: commandsMark
                anchors.centerIn: parent
                text: ">_"
                font.family: Theme.monoFamily
                font.pixelSize: Theme.fontMd
                color: commandsToggle.wrong ? Theme.danger
                       : commandsToggle.log !== null && commandsToggle.log.running
                         ? Theme.accent
                       : commandsToggle.open ? Theme.textPrimary
                       : Theme.textMuted
            }
            MouseArea {
                id: commandsMouse
                anchors.fill: parent
                hoverEnabled: true
                onClicked: topBar.curPage.toggleCommands()
                ToolTip.visible: containsMouse
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: commandsToggle.open
                              ? qsTr("Hide the git commands this window ran")
                              : commandsToggle.wrong
                                ? qsTr("The last command failed — read it here")
                                : qsTr("Show the git commands this window ran")
            }
        }
        // Fetch, and everything the network has to say about fetching.
        // There is no indicator beside it: a second thing on the toolbar
        // saying the same three states was one place too many for the
        // reader to look (デザイン規約 §リモートから取り込む).
        ActionButton {
            id: fetchButton
            /// Fetches that failed in a row, whoever asked for them.
            readonly property int fails: topBar.curPage !== null
                                         ? topBar.curPage.pageTab.fetchFailures : 0
            /// Enough of them that the timer was stopped. Only a hold on
            /// this button starts it again.
            readonly property bool stopped: topBar.curPage !== null
                                            && topBar.curPage.pageTab.autoFetchSuspended

            kind: "fetch"
            // Stopped, the button is no longer about one fetch: it is the
            // way back to fetching on its own, and it says so — in words,
            // where every other shape of it says the command.
            text: fetchButton.stopped ? qsTr("Resume") : "fetch"
            code: !fetchButton.stopped
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // Only the stopped step takes a colour for its word. A run of
            // failures is a notice, and a notice is said by the frame and
            // the mark while the word stays plain — colouring it would
            // borrow the look of the two buttons here that are held, and
            // this one is still a click (デザイン規約 §長押し —
            // 警告の色は語ではなく枠と印が持つ).
            tone: fetchButton.stopped ? Theme.danger : Theme.textPrimary
            // The state a step down, for the wait: a fetch that runs while
            // the last ones failed is still the button that failed
            // (デザイン規約 §暗く落とした段). Read off the state rather
            // than off `tone`, because the word the warned shape keeps is
            // not what the ring is standing in for — the frame is.
            toneDim: fetchButton.stopped ? Theme.dangerDim
                     : fetchButton.fails > 0 ? Theme.warningDim
                     : Theme.textMuted
            frameColor: fetchButton.stopped ? Theme.danger
                        : fetchButton.fails > 0 ? Theme.warning
                        : "transparent"
            // Only while the word is still `Fetch`: once it reads
            // `Resume`, the word is the news and a mark beside it is the
            // fourth thing on one button saying the same thing.
            alert: fetchButton.fails > 0 && !fetchButton.stopped
            alertTone: Theme.warning
            holdMs: fetchButton.stopped ? Metrics.holdMs : 0
            // Whoever asked for it, the network shows here: a fetch on
            // the timer turns the button the way a clicked one does.
            busy: topBar.curPage !== null
                  && (topBar.curPage.pageTab.busyOp === "fetch"
                      || topBar.curPage.pageTab.autoFetchRunning)
            still: AppBackend.shotDir !== ""
            enabled: topBar.curPage !== null
                     && topBar.curPage.pageTab.remoteCount > 0
                     && (fetchButton.stopped
                         || topBar.curPage.pageTab.busyCount === 0)
            ToolTip.visible: hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: {
                if (topBar.curPage === null)
                    return ""
                const what = fetchButton.stopped
                             ? qsTr("Automatic fetching stopped after %n failure(s). Hold to start it again.", "",
                                    fetchButton.fails)
                             : qsTr("Fetch all remotes and prune deleted branches")
                const why = topBar.curPage.pageTab.autoFetchError
                if (fetchButton.fails > 0 && why !== "")
                    return what + "\n\n" + why
                if (fetchButton.fails > 0)
                    return what
                return what + "\n" + (AppBackend.autoFetchMinutes > 0
                                      ? qsTr("Automatically every %n minute(s)", "",
                                             AppBackend.autoFetchMinutes)
                                      : qsTr("Automatic fetching is off"))
            }
            onActivated: topBar.curPage.pageTab.fetch("")
            onHeld: topBar.curPage.pageTab.resumeAutoFetch()
        }
        // Push, in whichever shape this branch's standing with its remote
        // allows (デザイン規約 §リモートへ送る). The counts behind it are
        // from the last fetch, so they are believed only where they refuse.
        ActionButton {
            id: pushButton
            readonly property string mode:
                topBar.curPage !== null ? topBar.curPage.pushState : "closed"
            /// The last go at sending this branch came back refused. The
            /// same warning a fetch wears after one or two failures, and
            /// for the same reason; there is no stopped step past it,
            /// because nothing sends on its own to be stopped
            /// (デザイン規約 §リモートへ送る).
            readonly property bool failed: topBar.curPage !== null
                                           && topBar.curPage.pushFailed
            /// The frame's warning colour, for either of the two things
            /// that call for it: what an overwrite would do, and what the
            /// last go did. The word takes it for only one of them (below).
            readonly property bool warned: pushButton.mode === "diverged"
                                           || pushButton.failed
            kind: "push"
            busy: topBar.curPage !== null
                  && topBar.curPage.pageTab.busyOp === "push"
            still: AppBackend.shotDir !== ""
            // Two fixed wordings, no counts: how far ahead the branch is
            // stands in the sidebar and in this button's own tooltip, and
            // a number here would make the button a different width for
            // every value it took (デザイン規約 §リモートへ送る).
            //
            // The button says the command in every state, first push
            // included: what makes that one different is not the command
            // but that nothing here knows where it goes yet, and the
            // question that opens is where that is said.
            text: mode === "diverged" ? "push -f" : "push"
            code: true
            // Every shape of both buttons measured against the longest of
            // them, so the toolbar's right-hand end sits still while the
            // branch's standing with its remote changes under it.
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // The overwrite colours its word; the refusal does not. Both
            // are warnings, but only one of them changes what the press
            // costs, and the word is where that is said — a refused push
            // is the same one click it was before git turned it down
            // (デザイン規約 §長押し — 警告の色は語ではなく枠と印が持つ).
            tone: pushButton.mode === "diverged" ? Theme.warning
                                                 : Theme.textPrimary
            // A force push on the wire is still a force push, and a second
            // go at one git turned down is still the button that was turned
            // down, so the ring and the frame keep the warning through the
            // wait, a step down (デザイン規約 §暗く落とした段). Both
            // shapes, not just the coloured one: while the wait lasts there
            // is no word to read, so what the ring stands in for is the
            // frame.
            toneDim: pushButton.warned ? Theme.warningDim : Theme.textMuted
            frameColor: pushButton.warned ? Theme.warning : "transparent"
            // Said past the word, where a fetch says it: the frame's colour
            // is worn by the diverged shape as well, so on its own it would
            // not tell "cannot land plainly" from "did not land".
            alert: pushButton.failed
            alertTone: Theme.warning
            // Diverged, the button stays live for the hold that is its
            // only gesture: a plain push cannot land there, so nothing
            // else is waiting on a click to be mistaken for.
            holdMs: mode === "diverged" ? Metrics.holdMs : 0
            enabled: topBar.curPage !== null
                     && (topBar.curPage.canPush
                         || (mode === "diverged"
                             && topBar.curPage.canForcePush))
            onHeld: topBar.curPage.forcePush()
            ToolTip.visible: hovered
            ToolTip.delay: Metrics.tipDelayMs
            ToolTip.text: {
                if (topBar.curPage === null)
                    return ""
                const to = topBar.curPage.pushTargetLabel
                // Nothing to name a target with in the first state: where
                // it goes is what the press is about to ask.
                const what = pushButton.mode === "publish"
                             ? qsTr("This branch has not been sent anywhere yet — asks where it goes")
                           : pushButton.mode === "ready"
                             ? qsTr("Push %n commit(s) to %1", "",
                                    topBar.curPage.pageWt.ahead).arg(to)
                           : pushButton.mode === "clean"
                             ? qsTr("Nothing to push — %1 is up to date")
                               .arg(to)
                           : pushButton.mode === "behind"
                             ? qsTr("Nothing to push — %1 has moved ahead")
                               .arg(to)
                           : pushButton.mode === "diverged"
                             ? qsTr("Hold to overwrite %1, dropping %n commit(s) it has (as of the last fetch)", "",
                                    topBar.curPage.pageWt.behind).arg(to)
                           : ""
                // What git said, under what the button would do next —
                // the mark says only that the last go failed, and this is
                // the one place with room for why (the same two-part
                // tooltip a failed fetch carries).
                const why = topBar.curPage.pushFailReason
                if (pushButton.failed && why !== "")
                    return what + "\n\n" + why
                return what
            }
            onActivated: topBar.curPage.pushNow()
        }
        // Where what the app owns ends and what the window owns begins.
        // Without it the two groups read as one row of controls with an
        // odd gap in it — the same mark a browser puts in the same place.
        // `iconMd` tall rather than the whole band: a rule that reached
        // the edges would be a second divider, and the one under the band
        // already says where it stops.
        Rectangle {
            visible: topBar.captionMerged
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconMd
            color: Theme.borderDefault
        }
        // The window's own three. Drawn here rather than left to the
        // platform: its own are a fixed 32px in a 40px band, with a hover
        // and glyphs that cannot be styled and a maximize mark that never
        // becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
        WindowButton {
            id: minimizeButton
            visible: topBar.captionMerged
            kind: "window-minimize"
            Accessible.name: qsTr("Minimize")
            onTriggered: topBar.minimizeRequested()
        }
        WindowButton {
            visible: topBar.captionMerged
            kind: topBar.windowMaximized ? "window-restore" : "window-maximize"
            Accessible.name: topBar.windowMaximized ? qsTr("Restore") : qsTr("Maximize")
            onTriggered: topBar.maximizeToggleRequested()
        }
        WindowButton {
            visible: topBar.captionMerged
            kind: "close"
            danger: true
            Accessible.name: qsTr("Close")
            onTriggered: topBar.closeRequested()
        }
    }
}
