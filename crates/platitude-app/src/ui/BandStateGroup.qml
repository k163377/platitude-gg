import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The state badges, giving way as the tab names do (規約 §ウィンドウの縁): whole words, narrowed together, one mark.
// The card the pointer opens carries all of it at every step.
Item {
    id: stateGroup

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether the window stands on its floor (only `Main` knows); the group folds there whatever else is true.
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one (`badges-hover` / `identity-tip`). It meets the real
    /// hover in `stateKeep`, so the card answers one question either way.
    property bool pointedAt: false
    /// What the tab strip made of its own run — the group folds off the strip's width as well as its own, and the two
    /// do not always change in the same frame.
    property real tabContentWidth: 0
    property real tabRunAvail: 0
    property int tabCount: 0
    /// The padding of the operation panel's buttons, handed in: the theme has no number for a Fusion `ToolButton`'s
    /// padding.
    property real controlPadding: 0
    /// The operation panel's buttons have folded into end cells (`TopBar.actionsFolded`), and the mark follows them
    /// (規約 §ウィンドウの縁「畳んだ `…` の箱」).
    property bool cellFolded: false
    /// Where the band's line of words stands, in this group's y: the tab names' baseline (`TabStrip.titleBaseline`).
    /// The badges' words stand on it with them, and the boxes are cut round the words from there
    /// (規約 §ウィンドウの縁「バッジの箱」).
    required property real lineBaseline

    /// What can be the matter here, one expression each, read by both the mark and the card (`BandStateCard`) so the
    /// two cannot disagree about whether there is anything to open.
    readonly property var stateWorkingTree: stateGroup.curPage !== null ? stateGroup.curPage.pageWorkingTree : null
    readonly property bool opBadgeShown: stateGroup.stateWorkingTree !== null
                                         && stateGroup.stateWorkingTree.opText !== ""
    readonly property bool conflictBadgeShown: stateGroup.stateWorkingTree !== null
                                               && stateGroup.stateWorkingTree.hasConflicts
    /// Also a save whose halves did not both land: the identity is then set, but not as asked.
    readonly property bool identityBadgeShown: AppBackend.identityState === "missing" || AppBackend.identityUnsaved
        || (stateGroup.curPage !== null && !stateGroup.curPage.pageTab.identityReady)
    readonly property bool oldGitBadgeShown: AppBackend.gitUnsupported
    /// The graph and the repository disagree, either way (`failed` / `stale`): one badge, the card tells them apart,
    /// and nowhere else says it (デザイン規約 §ウィンドウの縁).
    readonly property bool staleBadgeShown: stateGroup.curPage !== null
                                            && (stateGroup.curPage.pageGraph.failed
                                                || stateGroup.curPage.pageGraph.stale)
    /// Pending files need Git LFS and git cannot run it here (`WorkingTreeModel.lfsNeeded`).
    readonly property bool lfsBadgeShown: stateGroup.stateWorkingTree !== null
                                          && stateGroup.stateWorkingTree.lfsNeeded > 0
    readonly property bool stateShown: stateGroup.opBadgeShown || stateGroup.conflictBadgeShown
                                       || stateGroup.identityBadgeShown || stateGroup.oldGitBadgeShown
                                       || stateGroup.staleBadgeShown || stateGroup.lfsBadgeShown
    /// The narrowest a badge is drawn before the group gives up on words, in characters (規約 §ウィンドウの縁).
    readonly property int stateMinChars: 2
    /// Settled where it is measured (`BandStateMetrics`).
    readonly property real stateBadgeMinW: badgeMetrics.minW
    readonly property bool stateHasAlso: stateGroup.stateWorkingTree !== null
                                         && stateGroup.stateWorkingTree.opAlso !== ""
    readonly property bool stateHasStep: stateGroup.stateWorkingTree !== null
                                         && stateGroup.stateWorkingTree.opSteps > 0
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

    /// The folded mark's colour: `danger` when a conflict or a stale graph is among them (規約 §ウィンドウの縁 / §状態).
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
    /// not the box (規約 §ウィンドウの縁「畳んだ `…` の箱」). The cell runs the band's depth; a frame that deep lies
    /// along the band's top and bottom edges and reads as a box glued to the window.
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

    /// Whether anything is asking for the card: the pointer on the mark, the pointer inside the card, or the hook
    /// standing in for either.
    readonly property alias stateLit: stateKeep.lit

    /// Whether the band has ever placed this group; the card waits for it. A cell the row has not laid out yet is
    /// still at (0, 0), where the offscreen pointer sits (rules-refs/app-ui.md「初めて立つセルはまだ原点に居る」).
    /// Kept once said: the row leaves a cell where it put it. Never make this group the row's first cell — the card
    /// would never open (`badges-hover`).
    property bool placed: false
    onXChanged: stateGroup.placed = true

    signal identityEditRequested()

    /// Automation: `BandStateCard.wordsOffLevel`, for the band to hand on.
    function stateCardWordsOff() { return stateCard.wordsOffLevel() }
    /// Opens the card; `stateKeep` is what closes it a beat after the last thing asking for it lets go.
    function settleStateCard() {
        if (!stateGroup.stateLit || !stateGroup.placed) {
            stateKeep.settle()
            return
        }
        if (!stateCard.opened && stateGroup.stateShown)
            stateCard.open()
    }

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

    /// The stand-in pointer is down on a group that can answer it: standing, and placed. `stateLit` alone is one edge
    /// too few for a pointer that cannot move — written before `placed`, it would never ask again. A real hand is not
    /// watched here: it gets no card until it next moves (`placed`).
    readonly property bool standInAsking: stateGroup.pointedAt && stateGroup.placed && stateGroup.stateShown

    onStateLitChanged: stateGroup.settleStateCard()
    onStandInAskingChanged: stateGroup.settleStateCard()
    onRoomChanged: stateGroup.settleCap()
    onTabRunAvailChanged: stateGroup.settleCap()

    visible: stateGroup.stateShown
    implicitHeight: stateGroup.foldedDepth
    // The row shares out by what each item asks for above its floor, so ask for the words — and folded, for the mark
    // alone, or the words' width stands empty beside it (デザイン規約 §ウィンドウの縁「タブと群は同時に譲る」).
    implicitWidth: stateGroup.folded ? stateGroup.foldedWidth : stateGroup.naturalWidth

    HoverCardHost {
        id: stateKeep
        card: stateCard
        pointedAt: groupHover.hovered || stateGroup.pointedAt
    }

    BandStateMetrics {
        id: badgeMetrics
        stateWorkingTree: stateGroup.stateWorkingTree
        hasAlso: stateGroup.stateHasAlso
        hasStep: stateGroup.stateHasStep
        minChars: stateGroup.stateMinChars
    }
    /// The share-out (`BandStateShare`). Its floor is bound to the metrics' property, which is settled by assignment
    /// and so notifies.
    BandStateShare {
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
                text: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opText : ""
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
                visible: stateGroup.stateWorkingTree !== null && stateGroup.stateWorkingTree.opAlso !== ""
                tint: Theme.warning
                Layout.alignment: Qt.AlignTop
                Layout.topMargin: badgeMetrics.capMiddle - opDot.implicitHeight / 2
            }
            Label {
                visible: stateGroup.stateWorkingTree !== null && stateGroup.stateWorkingTree.opAlso !== ""
                text: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opAlso : ""
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Theme.fontWeightStrong
            }
            Label {
                visible: stateGroup.stateWorkingTree !== null && stateGroup.stateWorkingTree.opSteps > 0
                text: stateGroup.stateWorkingTree === null ? ""
                      : qsTr("%1/%2").arg(stateGroup.stateWorkingTree.opStep).arg(stateGroup.stateWorkingTree.opSteps)
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
        // Outlined `danger`, and ahead of `OLD GIT` (規約 §ウィンドウの縁).
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
        // (規約 §ウィンドウの縁).
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

    // `…` typed — the same glyph and width on both OSes (規約 §ウィンドウの縁). Outside `Theme.buttonMinWidth`, which
    // is for a box round a word.
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
        // (規約 §当たり判定「広げるのは判定だけ」).
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
        // On the band's line, as the words it stands for (規約 §操作パネル「印は字と同じくベースラインに立てる」):
        // the frame is cut round the words' capitals from that line, and a centred line puts the dots wherever the
        // face's descent leaves them. The face's own report of the dots' ink is no help — Noto Sans JP's spans 7px.
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
            // A press opens what the hover opens: a press that did nothing would read as a dead control.
            onClicked: stateGroup.settleStateCard()
        }
    }

    BandStateCard {
        id: stateCard
        // Flush under the group's right edge: centred, it would open past the window; a gap would be a band the
        // pointer crosses touching neither card nor mark (規約 §hover のツールチップ).
        x: stateGroup.width - width
        y: stateGroup.height
        box: badgeMetrics
        opText: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opText : ""
        opAlso: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opAlso : ""
        opStep: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opStep : 0
        opSteps: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.opSteps : 0
        conflictCount: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.conflictCount : 0
        identityUnsaved: AppBackend.identityUnsaved
        gitVersion: AppBackend.gitVersion
        minimumGit: AppBackend.minimumGit
        opShown: stateGroup.opBadgeShown
        conflictShown: stateGroup.conflictBadgeShown
        identityShown: stateGroup.identityBadgeShown
        oldGitShown: stateGroup.oldGitBadgeShown
        staleShown: stateGroup.staleBadgeShown
        lfsShown: stateGroup.lfsBadgeShown
        lfsNeeded: stateGroup.stateWorkingTree !== null ? stateGroup.stateWorkingTree.lfsNeeded : 0
        staleStopped: stateGroup.curPage !== null && stateGroup.curPage.pageGraph.failed
        staleWhy: stateGroup.curPage !== null ? stateGroup.curPage.pageGraph.error : ""
        onIdentityRequested: stateGroup.identityEditRequested()
    }
}
