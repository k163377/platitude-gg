import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The state badges give way in the two steps the tab names do (規約 §ウィンドウの縁): whole words, then narrowed *together* to
// one width, then a single mark. The card the pointer opens carries the whole of it at every step, so nothing is ever
// only in the band.
Item {
    id: stateGroup

    /// The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether the window is standing on its floor. Handed in, because the floor is the larger of the band's and the
    /// page's and only `Main` has both. The group gives up its words there whatever else is true (2026-08-11 ユーザー指示).
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one, so the card can be photographed (`badges-hover` /
    /// `identity-tip`). The real hover writes this same one property — hover is the input that cannot be injected, so
    /// the card has to be answering a single question or the headless run proves nothing about it.
    property bool pointedAt: false
    /// What the tab strip made of its own run. The group folds off the strip's width as well as its own, and the two do
    /// not always change in the same frame.
    property real tabContentWidth: 0
    property real tabRunAvail: 0
    property int tabCount: 0
    /// The padding and the height of the buttons this group stands between — what sets how big a target is here is the
    /// padding a Fusion `ToolButton` keeps around its content, a number the theme does not have. Handed in rather than
    /// read off a neighbour by id, so the group can be laid out beside anything.
    property real controlPadding: 0
    property real controlHeight: 0

    /// The four things that can be the matter here, folded into one mark and opened as a card (`BandStateCard`). One
    /// expression each, read by the mark and by the card: written twice, the two could disagree about whether there is
    /// anything here to open.
    readonly property var stateWt: stateGroup.curPage !== null ? stateGroup.curPage.pageWt : null
    readonly property bool opBadgeShown: stateGroup.stateWt !== null && stateGroup.stateWt.opText !== ""
    readonly property bool conflictBadgeShown: stateGroup.stateWt !== null && stateGroup.stateWt.hasConflicts
    /// A save whose halves did not both land leaves an identity that *is* set and not the one that was asked for, so
    /// nothing else on screen would mention it.
    readonly property bool identityBadgeShown: AppBackend.identityState === "missing" || AppBackend.identityUnsaved
        || (stateGroup.curPage !== null && !stateGroup.curPage.pageTab.identityReady)
    readonly property bool oldGitBadgeShown: AppBackend.gitUnsupported
    readonly property bool stateShown: stateGroup.opBadgeShown || stateGroup.conflictBadgeShown
                                       || stateGroup.identityBadgeShown || stateGroup.oldGitBadgeShown
    /// The narrowest a badge is drawn before the group gives up on words. Counted in characters rather than pixels (規約
    /// §ウィンドウの縁) — the same count costs a different number of pixels in each platform's UI font. Two, where the tab
    /// names keep three (2026-08-11 ユーザー指示).
    readonly property int stateMinChars: 2
    readonly property real stateBadgeMinW: Math.ceil(stateFont.advanceWidth("…")
                                                       + stateGroup.stateMinChars * stateFont.averageCharacterWidth)
        + 2 * Theme.spaceXs
    readonly property bool stateHasAlso: stateGroup.stateWt !== null && stateGroup.stateWt.opAlso !== ""
    readonly property bool stateHasStep: stateGroup.stateWt !== null && stateGroup.stateWt.opSteps > 0
    /// Whole pixels, for the reason the tab names are settled in them (規約 §ウィンドウの縁): a word asks for a fractional
    /// width, a box is laid out on a whole one, and a ceiling summed from the fractions is a few pixels under what the
    /// same widths add up to when each is rounded — so the group is handed exactly its natural width and the share-out
    /// still finds itself short, and every word elides in a band with room to spare (measured 2026-08-11 on Linux:
    /// `cap=103` with `groupW=270`, which was the natural width).
    readonly property int opBadgeW: Math.ceil(mOpText.implicitWidth
                                               + (stateGroup.stateHasAlso
                                                  ? 2 * Theme.spaceXs + mDot.implicitWidth + mOpAlso.implicitWidth : 0)
                                               + (stateGroup.stateHasStep ? Theme.spaceXs + mOpStep.implicitWidth : 0))
        + 2 * Theme.spaceXs
    readonly property int conflictBadgeW: Math.ceil(mConflict.implicitWidth) + 2 * Theme.spaceXs
    readonly property int identityBadgeW: Math.ceil(mIdentity.implicitWidth) + 2 * Theme.spaceXs
    readonly property int oldGitBadgeW: Math.ceil(mOldGit.implicitWidth) + 2 * Theme.spaceXs

    /// Automation: which of the group's three shapes is on screen, what the badges were narrowed to, and what the card
    /// came back with. The conditions above are what asks for a state; these are what the band made of it
    /// (`PG_AUTO_ACT=badges` / `badges-hover`).
    readonly property bool stateWordsShown: badgeRow.visible
    readonly property bool stateMarkShown: stateToggle.visible
    /// What the mark was actually painted with — the heaviest state's colour (規約 §状態). Read off the group rather than
    /// recomputed, for the reason `commandsMarkColor` is: what is being checked is that the rule reached the paint, and
    /// a second copy of the rule cannot say so.
    readonly property color stateMarkColor: stateGroup.tint
    readonly property int stateCapW: stateGroup.cap === Number.MAX_VALUE ? -1 : Math.round(stateGroup.cap)
    readonly property int stateGroupW: Math.round(stateGroup.width)
    readonly property bool stateCardOpen: stateCard.opened
    readonly property string stateCardRows: stateCard.rowsLaidOut()
    readonly property string stateCardSize: stateCard.laidOutSize

    /// Which colour the mark takes once the words are gone. The conflict is the one of the three that stops work, so it
    /// wins whenever it is among them (規約 §状態).
    readonly property color tint: stateGroup.conflictBadgeShown ? Theme.danger : Theme.warning
    /// The group's ceiling; `foldedWidth` below is its floor, and the window's floor is costed at the mark
    /// (`TopBar.floorWidth`).
    readonly property real naturalWidth: (stateGroup.opBadgeShown ? stateGroup.opBadgeW + Theme.spaceXs : 0)
        + (stateGroup.conflictBadgeShown ? stateGroup.conflictBadgeW + Theme.spaceXs : 0)
        + (stateGroup.identityBadgeShown ? stateGroup.identityBadgeW + Theme.spaceXs : 0)
        + (stateGroup.oldGitBadgeShown ? stateGroup.oldGitBadgeW + Theme.spaceXs : 0)
        - Theme.spaceXs
    readonly property real foldedWidth: stateMark.implicitWidth + 2 * stateGroup.controlPadding
    /// What each badge's box is drawn at, once they have given way together. `Number.MAX_VALUE` is "nothing is
    /// narrowed". Settled by hand (`settleCap`), since it is read off a list of measurements.
    property real cap: Number.MAX_VALUE
    /// Whether the words have been given up altogether. Bound rather than assigned beside the cap: two of the three
    /// conditions can change without the group's own width moving, and an assignment made in `settleCap` would never be
    /// asked for again (2026-08-11 報告).
    readonly property bool tabsScrolling: stateGroup.tabContentWidth > stateGroup.tabRunAvail
    /// How many tabs the strip has room for as they are drawn now. Off their real width rather than their cap: a cap is
    /// a ceiling the names may be nowhere near.
    readonly property int tabsInView: {
        const each = stateGroup.tabCount > 0 ? stateGroup.tabContentWidth / stateGroup.tabCount : 0
        return each > 0 ? Math.floor(stateGroup.tabRunAvail / each) : 3
    }
    readonly property bool folded: stateGroup.cap < stateGroup.stateBadgeMinW
        || (stateGroup.tabsScrolling && stateGroup.tabsInView < 3) || stateGroup.windowAtFloor

    /// Whether anything is asking for the card: the pointer on the mark, the pointer inside the card, or the hook
    /// standing in for either.
    readonly property alias stateLit: stateKeep.lit

    signal identityEditRequested()

    /// Opens the card; `stateKeep` is what closes it a beat after the last thing asking for it lets go.
    function settleStateCard() {
        if (!stateGroup.stateLit) {
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
        if (stateGroup.oldGitBadgeShown)
            want.push(stateGroup.oldGitBadgeW)
        if (want.length === 0) {
            stateGroup.cap = Number.MAX_VALUE
            return
        }
        let left = Math.floor(stateGroup.width) - (want.length - 1) * Theme.spaceXs
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
        stateGroup.cap = cap
    }

    onStateLitChanged: stateGroup.settleStateCard()
    onWidthChanged: stateGroup.settleCap()
    onTabRunAvailChanged: stateGroup.settleCap()

    visible: stateGroup.stateShown
    implicitHeight: stateGroup.controlHeight
    // The row hands its leftover out in proportion to what each filling item asked for, so **what is asked for is what
    // decides who gives way** — asking for the mark got this group a sliver, asking with a stretch of 100 got it
    // everything (both measured 2026-08-11). Asking for the words and no more puts the strip and this group in
    // proportion.
    implicitWidth: stateGroup.naturalWidth

    HoverCardHost {
        id: stateKeep
        card: stateCard
        pointedAt: groupHover.hovered || stateGroup.pointedAt
    }
    FontMetrics {
        id: stateFont
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
    }

    /// The three badges at their natural width, measured off labels that are never drawn.
    ///
    /// The badges in the band cannot also be what the cap is measured from. A `RowLayout` that is not being laid out
    /// reports the width it had when it last was, and the row of badges goes away the moment the group folds — so a cap
    /// read from there makes the fold one that nothing comes back from (measured 2026-08-11: `cap=32` with a 1440-wide
    /// window, and no width would bring the words back).
    component BadgeWord: Label {
        visible: false
        font.pixelSize: Theme.fontSm
        font.weight: Font.DemiBold
    }
    BadgeWord {
        id: mOpText
        text: stateGroup.stateWt !== null ? stateGroup.stateWt.opText : ""
    }
    BadgeWord {
        id: mOpAlso
        text: stateGroup.stateWt !== null ? stateGroup.stateWt.opAlso : ""
    }
    BadgeWord {
        id: mOpStep
        text: stateGroup.stateWt === null ? ""
              : qsTr("%1/%2").arg(stateGroup.stateWt.opStep).arg(stateGroup.stateWt.opSteps)
    }
    BadgeWord {
        id: mConflict
        text: Words.badgeConflicts
    }
    BadgeWord {
        id: mIdentity
        text: Words.badgeSetIdentity
    }
    BadgeWord {
        id: mOldGit
        text: Words.badgeOldGit
    }
    DotMark {
        id: mDot
        visible: false
    }

    // A handler rather than an area: it is passive, so the identity badge under it still takes its own press
    // (app-ui.md).
    HoverHandler {
        id: groupHover
    }

    // Right-aligned: the group grows and shrinks against the tabs on its left, and what has to stay put is its edge
    // with `>_`.
    Row {
        id: badgeRow
        visible: !stateGroup.folded
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs

        StateBadge {
            id: opBadge
            visible: stateGroup.opBadgeShown
            naturalW: stateGroup.opBadgeW
            cap: stateGroup.cap
            Label {
                text: stateGroup.stateWt !== null ? stateGroup.stateWt.opText : ""
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                // The one part of this badge that gives: the count and the second operation mean nothing cut in half.
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
            }
            // Drawn, not typed — a middle dot would put a full-width cell in the badge (規約 §余白).
            DotMark {
                visible: stateGroup.stateWt !== null && stateGroup.stateWt.opAlso !== ""
                tint: Theme.warning
                Layout.alignment: Qt.AlignVCenter
            }
            Label {
                visible: stateGroup.stateWt !== null && stateGroup.stateWt.opAlso !== ""
                text: stateGroup.stateWt !== null ? stateGroup.stateWt.opAlso : ""
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
            Label {
                visible: stateGroup.stateWt !== null && stateGroup.stateWt.opSteps > 0
                text: stateGroup.stateWt === null ? ""
                      : qsTr("%1/%2").arg(stateGroup.stateWt.opStep).arg(stateGroup.stateWt.opSteps)
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
            }
        }
        StateBadge {
            id: conflictBadge
            visible: stateGroup.conflictBadgeShown
            naturalW: stateGroup.conflictBadgeW
            cap: stateGroup.cap
            filled: true
            Label {
                text: Words.badgeConflicts
                color: Theme.textOnAccent
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
            }
        }
        StateBadge {
            id: identityBadge
            visible: stateGroup.identityBadgeShown
            naturalW: stateGroup.identityBadgeW
            cap: stateGroup.cap
            pressable: true
            onPressed: stateGroup.identityEditRequested()
            Label {
                text: Words.badgeSetIdentity
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
            }
        }
        StateBadge {
            id: oldGitBadge
            visible: stateGroup.oldGitBadgeShown
            naturalW: stateGroup.oldGitBadgeW
            cap: stateGroup.cap
            Label {
                text: Words.badgeOldGit
                color: Theme.warning
                font.pixelSize: Theme.fontSm
                font.weight: Font.DemiBold
                elide: Text.ElideRight
                Layout.fillWidth: true
                Layout.maximumWidth: implicitWidth
            }
        }
    }

    // `…` typed rather than drawn: the ellipsis is what Qt spends on its own eliding and it measured the same on both
    // OSes (764c362). The same box the command log's mark takes (規約 §ウィンドウの縁). `Theme.buttonMinWidth` does not reach it
    // — that floor is for a box put round a *word*, and given it the mark came out 80 wide with 26px of air at either
    // end of three dots.
    Rectangle {
        id: stateToggle
        visible: stateGroup.folded
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        implicitWidth: stateGroup.foldedWidth
        height: stateGroup.controlHeight
        width: implicitWidth
        radius: Theme.radiusSm
        color: stateMouse.containsMouse ? Theme.bgHover : "transparent"
        border.width: Theme.borderWidth
        border.color: stateGroup.tint
        Accessible.role: Accessible.Button
        Accessible.name: qsTr("What needs attention here")
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
            // A press opens what the hover opens: a press that did nothing would read as a dead control.
            onClicked: stateGroup.settleStateCard()
        }
    }

    BandStateCard {
        id: stateCard
        // Under the group and flush with its right-hand edge: the group sits at the band's right-hand end, and a card
        // centred on it would open past the window.
        x: stateGroup.width - width
        y: stateGroup.height + Theme.spaceXs
        opText: stateGroup.stateWt !== null ? stateGroup.stateWt.opText : ""
        opAlso: stateGroup.stateWt !== null ? stateGroup.stateWt.opAlso : ""
        opStep: stateGroup.stateWt !== null ? stateGroup.stateWt.opStep : 0
        opSteps: stateGroup.stateWt !== null ? stateGroup.stateWt.opSteps : 0
        conflictCount: stateGroup.stateWt !== null ? stateGroup.stateWt.conflictCount : 0
        identityUnsaved: AppBackend.identityUnsaved
        gitVersion: AppBackend.gitVersion
        minimumGit: AppBackend.minimumGit
        opShown: stateGroup.opBadgeShown
        conflictShown: stateGroup.conflictBadgeShown
        identityShown: stateGroup.identityBadgeShown
        oldGitShown: stateGroup.oldGitBadgeShown
        onIdentityRequested: stateGroup.identityEditRequested()
    }
}
