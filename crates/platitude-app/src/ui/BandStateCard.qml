pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The band's state badges, opened out. The band itself keeps one mark for however many of them are standing (`TopBar` の
// `…`): the tabs are what that row is for, and a row of badge words beside them is the widest thing this app ever puts
// there (デザイン規約 §ウィンドウの縁).
//
// A card, for the reason `EolHoverCard` is one: a tooltip's ground is the Fusion default from
// outside this theme, and what stands on it here is a badge whose whole meaning is its colour.
//
// Each row is one of the band's badges, with the one thing that badge had no room for beside it (規約 §hover
// のツールチップ = ラベルに入らなかった 1 点だけを足す). The badges keep their own shapes — the frame and the fill are what says which of them
// is the one that stops work — so this is the picture the band stands for, only stacked.
//
// **The sentences can be taken away.** A `CardText` is what the right-hand half of every row is,
// because the git version and the count of waiting files are things a reader wants in their hands. A badge is a mark
// — its word is the band's own, it is drawn to be recognised by colour and shape, and one
// of them is a button.
//
// Owned by the band: a popup parented to something that can be laid out away takes the card
// with it (app-ui.md).
AppCard {
    id: stateCard

    /// What the working tree is in the middle of, handed in: the card is a view, and the band is the
    /// one that knows which page is current (app-ui.md コンポーネント配線規約).
    property string opText: ""
    property string opAlso: ""
    property int opStep: 0
    property int opSteps: 0
    property int conflictCount: 0
    property bool identityUnsaved: false
    /// The two halves of the old-git row's sentence: what answered, and what this app is built for.
    property string gitVersion: ""
    property string minimumGit: ""
    /// Which rows stand. Handed in for the same reason, and read by the mark as well — the mark is out exactly when at
    /// least one of these is, so the two cannot disagree about whether there is anything here to open.
    property bool opShown: false
    property bool conflictShown: false
    property bool identityShown: false
    property bool oldGitShown: false
    property bool staleShown: false
    /// Which of the two states the badge is standing for: the walk stopped part-way, so rows are missing. False is the
    /// other one — every row is drawn and the rebuild that would have refreshed them did not land. **True wins where
    /// both are** (a stream that stopped, then a rebuild over it that did not land): missing rows is the heavier of
    /// the two, and it is the one that has words to show for itself.
    property bool staleStopped: false
    /// What was said about the walk that gave up, where anybody said anything (`GraphModel.error`).
    property string staleWhy: ""

    /// The one row that is also a way somewhere. The badge it replaced was pressable, and folding the band keeps
    /// the way back to the setup screen after "Not now" (規約 §identity).
    signal identityRequested()

    /// One width for the badge column, so the sentences beside them all start on the same line (the reset submenu's
    /// chip column, same reasoning). Only the rows that stand are measured — the column is as wide as what
    /// is drawn.
    readonly property real badgeRun: Math.max(stateCard.opShown ? opBadge.implicitWidth : 0,
                                               stateCard.conflictShown ? conflictBadge.implicitWidth : 0,
                                               stateCard.identityShown ? identityBadge.implicitWidth : 0,
                                               stateCard.staleShown ? staleBadge.implicitWidth : 0,
                                               stateCard.oldGitShown ? oldGitBadge.implicitWidth : 0)

    /// Automation: which rows are standing, in band order. Read off the rows, so a row
    /// that is asked for and left out is not counted here.
    ///
    /// **Standing precedes drawn**: the layout gives a row its place a frame later, and this says nothing about
    /// that. `laidOut` is what says the card holds these — a reader who wants the picture's own answer waits for it
    /// first (`badges-hover` の `rows=`).
    function rowsLaidOut() {
        let out = []
        if (opRow.visible)
            out.push("op")
        if (conflictRow.visible)
            out.push("conflicts")
        if (identityRow.visible)
            out.push("identity")
        if (staleRow.visible)
            out.push("stale")
        if (oldGitRow.visible)
            out.push("old-git")
        return out.join(",")
    }

    /// Automation: what the card came out to. `RefListPopup` settles its rows before opening
    /// (`Positioner.forceLayout()`), but a `ColumnLayout` has no such call — that one belongs to the positioners — so
    /// this card is measured after the fact instead (`badges-hover` の `card=`). What the measuring is for is the trap
    /// in app-ui.md: a card drawn at its padding and grown a frame later cannot see a hand that is already inside it.
    ///
    /// It holds here because of where this one opens: under the mark, with the hand still on the mark, and the grip is
    /// kept by `pointerInside` from either side of the gap.
    readonly property string laidOutSize: Math.round(width) + "x" + Math.round(height)

    /// Automation: whether the two readings above are of the card or of a frame it is passing through. **"After the
    /// fact" is a fact somebody has to wait for.** A `ColumnLayout` settles on polish, so a row that begins to stand
    /// is in `rowsLaidOut()` in that same frame and in the picture only in the next one; in between, the card is the
    /// one it was before, with every new row piled at the top of it (measured: all three rows standing at y=0 in a
    /// card 36 tall, reported as a three-row card). A report written there says a card nobody could photograph.
    ///
    /// What is read is the rows agreeing with the card: each one that stands sits below the one before it, and
    /// together they fill the content exactly. A row arriving breaks the first half, a row leaving the second.
    readonly property bool laidOut: stateCard.rowsSettled()
    function rowsSettled() {
        const all = [opRow, conflictRow, identityRow, staleRow, oldGitRow]
        let bottom = -1
        for (let i = 0; i < all.length; i++) {
            if (!all[i].visible)
                continue
            if (all[i].y < bottom)
                return false
            bottom = all[i].y + all[i].height
        }
        return bottom > 0 && Math.abs(rows.height - bottom) < 0.5
    }

    // Escape only: the pointer leaving is what closes this one (`BandStateGroup.settleStateCard`), and a
    // press outside is already on its way somewhere else.
    closePolicy: Popup.CloseOnEscape
    // The pointer leaving is what closes it, so the card has to see both halves of `AppCard.pointerInside` — the face
    // and the rows.
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start (規約 §hover のツールチップ). The badges keep their own
    // face: the pad lies under them (規約: バッジと印も対象外).
    textContent: rows

    contentItem: ColumnLayout {
        id: rows
        spacing: Theme.spaceXs
        HoverHandler {
            id: contentHover
        }

        // The stopped operation. Its own way out is a card in the working-tree pane, and
        // where that sits is the one thing the badge could not say (規約 §進行中の操作から出る).
        RowLayout {
            id: opRow
            visible: stateCard.opShown
            spacing: Theme.spaceSm
            Rectangle {
                id: opBadge
                color: "transparent"
                border.color: Theme.warning
                border.width: Theme.borderWidth
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: opLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                RowLayout {
                    id: opLabel
                    anchors.centerIn: parent
                    spacing: Theme.spaceXs
                    Label {
                        text: stateCard.opText
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                    // Bisect runs alongside, so it is the one thing that can share this badge. What
                    // goes between the two names is drawn — a typed middle dot would put a full-width cell inside
                    // the badge (規約 §余白).
                    DotMark {
                        visible: stateCard.opAlso !== ""
                        tint: Theme.warning
                        Layout.alignment: Qt.AlignVCenter
                    }
                    Label {
                        visible: stateCard.opAlso !== ""
                        text: stateCard.opAlso
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                    // The count is the half a stopped rebase cannot say without it; a merge steps through nothing and
                    // has none.
                    Label {
                        visible: stateCard.opSteps > 0
                        text: qsTr("%1/%2").arg(stateCard.opStep).arg(stateCard.opSteps)
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
            }
            CardText {
                text: qsTr("Ways out sit under the commit button")
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // The conflicts. How many is what the badge had no room for, and it is also the only number that says how much
        // is left to do.
        RowLayout {
            id: conflictRow
            visible: stateCard.conflictShown
            spacing: Theme.spaceSm
            Rectangle {
                id: conflictBadge
                color: Theme.danger
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: conflictLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                Label {
                    id: conflictLabel
                    anchors.centerIn: parent
                    text: Words.badgeConflicts
                    color: Theme.textOnAccent
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            CardText {
                text: qsTr("%n file(s) waiting on a decision", "", stateCard.conflictCount)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // Nothing to attribute commits to. The row is pressable: this is the one state of
        // the three the band can do something about on the spot.
        RowLayout {
            id: identityRow
            visible: stateCard.identityShown
            spacing: Theme.spaceSm
            Rectangle {
                id: identityBadge
                color: "transparent"
                border.color: Theme.warning
                border.width: Theme.borderWidth
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: identityLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSm
                    color: Theme.bgHover
                    visible: identityMouse.containsMouse
                }
                Label {
                    id: identityLabel
                    anchors.centerIn: parent
                    text: Words.badgeSetIdentity
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
                MouseArea {
                    id: identityMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: stateCard.identityRequested()
                }
            }
            // The word the badge stands for, and which of the two ways it came to be standing — a half-landed save
            // leaves an identity that *is* set, and nothing else on screen would say so (規約 §identity).
            CardText {
                text: stateCard.identityUnsaved
                      ? qsTr("Name and email were not both saved")
                      : qsTr("No name or email set for commits")
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // The graph and the repository disagree. **Which of the two ways it came to be so is the one thing the
        // badge could not say**, and it is what a reader needs to know here: rows are missing, or every row is there
        // and out of date. The two are the same position — nothing on screen may be acted on — so they share a badge
        // and part on this line.
        //
        // **Only the stopped walk quotes git.** A rebuild that failed said so where every other read does — the error
        // line and the log's mark — and a sentence said twice is two places to keep in step. The stopped walk has
        // nowhere else: its words come in on the stream and stop here.
        RowLayout {
            id: staleRow
            visible: stateCard.staleShown
            spacing: Theme.spaceSm
            Rectangle {
                id: staleBadge
                color: "transparent"
                border.color: Theme.danger
                border.width: Theme.borderWidth
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: staleLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                Label {
                    id: staleLabel
                    anchors.centerIn: parent
                    text: Words.badgeStaleGraph
                    color: Theme.danger
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            CardText {
                text: !stateCard.staleStopped
                      ? qsTr("The latest history is not being shown")
                      : stateCard.staleWhy !== ""
                        ? stateCard.staleWhy
                        : qsTr("Only part of the history could be read")
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // The git on this machine is older than the one this app is built for. Which two versions those are is what the
        // badge had no room for, and it is the whole of what can be done about it — the way out is installing a newer
        // git, which is not in this window.
        RowLayout {
            id: oldGitRow
            visible: stateCard.oldGitShown
            spacing: Theme.spaceSm
            Rectangle {
                id: oldGitBadge
                color: "transparent"
                border.color: Theme.warning
                border.width: Theme.borderWidth
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: oldGitLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                Label {
                    id: oldGitLabel
                    anchors.centerIn: parent
                    text: Words.badgeOldGit
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            CardText {
                text: qsTr("git %1 is older than the %2 this app is built for")
                          .arg(stateCard.gitVersion).arg(stateCard.minimumGit)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }
    }
}
