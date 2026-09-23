pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

Rectangle {
    id: topBar

    required property var tabsModel
    // The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether this band is the window's title bar. When it is, the band carries the window's own buttons and tells the
    /// platform where its empty runs sit — the gestures on those (drag, snap, the double-click, the window menu) are
    /// the platform's own, because the hit test calls them caption (`AppBackend.setCaptionStrips`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the window's own buttons, or the run of band
    /// left to take hold of, or push either off the end — and none of that shows in a screenshot taken where the
    /// platform draws no buttons at all. The strip's readings come back through the band (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    /// …and the band's other run, which a picture holds no better: the divider is drawn the same whether or not the
    /// hit test was ever told about the stretch it stands in.
    readonly property real bandDividerRun: dividerRun.width
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    readonly property real bandTabsWidth: tabStrip.tabsWidth
    readonly property int bandTabCount: tabStrip.tabCount
    readonly property real bandTabRun: tabStrip.runAvail
    readonly property real bandTabContent: tabStrip.contentWidth
    readonly property bool bandTabScrolls: tabStrip.contentWidth > tabStrip.tabsWidth
    /// …and the stand-in for the tab in front, which a picture of a scrolled strip cannot be read for
    /// (`TabStrip.tabPinShown`).
    readonly property bool tabPinShown: tabStrip.tabPinShown
    readonly property bool tabPinRidesLeft: tabStrip.tabPinRidesLeft
    readonly property bool tabPinNameKept: tabStrip.tabPinNameKept
    readonly property bool frontTabWhole: tabStrip.frontTabWhole
    readonly property bool tabRunTravelling: tabStrip.runTravelling
    /// Automation: the five things that can be the matter here. The group owns the conditions and the hidden
    /// measurements behind them; what the hooks ask the band for comes back through here (`BandStateGroup`).
    readonly property bool opBadgeShown: stateGroup.opBadgeShown
    readonly property bool conflictBadgeShown: stateGroup.conflictBadgeShown
    readonly property bool identityBadgeShown: stateGroup.identityBadgeShown
    readonly property bool oldGitBadgeShown: stateGroup.oldGitBadgeShown
    readonly property bool staleBadgeShown: stateGroup.staleBadgeShown
    readonly property real stateBadgeMinW: stateGroup.stateBadgeMinW

    /// Automation: which of the group's three shapes is on screen, what the badges were narrowed to, and what the card
    /// came back with (`PGG_AUTO_ACT=badges` / `badges-hover`). The group makes them; the band is where they are read.
    readonly property bool stateWordsShown: stateGroup.stateWordsShown
    readonly property bool stateMarkShown: stateGroup.stateMarkShown
    readonly property color stateMarkColor: stateGroup.stateMarkColor
    readonly property int stateCapW: stateGroup.stateCapW
    readonly property int stateGroupW: stateGroup.stateGroupW
    readonly property bool stateCardOpen: stateGroup.stateCardOpen
    /// Whether the row has ever placed the group — the third of the three things the stand-in pointer needs before a
    /// card can come up (`BandStateGroup.standInAsking`), and the only one of them a verb cannot otherwise read. A run
    /// that stood the pointer in and never saw a card is two different faults depending on this.
    readonly property bool statePlaced: stateGroup.placed
    readonly property string stateCardRows: stateGroup.stateCardRows
    readonly property string stateCardSize: stateGroup.stateCardSize
    readonly property bool stateCardLaidOut: stateGroup.stateCardLaidOut
    /// Whether the window is standing on its floor. Handed in, because the floor is the larger of this band's and the
    /// page's and only the body that seats both has it (`WindowBody`). The group gives up its words there whatever
    /// else is true.
    property bool windowAtFloor: false
    /// The narrowest the window is laid out at **with the left list open**, whether or not it is open now
    /// (`Main.floorWidth` measured on `RepoPage.openFloorWidth`). The three actions finish giving their words up
    /// exactly there. The open floor: folding the list lowers the real floor, and
    /// a schedule read off that would put the words back on screen as the rail took the list's place — a thing nobody
    /// asked to see move (規約 §窓の床).
    property real windowFloorWidth: 0
    /// Stands in for the pointer where headless cannot put one, so the card can be photographed (`badges-hover` /
    /// `identity-tip`). The real hover writes this same one property — hover is the input that cannot be injected, so
    /// the card has to be answering a single question or the headless run proves nothing about it.
    property bool statePointedAt: false
    /// Whether the fetch button can be pressed at all — the edge `fetch-tip` waits on, since what it reads is only
    /// worth anything once the band has settled on an answer.
    readonly property bool fetchLive: fetchButton.enabled

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()
    /// The grab-run moved or changed size in this band's own layout. `Main` folds in the shifts this band cannot see
    /// from here (the maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()
    /// The two runs themselves, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: tabStrip.grabRunItem
    readonly property Item dividerRunItem: dividerRun
    /// Whether the ☰'s card is standing (`TabStrip`). The runs are the parts of the band a press never reaches, so it
    /// hands them back to the scene while the card is up (`WindowChrome.captionYielded`).
    readonly property bool appMenuOpen: tabStrip.appMenuOpen

    /// The narrowest this band can be laid out at — one of the two numbers the window's floor is the larger of
    /// (`Main.floorWidth`). Read off the row's *minimum*, so a control added to the band later
    /// is counted without anybody remembering to.
    readonly property real floorWidth: bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin

    /// Keeps the two actions' waiting visual up past the operation that raised it. Written from outside and false
    /// wherever nobody wrote it: a fast remote is done before a picture of the wait can be grabbed, and the wait is
    /// what the pair of buttons was drawn for — the framed one and the bare one
    /// (デザイン規約 §進行中・長押しの定数).
    property bool holdPushBusy: false
    property bool holdFetchBusy: false
    readonly property string pushMode: pushButton.mode
    /// Automation: what the Stash button read the working tree as, and the edge `stash-state` waits on. The reading
    /// is a binding over a HEAD and four counts, and a photograph of a dim button carries none of them — every
    /// refusal frames the same way.
    readonly property string stashMode: stashButton.mode
    readonly property bool stashLive: stashButton.enabled
    /// Whether either action has a string to open under a pointer. **The string itself** — a
    /// disabled control takes hover and opens its attached ToolTip like any other one (rules-refs/app-ui.md §hover),
    /// so what tells the two sides apart is whether there is anything to open (`fetch-tip`).
    readonly property bool stashTipShown: stashButton.tip !== ""
    readonly property bool fetchTipShown: fetchButton.tip !== ""
    /// Automation: the pointer's stand-in on the fetch button, and the two things the button answers it with — the tip
    /// standing (the words on screen, `tipDelayMs` waited out) and the word come back to full (`fetch-hover`).
    property alias fetchPointedAt: fetchButton.pointedAt
    readonly property bool fetchTipStanding: fetchButton.tipShown
    readonly property bool fetchWordFull: Qt.colorEqual(fetchButton.fg, Theme.textPrimary)
    /// The same stand-in on the push button, and its tip standing (`push-hover`).
    property alias pushPointedAt: pushButton.pointedAt
    readonly property bool pushTipStanding: pushButton.tipShown
    /// How the fetch button stands: how many fetches have failed, and whether it is wearing a frame. `framed` is
    /// read because "a button that never wore a frame grows none while it waits" (デザイン規約 §進行中・長押しの定数) is a
    /// claim about a line that is not there, and a picture cannot be judged on the absence of one.
    readonly property int fetchFails: fetchButton.fails
    readonly property bool fetchFramed: fetchButton.framed
    /// Automation: the Stash button, pressed (`PGG_AUTO_ACT=stash`). Put in at the button,
    /// so what answers is the band's real wiring. **Answers whether the
    /// press went in**: the band refuses it while the tab is busy — the fetch a repository does on the way open is one
    /// — and a shot fired at nothing is not one to latch, so the caller keeps offering it.
    function stashNow() {
        if (!stashButton.enabled)
            return false
        stashButton.clicked()
        return true
    }
    /// Automation: the Fetch button, pressed (`PGG_AUTO_ACT=fetch-resume`). Put in at the button for the reason
    /// `stashNow` is: the stopped button is the one place that asks for the timer back, so a run that called the slot
    /// behind it would pass a build where the press no longer reaches it. Answers whether the press went in.
    function fetchNow() {
        if (!fetchButton.enabled)
            return false
        fetchButton.clicked()
        return true
    }
    function completePushHold() {
        pushButton.completeHold()
    }

    /// Automation: the strip's own hooks, handed on. What `Main` and `WindowAutoActDriver` hold is the band, so the way
    /// in stays here after the tabs themselves have gone (`TabStrip`).
    function clickAppMenu() { tabStrip.clickAppMenu() }
    function clickCloneRow() { tabStrip.clickCloneRow() }
    function middleClickTab(index) { return tabStrip.middleClickTab(index) }
    function dragTabTo(from, to) { return tabStrip.dragTabTo(from, to) }
    function holdTabAt(index) { return tabStrip.holdTabAt(index) }
    function heldTabShift() { return tabStrip.heldTabShift() }
    function carryTabPastEnd(index) { return tabStrip.carryTabPastEnd(index) }
    function heldTabIndex() { return tabStrip.heldIndex() }
    function dropCarriedTab() { tabStrip.dropTab() }
    function runAtEnd() { return tabStrip.runAtEnd() }
    function runOffset() { return tabStrip.runOffset() }
    function sendTabRunAway() { return tabStrip.sendRunAway() }
    function pressTabPin() { return tabStrip.pressTabPin() }
    /// The strip's own list, passed on for the harness's probe (`TabStrip.tabsView`).
    readonly property alias tabsView: tabStrip.tabsView

    /// The word all three toolbar buttons are measured for: one box for the set keeps any of them from shifting the
    /// others, and which wording is wider is a question about the installed fonts. Settled once: a
    /// binding that reads four text metrics and feeds three button widths is a loop as far as the engine is concerned.
    property string widestAction: ""
    property bool widestActionCode: false

    // ---- how the three give way as the band runs short -----------------
    // The band narrows in the order §ウィンドウの縁 sets out: the tab names and the state words give together, the strip
    // scrolls, the group becomes a mark — and these three narrow with them and then give their words up altogether.
    /// What one of them holds between the box the set shares and the band's own end-cell width. All three are the same
    /// width by construction (one box, one seat), so one of them answers for the set.
    readonly property real actionGive: fetchButton.naturalWidth - Theme.railWidth
    /// The floor a wording is cut down to — two characters of the family this band says its wordings in
    /// (`BandWidest.wordFloor`).
    readonly property real actionWordFloor: widest.wordFloor
    /// The narrowest cell that still holds a word, for the **set**: the widest of the three floors, since a cell that
    /// only fits the shortest wording's floor cuts the longest one past its own (`ActionButton.foldWidth`). It moves
    /// with what the buttons are saying — a longer wording gives up sooner, which is the rule the shared box is under.
    readonly property real actionFold: Math.max(fetchButton.foldWidth,
                                                pushButton.foldWidth,
                                                stashButton.foldWidth)
    /// The widest a button may be drawn at the width the window is standing at now. **The three give what they have
    /// over the last of the window's own travel**: their room between them is exactly what a window narrowing towards
    /// its floor has left to take, so that is the stretch the giving is spread over. One stretch above the floor every
    /// wording is whole; on the floor every one of them is a mark in an end cell; between the two the cap comes down
    /// evenly and each wording elides into what is left (`ActionButtonLabel.cap`).
    ///
    /// Read off the window's width: the row's leftover is a question about how many tabs are open and how long their
    /// names are; the width this has to be finished at is the window's. The row still
    /// takes more when the tabs need it — the cap is a ceiling, and the share-out underneath it can come down to the
    /// end cell on its own.
    readonly property real actionCap:
        topBar.actionGive <= 0 ? fetchButton.naturalWidth
        : Math.max(Theme.railWidth,
                   Math.min(fetchButton.naturalWidth,
                            fetchButton.naturalWidth
                            - (topBar.windowFloorWidth + 3 * topBar.actionGive - topBar.width) / 3))
    /// Where the words go. Said once for the set: three cells the row rounded differently come out in one
    /// shape.
    readonly property bool actionsFolded: topBar.actionCap < topBar.actionFold

    /// Automation: the two ends of the cap's travel and what the band made of it at this width (`band-actions`). A
    /// photograph says which shape landed but not which arithmetic put it there, and the widths the three shapes sit
    /// at are a question about the installed fonts.
    readonly property int actionNaturalW: Math.round(fetchButton.naturalWidth)
    readonly property int actionFoldW: Math.round(topBar.actionFold)
    readonly property int actionCapW: Math.round(topBar.actionCap)
    readonly property int actionWordFloorW: Math.round(topBar.actionWordFloor)
    /// The widest wording actually **on** the band right now: the
    /// box holds every state's wording, and the run that has to land between "cut" and "given up" has to aim at the
    /// one being said (`band-actions`).
    readonly property int actionWantW: Math.round(Math.max(fetchButton.wordWant,
                                                           pushButton.wordWant,
                                                           stashButton.wordWant))
    /// The cell width at which that wording starts being cut: itself, plus everything the cell holds around a word.
    readonly property int actionCutW:
        Math.round(fetchButton.naturalWidth - fetchButton.wordBox + topBar.actionWantW)
    /// …and what one button came out as. Read off push — the one that says the longest wording and wears the frame,
    /// the `!` and the hold when its branch has diverged (`BandPushButton`).
    readonly property int actionCellW: Math.round(pushButton.width)
    readonly property int actionCellH: Math.round(pushButton.height)
    readonly property int actionWordW: Math.round(pushButton.wordRoom)
    readonly property int actionInkW: Math.round(pushButton.wordInk)
    /// Whether any of the three had to cut its wording. Asked of the set: the cell is shared, so
    /// the widest wording is the first to be cut, and which of the three is saying it is a question about the fonts.
    readonly property bool actionWordCut:
        fetchButton.wordCut || pushButton.wordCut || stashButton.wordCut
    /// Whether either of the two that can wear a `!` is wearing one. Asked of the pair: the run
    /// that can stage a refusal without a network is the fetch that cannot reach its remote (`band-actions-alert`).
    readonly property bool actionAlertShown: fetchButton.alert || pushButton.alert

    Component.onCompleted: {
        let best = widest.fetchCodeWidest
        for (const m of [widest.fetchWordWidest, widest.pushCodeWidest, widest.stashCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // The four candidate wordings, drawn invisibly the way a button would draw them (`BandWidest`).
    BandWidest {
        id: widest
    }

    /// Automation: what the strip made of the run it was handed, and the two ends it was settled between
    /// (`PGG_AUTO_ACT=tab-widths` / `badges`). Re-exposed for the reason the band's other readings are.
    readonly property real tabTitleCap: tabStrip.tabTitleCap
    readonly property int tabTitleMinW: tabStrip.tabTitleMinW
    readonly property real tabTitleEaseW: tabStrip.tabTitleEaseW
    readonly property int tabTitleMaxW: tabStrip.tabTitleMaxW
    /// The room every tab is keeping for its mark, which is what the strip gives up first — the two ends it is settled
    /// between are the strip's own (`TabMetrics.markRoomFull` / `markRoomMin`). A picture of a strip that has given it
    /// all up reads the same as one that has not had to.
    readonly property real tabMarkRoom: tabStrip.tabMarkRoom
    /// And which of the two states that leaves the strip in — the reading a run is judged on, since the count that
    /// reaches either is the platform's answer and not the run's (`TabStrip.tabNamesCut`).
    readonly property bool tabNamesCut: tabStrip.tabNamesCut
    readonly property bool tabMarksFolded: tabStrip.tabMarksFolded

    implicitHeight: Theme.toolbarHeight
    color: Theme.bgElevated

    RowLayout {
        id: bandRow
        anchors.fill: parent
        // The band's own right edge is not the window's: the client area reaches past what is drawn, so a row flush
        // with it puts the close button's last few pixels off screen and its wash reads as clipped. `spaceXs` lands
        // it flush.
        anchors.rightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
        spacing: Theme.spaceXs
        TabStrip {
            id: tabStrip
            tabsModel: topBar.tabsModel
            // The band's own ground, said by the band: a tab that is not in front paints none of its own, so this is
            // what its name is read against — and what the name goes quiet into under its mark.
            bandColor: topBar.color
            captionMerged: topBar.captionMerged
            curPage: topBar.curPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: tabStrip.tabStripFloorW
            // The band's order when it runs short: the tab names narrow together, then the state group's words do,
            // then the strip scrolls, then the group becomes a mark — a stretch this much larger than the group's puts
            // the strip at the front of both queues.
            Layout.horizontalStretchFactor: 100
            onOpenRepositoryRequested: topBar.openRepositoryRequested()
            onCloneRepositoryRequested: topBar.cloneRepositoryRequested()
            onSettingsRequested: topBar.settingsRequested()
            // The Exit row is the ✕ by another name: one signal out of the band, so the window has one close gate.
            onExitRequested: topBar.closeRequested()
            onCaptionStripMoved: topBar.captionStripMoved()
        }

        BandStateGroup {
            id: stateGroup
            curPage: topBar.curPage
            windowAtFloor: topBar.windowAtFloor
            pointedAt: topBar.statePointedAt
            // The group folds off the strip's width as well as its own, and the two do not always change together.
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: tabStrip.runAvail
            tabCount: tabStrip.tabCount
            // Measured off the pair beside it: what sets how big a target is
            // here is the padding a Fusion `ToolButton` keeps around its content — a number the theme does not have.
            // The button's `padding`: the two sides it settles to carry the shared box's slack as
            // well (`ActionButton.slack`), so reading them would move this group whenever fetch changed its wording.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            cellFolded: topBar.actionsFolded
            Layout.fillWidth: true
            Layout.fillHeight: stateGroup.cellFolded
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the order).
            Layout.horizontalStretchFactor: 1
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // Fetch, and everything the network has to say about fetching (`BandFetchButton`).
        BandFetchButton {
            id: fetchButton
            curPage: topBar.curPage
            busyLatched: topBar.holdFetchBusy
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            wordFloor: topBar.actionWordFloor
            foldRequested: topBar.actionsFolded
            // Laid out by the row, so the band can take its width back as it
            // runs short. **The three ask for the same three numbers** — the box the set shares, the band's end cell,
            // and the cap the window's width settles — and none of them moves with the shape the button is in, so
            // giving the word up cannot change the width that decided to. The floor is the **folded** width, the way
            // the state group's is (規約 §窓の床「帯の床は畳んだ姿で数える」): one counted with a word still on it would rise
            // the moment the words came back, and a window standing on it would be grown by its own band.
            Layout.fillWidth: true
            Layout.fillHeight: fetchButton.folded
            Layout.preferredWidth: fetchButton.naturalWidth
            Layout.minimumWidth: Theme.railWidth
            Layout.maximumWidth: topBar.actionCap
        }
        // Push, in whichever shape this branch's standing with its remote allows (`BandPushButton`).
        BandPushButton {
            id: pushButton
            curPage: topBar.curPage
            busyLatched: topBar.holdPushBusy
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            wordFloor: topBar.actionWordFloor
            foldRequested: topBar.actionsFolded
            Layout.fillWidth: true
            Layout.fillHeight: pushButton.folded
            Layout.preferredWidth: pushButton.naturalWidth
            Layout.minimumWidth: Theme.railWidth
            Layout.maximumWidth: topBar.actionCap
        }
        // Everything uncommitted, set aside in one entry, on the press (`BandStashButton`).
        BandStashButton {
            id: stashButton
            curPage: topBar.curPage
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            wordFloor: topBar.actionWordFloor
            foldRequested: topBar.actionsFolded
            Layout.fillWidth: true
            Layout.fillHeight: stashButton.folded
            Layout.preferredWidth: stashButton.naturalWidth
            Layout.minimumWidth: Theme.railWidth
            Layout.maximumWidth: topBar.actionCap
        }
        // Where what the app owns ends and what the window owns begins.
        Rectangle {
            id: chromeDivider
            visible: topBar.captionMerged
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: Theme.borderWidth
            implicitHeight: Theme.iconMd
            color: Theme.borderDefault
        }
        // The window's own three, drawn here: the platform's cannot be styled and its
        // maximize mark never becomes a restore mark (P3-確認事項 §ウィンドウ chrome).
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

    // The run the divider stands in: the line, and the band's own spacing either side of it — which together are the
    // whole stretch between the last thing the app can be asked and the first thing the window can. The hit test
    // answers HTCAPTION for it the way it does for the run past the last tab (`TabStrip.grabArea`), so a press here
    // is the platform's own gesture.
    //
    // Outside the row, so that measuring the row's own layout does not become part of it: a child of a `RowLayout` is
    // laid out, and this one only wants to know where two of the row's items came to rest. `bandRow` fills the band,
    // so its children's coordinates are this item's.
    Item {
        id: dividerRun
        x: bandRow.x + chromeDivider.x - bandRow.spacing
        // Nothing at all while the band is not the title bar: the divider is not drawn there, and a `RowLayout` leaves
        // an item it is not laying out at whatever geometry it last had.
        width: chromeDivider.visible ? chromeDivider.width + 2 * bandRow.spacing : 0
        height: topBar.height
        onXChanged: topBar.captionStripMoved()
        onWidthChanged: topBar.captionStripMoved()
        Component.onCompleted: topBar.captionStripMoved()
    }
}
