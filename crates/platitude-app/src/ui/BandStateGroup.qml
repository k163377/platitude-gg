import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The state badges, giving way as the tab names do: whole words, narrowed together, one mark. The card the pointer
// opens carries all of it at every step (`BandStateCard`). Which states the band says, and in which colour, is
// 規約 §ウィンドウの縁; how they are drawn and give way is this file's.
//
// **Order**: the operation under way, `CONFLICTS`, `SET IDENTITY`, `STALE GRAPH`, `NO LFS`, `OLD GIT`. `OLD GIT` is the
// environment's mark and stands until git is replaced, so it comes last; the five before it say something is in the
// middle of something — `STALE GRAPH` goes with the next reading that lands, `NO LFS` with its files leaving the list.
//
// **Three steps as the band narrows** (the tabs give way at the same time — `TopBar`'s tab strip):
//   1. While there is room, the words whole: the group's ceiling is the natural width of the words standing.
//   2. Short of it, the whole group narrows together to its first letters and a `…`, on the tab names' max-min share
//      (`BandStateShare.cap`: only the long give, and those giving come out at one width) — floored in characters as
//      a tab is, but lower (`stateMinChars`).
//   3. Under that floor, or once the tabs are cramped (`BandStateShare.folded`), the whole group folds into one `…`.
// Tuned on shapes an ordinary setup stands in: `OLD GIT` goes once git is replaced, and fitting the giving way to it
// would fit it to what most windows never have. So: with one tab the group does not narrow (the one name asks for no
// more than itself), and three states beside a strip of long names stay words above the floor — the names stop at
// their floor and scroll, leaving the group no one to give to.
//
// A badge added touches: its word in `Words`; here its `*BadgeShown`, `*BadgeW`, `naturalWidth`, `settleCap`'s `want`
// and its `StateBadge`, and `tint` if it is outlined in `danger`; `BandStateMetrics`' measuring `BadgeWord` and `*W`;
// `BandStateCard`'s row, `badgeRun` and `rowsLaidOut`; and `TopBar`'s automation exposure.
//
// What answers for it, one layer each: the share-out's table is `tst_bandshare.qml`; the wiring from the share-out to
// what is drawn, which model value raises which badge, and the card waiting for a hand the marks came under, is
// `tst_bandgroup.qml` (this group built for real, a real `BandStateMetrics` beside it as witness, a real pointer for
// the hand); the stopped operation's four words and bisect are
// `models::worktree::tests`. The table alone is not enough: with the group's `badgeMinW` wiring pinned to 0,
// `tst_bandshare` and the three UI verbs still pass. Headless: `badges` / `badges-hover` / `badges-hover-early`
// (`identity-tip` reads the same card).
Item {
    id: stateGroup

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether the window stands on its floor (only `Main` knows); the group folds there whatever else is true.
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one (`badges-hover` / `identity-tip`). It meets the real
    /// hover in `stateKeep`, so the card answers one question either way.
    property bool pointedAt: false
    /// The hand over the band (`TopBar.bandHand`). The group moves under a hand that does not: a read stopping an
    /// operation adds badges, a narrower window folds them (`movedUnderHand`).
    property HandWatch hand: null
    /// What the tab strip made of its own run — the group folds off the strip's width as well as its own, and the two
    /// do not always change in the same frame.
    property real tabContentWidth: 0
    property real tabRunAvail: 0
    property int tabCount: 0
    /// The padding of the operation panel's buttons, handed in: the theme has no number for a Fusion `ToolButton`'s
    /// padding, so the folded mark's width borrows theirs. Handed in as that relation ("as the buttons"), never a
    /// copied number — a copy goes quietly wrong when the other side moves.
    property real controlPadding: 0
    /// The operation panel's buttons have folded into end cells (`TopBar.actionsFolded`), and the mark follows them
    /// into a `railWidth` cell with the wash over the whole cell, as theirs (`stateToggle`). Off the panel's fold,
    /// which is a function of the window's width: the group's `Layout.minimumWidth` moving with it is no ring with the
    /// row's share-out, where one read off this group's own laid-out width would be.
    property bool cellFolded: false
    /// Where the band's line of words stands, in this group's y: the tab names' baseline (`TabStrip.titleBaseline`).
    /// The badges' words stand on it with them, and the boxes are cut round the words from there (`BandStateMetrics`'
    /// badge box).
    required property real lineBaseline

    /// What can be the matter here, one expression each, read by both the mark and the card (`BandStateCard`) so the
    /// two cannot disagree about whether there is anything to open.
    readonly property var stateWorktree: stateGroup.curPage !== null ? stateGroup.curPage.pageWorktree : null
    readonly property bool opBadgeShown: stateGroup.stateWorktree !== null
                                         && stateGroup.stateWorktree.opText !== ""
    readonly property bool conflictBadgeShown: stateGroup.stateWorktree !== null
                                               && stateGroup.stateWorktree.hasConflicts
    /// Also a save whose halves did not both land: the identity is then set, but not as asked — the gate closed over a
    /// half hands it here. Still `SET IDENTITY` (the same operation keeps its word); only the tooltip tells the half
    /// apart. For this session only: carried over, it would be a copy of the identity the app does not keep.
    readonly property bool identityBadgeShown: AppBackend.identityState === "missing" || AppBackend.identityUnsaved
        || (stateGroup.curPage !== null && !stateGroup.curPage.pageTab.identityReady)
    /// Headless: `old-git` / `old-git-card` / `old-git-fold`, on `git_shim`'s old version.
    readonly property bool oldGitBadgeShown: AppBackend.gitUnsupported
    /// The graph and the repository disagree, either way (user decision — ask before changing): the walk stopped
    /// part-way and rows are missing (`failed`), or every row is there but the rebuild meant to replace them did not
    /// land, so all are old (`stale`). The reader stands the same in both — nothing on screen can be taken as now — so
    /// one word, and the card's line tells them apart (`BandStateCard`'s `staleRow`). `danger` either way (規約 §状態
    /// 「今止まっている・失敗した」), so the folded mark turns red too (`tint`): two colours on one word would give a
    /// mark that says nothing but its colour two weights. Outlined, not filled — the fill is `CONFLICTS`', files
    /// waiting on the reader; this one is only read. Not the window's cut (`Only the first N commits are loaded`): that
    /// walk ran to the end and stops by design, in `textSecondary`.
    ///
    /// **Said by this badge alone.** In the middle of the column the words would sit over the rows that did arrive; at
    /// the foot (the window's cut line) they are off screen at opening, the first chunk being 512 rows. One state, one
    /// place — the band is always in view, and git's words go in the badge's card. `failed` is one state whether part
    /// of the walk arrived or none of it. Headless: `graph-stale` / `graph-stopped` (`WindowBadgeActs`).
    readonly property bool staleBadgeShown: stateGroup.curPage !== null
                                            && (stateGroup.curPage.pageGraph.failed
                                                || stateGroup.curPage.pageGraph.stale)
    /// Pending files need Git LFS and git cannot run it here (`WorktreeModel.lfsNeeded`; what counts and when it is
    /// asked again: `platitude_core::lfs`, `session/lfs.rs`). `warning`'s outline: the app is saying it, and nothing
    /// has happened yet (規約 §状態). Not pressable and no road out, as `OLD GIT`: installing is work outside this
    /// window. Gone on the next tick after an install. The count rides in with the status
    /// (`SessionEvent::StatusLoaded::lfs_needed`), so nothing here counts or asks. Wiring: `tst_bandgroup.qml`;
    /// headless `no-lfs` / `no-lfs-card` (`--preset lfs`) and all six at once, `badges-all` / `badges-all-hover`
    /// (`--preset lfs-conflict`). The desk's Git for Windows carries LFS, so a git without it is `git_shim --no-lfs`
    /// (picked by the verb's name).
    readonly property bool lfsBadgeShown: stateGroup.stateWorktree !== null
                                          && stateGroup.stateWorktree.lfsNeeded > 0
    readonly property bool stateShown: stateGroup.opBadgeShown || stateGroup.conflictBadgeShown
                                       || stateGroup.identityBadgeShown || stateGroup.oldGitBadgeShown
                                       || stateGroup.staleBadgeShown || stateGroup.lfsBadgeShown
    /// The narrowest a badge is drawn before the group gives up on words, in characters: two and a `…`, under the tab
    /// names' floor — capital words are told apart by two letters (user decision).
    readonly property int stateMinChars: 2
    /// Settled where it is measured (`BandStateMetrics`).
    readonly property real stateBadgeMinW: badgeMetrics.minW
    readonly property bool stateHasAlso: stateGroup.stateWorktree !== null
                                         && stateGroup.stateWorktree.opAlso !== ""
    readonly property bool stateHasStep: stateGroup.stateWorktree !== null
                                         && stateGroup.stateWorktree.opSteps > 0
    /// What each badge would like to be, measured off labels that are never drawn (`BandStateMetrics`).
    readonly property int opBadgeW: badgeMetrics.opW
    readonly property int conflictBadgeW: badgeMetrics.conflictW
    readonly property int identityBadgeW: badgeMetrics.identityW
    readonly property int oldGitBadgeW: badgeMetrics.oldGitW
    readonly property int staleBadgeW: badgeMetrics.staleW
    readonly property int lfsBadgeW: badgeMetrics.lfsW

    /// Automation: which of the three shapes is on screen, what the badges were narrowed to, and the card — what the
    /// band made of the state (`PGG_AUTO_ACT=badges` / `badges-hover`).
    readonly property bool stateWordsShown: badgeRow.visible
    readonly property bool stateMarkShown: stateToggle.visible
    /// The colour the mark was painted with, read off the paint so a check sees the rule reach it (as
    /// `commandsMarkColor`).
    readonly property color stateMarkColor: stateGroup.tint
    readonly property int stateCapW: stateGroup.cap === Number.MAX_VALUE ? -1 : Math.round(stateGroup.cap)
    readonly property int stateGroupW: Math.round(stateGroup.width)
    readonly property bool stateCardOpen: stateCard.opened
    readonly property string stateCardRows: stateCard.rowsLaidOut()
    readonly property string stateCardSize: stateCard.laidOutSize
    readonly property bool stateCardLaidOut: stateCard.laidOut

    /// The folded mark's colour — the heaviest state's: `danger` when a conflict or a stale graph is among them, else
    /// `warning` (規約 §状態). The mark says nothing but its colour, so its name is `Accessible.name`.
    readonly property color tint: stateGroup.conflictBadgeShown || stateGroup.staleBadgeShown
                                  ? Theme.danger : Theme.warning
    /// The group's ceiling; `foldedWidth` below is its floor, and the window's floor is costed at the mark
    /// (`TopBar.floorWidth`).
    readonly property real naturalWidth: (stateGroup.opBadgeShown ? stateGroup.opBadgeW + Theme.spaceXs : 0)
        + (stateGroup.conflictBadgeShown ? stateGroup.conflictBadgeW + Theme.spaceXs : 0)
        + (stateGroup.identityBadgeShown ? stateGroup.identityBadgeW + Theme.spaceXs : 0)
        + (stateGroup.staleBadgeShown ? stateGroup.staleBadgeW + Theme.spaceXs : 0)
        + (stateGroup.lfsBadgeShown ? stateGroup.lfsBadgeW + Theme.spaceXs : 0)
        + (stateGroup.oldGitBadgeShown ? stateGroup.oldGitBadgeW + Theme.spaceXs : 0)
        - Theme.spaceXs
    readonly property real foldedWidth: stateGroup.cellFolded
        ? Theme.railWidth : stateMark.implicitWidth + 2 * stateGroup.controlPadding
    /// The folded mark's frame: as deep as the badges it stands for, and where they stand — folding gives up the words,
    /// not the box; only the width changes, as the panel's buttons fold (`ActionButton.foldedDepth`). The cell runs the
    /// band's depth; a frame that deep lies along the band's top and bottom edges and reads as a box glued to the
    /// window. Always framed: the mark stands only while something is the matter.
    readonly property real foldedDepth: opBadge.height
    /// Where every badge's box stands: its words' baseline on the band's line. Whole pixels, so the frame's lines stay
    /// sharp.
    readonly property int boxTop: Math.round(stateGroup.lineBaseline) - badgeMetrics.wordBase
    /// Each badge box's ceiling once they give way together; `Number.MAX_VALUE` is "nothing is narrowed". Settled by
    /// hand (`settleCap`).
    property real cap: Number.MAX_VALUE
    /// The room the words are narrowed into: what the band would hand the group with its words (`TopBar`'s
    /// `bandAsked`). Not the group's own width once folded — that would keep it folded however wide the window grew.
    property real room: stateGroup.width
    /// Automation: whether a folded group is exactly its mark's width (`old-git-fold`).
    readonly property bool markFitted: !stateGroup.folded || Math.abs(stateGroup.width - stateToggle.width) < 1
    /// Automation: whether a folded mark's frame is a badge's depth (`old-git-fold`). It catches a frame drawn off some
    /// other depth — the cell's, a neighbour's — but follows the badge box wherever that goes, and says nothing of
    /// where the frame stands: both are `tst_bandgroup.qml`'s.
    readonly property bool markDeep: !stateGroup.folded || Math.abs(markFrame.height - opBadge.height) < 0.5
    /// Whether the words are given up. Bound, not settled: two of the conditions change without the width moving.
    /// Every input is read here in the expression — a read inside the share's body takes no dependency.
    readonly property bool folded: share.folded(stateGroup.cap, stateGroup.tabContentWidth,
                                                stateGroup.tabRunAvail, stateGroup.tabCount,
                                                stateGroup.windowAtFloor)

    /// Whether the band has ever placed this group; the card waits for it. The row places no cell that is not
    /// standing, so a cell standing for the first time is still at (0, 0), where the offscreen pointer sits
    /// (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」): the hover taken there would open the card over a band
    /// nobody touched and close it `hoverKeepMs` later — a different picture every run. Placed under a hand already
    /// there, the card waits for that hand to move. Kept once said: the row leaves a cell where it put it. Never make
    /// this group the row's first cell — the origin would be its place, and the card would never open
    /// (`badges-hover`). Only the automation's stand-in (`pointedAt`) is asked again once this turns
    /// (`badges-hover-early`, the pointer put down before the group is placed).
    property bool placed: false
    onXChanged: {
        stateGroup.placed = true
        stateGroup.movedUnderHand()
    }
    onWidthChanged: stateGroup.movedUnderHand()

    /// The marks came under whatever hand is on the band without its moving: what they offer waits for its next move
    /// (デザイン規約 §hover のツールチップ「手が動いたかの見切り」 — the state card is named there). The row lays the group
    /// out in its polish, before the next frame hands hover to what stands under a still pointer.
    function movedUnderHand() {
        if (stateGroup.hand !== null)
            stateGroup.hand.rowsMoved()
    }

    signal identityEditRequested()

    /// Automation: `BandStateCard.wordsOffLevel`, for the band to hand on.
    function stateCardWordsOff() { return stateCard.wordsOffLevel() }

    /// The same max-min share the tab names are settled with (`TabStrip.settleTitleCap`), settled by hand for the same
    /// reason.
    function settleCap() {
        let want = []
        if (stateGroup.opBadgeShown)
            want.push(stateGroup.opBadgeW)
        if (stateGroup.conflictBadgeShown)
            want.push(stateGroup.conflictBadgeW)
        if (stateGroup.identityBadgeShown)
            want.push(stateGroup.identityBadgeW)
        if (stateGroup.staleBadgeShown)
            want.push(stateGroup.staleBadgeW)
        if (stateGroup.lfsBadgeShown)
            want.push(stateGroup.lfsBadgeW)
        if (stateGroup.oldGitBadgeShown)
            want.push(stateGroup.oldGitBadgeW)
        stateGroup.cap = share.cap(want, stateGroup.room)
    }

    /// The pointer, or the stand-in, is on a group that can answer it: standing, and placed — a pointer that cannot
    /// move, written before `placed`, would never ask again. The card unfolds what the mark stands for, so it opens
    /// at once (規約 §hover のツールチップ「展開は即時に開く」; this card on that side of the line is a user decision —
    /// ask before changing; a cut name's whole text stays on the other, `TipSeat`); the seat closes it a beat after the
    /// last thing asking for it lets go, and does not put it back under the same still hand once dismissed. **A parked
    /// hand asks nothing**
    /// (`Hand.away`, as every `HandWatch` reads it): a headless run gets Qt's enter at the offscreen cursor while the
    /// band has not placed this group yet, and the hover that leaves behind would raise the card for a beat in a run
    /// that put no hand here — a card in the census by the luck of the frame.
    readonly property bool asking: ((groupHover.hovered && !Hand.away) || stateGroup.pointedAt)
                                   && stateGroup.placed && stateGroup.stateShown
    onAskingChanged: {
        if (stateGroup.asking)
            stateKeep.point({ "key": "state" }, true)
        else
            stateKeep.unpoint("state")
    }
    // The state the card speaks for went (the operation finished, the conflicts resolved): the card goes as the
    // owner's close, the hand inside it or not.
    onStateShownChanged: {
        if (!stateGroup.stateShown)
            stateKeep.takeDown("gone", false)
    }

    onRoomChanged: stateGroup.settleCap()
    onTabRunAvailChanged: stateGroup.settleCap()

    visible: stateGroup.stateShown
    implicitHeight: stateGroup.foldedDepth
    // The row shares out by what each item asks for above its floor, so ask for the words — and folded, for the mark
    // alone, or the words' width stands empty beside it (`TopBar`'s tab strip: the two give way at once).
    implicitWidth: stateGroup.folded ? stateGroup.foldedWidth : stateGroup.naturalWidth

    readonly property HoverSeat stateKeep: HoverSeat {
        id: stateKeep
        card: stateCard
        // The card opens the moment the hand is on the marks — once the hand has moved, when the marks came under it
        // (`movedUnderHand`). A card the reader dismissed comes back with the hand leaving the group and coming back,
        // or, folded, a press on the `…` (`stateMouse`, `HoverSeat.expands`).
        hand: stateGroup.hand
        expands: true
        // Behind any menu in the window — a page's right-click, the ☰ beside it — the card goes the moment it stands
        // and nothing opens while it does (デザイン規約 §メニュー「メニューが立っている間、その裏の hover は引っ込む」).
        blockedBy: OutsidePress.standsOver(stateGroup) ? "menu" : ""
        onOpenAsked: stateCard.open()
    }

    BandStateMetrics {
        id: badgeMetrics
        stateWorktree: stateGroup.stateWorktree
        hasAlso: stateGroup.stateHasAlso
        hasStep: stateGroup.stateHasStep
        minChars: stateGroup.stateMinChars
    }
    /// The share-out (`BandStateShare`). Its floor is bound to the metrics' property, which is settled by assignment
    /// and so notifies.
    readonly property BandStateShare share: BandStateShare {
        id: share
        gap: Theme.spaceXs
        badgeMinW: badgeMetrics.minW
    }

    // A handler: it is passive, so the identity badge under it still takes its own press.
    HoverHandler {
        id: groupHover
    }

    // Right-aligned: the edge by `>_` stays put while the group gives way to the tabs.
    Row {
        id: badgeRow
        visible: !stateGroup.folded
        anchors.right: parent.right
        y: stateGroup.boxTop
        spacing: Theme.spaceXs

        StateBadge {
            id: opBadge
            box: badgeMetrics
            visible: stateGroup.opBadgeShown
            naturalW: stateGroup.opBadgeW
            cap: stateGroup.cap
            Label {
                text: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opText : ""
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                // The one part of this badge that gives: the count and the second operation mean nothing cut in half.
                elide: Text.ElideRight
                Layout.fillWidth: true
                // Rounded up, here and below (rules-refs/app-ui.md「自然幅の上限は切り上げる」).
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
            // Drawn — a typed middle dot would put a full-width cell in the badge (規約 §余白). On the capitals' middle,
            // not the line's: the words beside it are capitals only.
            DotMark {
                id: opDot
                visible: stateGroup.stateWorktree !== null && stateGroup.stateWorktree.opAlso !== ""
                tint: Theme.warning
                Layout.alignment: Qt.AlignTop
                Layout.topMargin: badgeMetrics.capMiddle - opDot.implicitHeight / 2
            }
            Label {
                visible: stateGroup.stateWorktree !== null && stateGroup.stateWorktree.opAlso !== ""
                text: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opAlso : ""
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
            }
            Label {
                visible: stateGroup.stateWorktree !== null && stateGroup.stateWorktree.opSteps > 0
                text: stateGroup.stateWorktree === null ? ""
                      : qsTr("%1/%2").arg(stateGroup.stateWorktree.opStep).arg(stateGroup.stateWorktree.opSteps)
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
            }
        }
        StateBadge {
            id: conflictBadge
            box: badgeMetrics
            visible: stateGroup.conflictBadgeShown
            naturalW: stateGroup.conflictBadgeW
            cap: stateGroup.cap
            filled: true
            Label {
                text: Words.badgeConflicts
                color: Theme.textOnAccent
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
        }
        StateBadge {
            id: identityBadge
            box: badgeMetrics
            visible: stateGroup.identityBadgeShown
            naturalW: stateGroup.identityBadgeW
            cap: stateGroup.cap
            pressable: true
            onPressed: stateGroup.identityEditRequested()
            Label {
                text: Words.badgeSetIdentity
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
        }
        // Outlined `danger`, and ahead of `OLD GIT` (`staleBadgeShown`, the header's order).
        StateBadge {
            id: staleBadge
            box: badgeMetrics
            visible: stateGroup.staleBadgeShown
            naturalW: stateGroup.staleBadgeW
            cap: stateGroup.cap
            tint: Theme.danger
            Label {
                text: Words.badgeStaleGraph
                color: Theme.danger
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
        }
        // Cleared by an install as `OLD GIT` is, but ahead of it: it also goes with the files that raised it
        // (`lfsBadgeShown`).
        StateBadge {
            id: lfsBadge
            box: badgeMetrics
            visible: stateGroup.lfsBadgeShown
            naturalW: stateGroup.lfsBadgeW
            cap: stateGroup.cap
            Label {
                text: Words.badgeNoLfs
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
        }
        StateBadge {
            id: oldGitBadge
            box: badgeMetrics
            visible: stateGroup.oldGitBadgeShown
            naturalW: stateGroup.oldGitBadgeW
            cap: stateGroup.cap
            Label {
                text: Words.badgeOldGit
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: Math.ceil(implicitWidth)
            }
        }
    }

    // `…` typed — the same glyph and width on both OSes, unlike the East Asian Ambiguous `↑` `↓` `→` `·`. Outside
    // `Theme.buttonMinWidth`, which is for a box round a word. Hover and a press both open the card: a small target
    // answers however it is touched.
    Rectangle {
        id: stateToggle
        visible: stateGroup.folded
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        implicitWidth: stateGroup.foldedWidth
        // The band's depth is the target; the frame is the badges' box, where they stand (`foldedDepth` / `boxTop`).
        height: stateGroup.height
        width: implicitWidth
        radius: Theme.radiusSm
        // In an end cell the wash fills the cell, as the folded buttons' do; elsewhere it stays in the frame
        // (規約 §QML 実装ルール「広げるのは判定だけ」).
        color: stateGroup.cellFolded && stateMouse.containsMouse ? Theme.bgHover : "transparent"
        Accessible.role: Accessible.Button
        Accessible.name: qsTr("What needs attention here")
        Rectangle {
            id: markFrame
            anchors.left: parent.left
            anchors.right: parent.right
            y: stateGroup.boxTop
            height: stateGroup.foldedDepth
            color: !stateGroup.cellFolded && stateMouse.containsMouse ? Theme.bgHover : "transparent"
            border.width: Theme.borderWidth
            border.color: stateGroup.tint
            radius: Theme.radiusSm
        }
        // On the band's line, as the words it stands for (a glyph, on the line's one baseline): the frame is cut round
        // the words' capitals from that line, and a centred line puts the dots wherever the face's descent leaves them.
        // Where in the line the dots fall is the face's: in Noto Sans JP / Noto Sans CJK JP they come within 1px of the
        // middle of the capital-cut frame. The face's own report of the dots' ink is no help — Noto Sans JP's spans
        // 7px.
        Label {
            id: stateMark
            anchors.horizontalCenter: markFrame.horizontalCenter
            anchors.baseline: parent.top
            anchors.baselineOffset: Math.round(stateGroup.lineBaseline)
            text: "…"
            font.pixelSize: Theme.fontMd
            font.weight: Theme.fontWeightStrong
            color: stateGroup.tint
        }
        MouseArea {
            id: stateMouse
            anchors.fill: parent
            hoverEnabled: true
            // A press opens what the hover opens — a dismissed card too: a press that did nothing would read as a
            // dead control.
            onClicked: stateKeep.press({ "key": "state" })
        }
    }

    BandStateCard {
        id: stateCard
        // Flush under the group's right edge: centred, it would open past the window; a gap would be a band the
        // pointer crosses touching neither card nor mark (規約 §hover のツールチップ).
        x: stateGroup.width - width
        y: stateGroup.height
        box: badgeMetrics
        opText: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opText : ""
        opAlso: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opAlso : ""
        opStep: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opStep : 0
        opSteps: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.opSteps : 0
        conflictCount: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.conflictCount : 0
        identityUnsaved: AppBackend.identityUnsaved
        gitVersion: AppBackend.gitVersion
        minimumGit: AppBackend.minimumGit
        opShown: stateGroup.opBadgeShown
        conflictShown: stateGroup.conflictBadgeShown
        identityShown: stateGroup.identityBadgeShown
        oldGitShown: stateGroup.oldGitBadgeShown
        staleShown: stateGroup.staleBadgeShown
        lfsShown: stateGroup.lfsBadgeShown
        lfsNeeded: stateGroup.stateWorktree !== null ? stateGroup.stateWorktree.lfsNeeded : 0
        staleStopped: stateGroup.curPage !== null && stateGroup.curPage.pageGraph.failed
        staleWhy: stateGroup.curPage !== null ? stateGroup.curPage.pageGraph.error : ""
        onIdentityRequested: stateGroup.identityEditRequested()
    }
}
