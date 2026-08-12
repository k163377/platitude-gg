pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The band's state badges, opened out. The band itself keeps one mark for
// however many of them are standing (`TopBar` の `…`): the tabs are what
// that row is for, and four badges of words beside them is the widest
// thing this app ever puts there (デザイン規約 §ウィンドウの縁).
//
// A card rather than a `ToolTip`, for the reason `EolHoverCard` is one: a
// tooltip's ground is the Fusion default from outside this theme, and what
// stands on it here is a badge whose whole meaning is its colour.
//
// Each row is the badge the band used to draw, with the one thing that
// badge had no room for beside it (規約 §hover のツールチップ = ラベルに
// 入らなかった 1 点だけを足す). The badges keep their own shapes — the
// frame and the fill are what says which of them is the one that stops
// work — so this is the same picture the band drew, only stacked.
//
// Owned by the band rather than by the mark: a popup parented to something
// that can be laid out away takes the card with it (app-ui.md).
Popup {
    id: stateCard

    /// What the working tree is in the middle of, handed in rather than
    /// read: the card is a view, and the band is the one that knows which
    /// page is current (app-ui.md コンポーネント配線規約).
    property string opText: ""
    property string opAlso: ""
    property int opStep: 0
    property int opSteps: 0
    property int conflictCount: 0
    property bool identityUnsaved: false
    /// The two halves of the old-git row's sentence: what answered, and
    /// what this app is built for.
    property string gitVersion: ""
    property string minimumGit: ""
    /// Which rows stand. Handed in for the same reason, and read by the
    /// mark as well — the mark is out exactly when at least one of these
    /// is, so the two cannot disagree about whether there is anything
    /// here to open.
    property bool opShown: false
    property bool conflictShown: false
    property bool identityShown: false
    property bool oldGitShown: false

    /// The one row that is also a way somewhere. The badge it replaced was
    /// pressable, and folding the band must not cost the way back to the
    /// setup screen after "Not now" (規約 §identity).
    signal identityRequested()

    /// Whether the pointer is anywhere on this card. Two handlers OR'd,
    /// because `background` and `contentItem` are siblings rather than
    /// parent and child — a card carrying only one of them goes blind the
    /// moment the hand reaches the rows (app-ui.md の 5 つの罠 (2)).
    readonly property bool pointerInside: insideHover.hovered
                                          || contentHover.hovered

    /// One width for the badge column, so the sentences beside them all
    /// start on the same line (the reset submenu's chip column, same
    /// reasoning). Only the rows that stand are measured — a badge that
    /// is not drawn should not push the column out.
    readonly property real badgeRun:
        Math.max(stateCard.opShown ? opBadge.implicitWidth : 0,
                 stateCard.conflictShown ? conflictBadge.implicitWidth : 0,
                 stateCard.identityShown ? identityBadge.implicitWidth : 0,
                 stateCard.oldGitShown ? oldGitBadge.implicitWidth : 0)

    /// Automation: which rows the card actually laid out, in band order.
    /// Read off the rows themselves rather than off the flags above —
    /// asking for a row and getting one are different things, and only
    /// one of them is what the picture holds.
    function rowsLaidOut() {
        let out = []
        if (opRow.visible)
            out.push("op")
        if (conflictRow.visible)
            out.push("conflicts")
        if (identityRow.visible)
            out.push("identity")
        if (oldGitRow.visible)
            out.push("old-git")
        return out.join(",")
    }

    /// Automation: what the card came out to. `RefListPopup` settles its
    /// rows before opening (`Positioner.forceLayout()`), but a
    /// `ColumnLayout` has no such call — that one belongs to the
    /// positioners — so this card is measured after the fact instead
    /// (`badges-hover` の `card=`). What the measuring is for is the trap
    /// in app-ui.md: a card drawn at its padding and grown a frame later
    /// cannot see a hand that is already inside it.
    ///
    /// It holds here because of where this one opens: under the mark,
    /// with the hand still on the mark, and the grip is kept by
    /// `pointerInside` from either side of the gap.
    readonly property string laidOutSize:
        Math.round(width) + "x" + Math.round(height)

    padding: Theme.spaceSm
    // No `CloseOnPressOutside`: the pointer leaving is what closes this
    // one (`TopBar.settleStateCard`), and a press outside is already on
    // its way somewhere else.
    closePolicy: Popup.CloseOnEscape
    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
        HoverHandler {
            id: insideHover
        }
    }

    contentItem: ColumnLayout {
        id: rows
        spacing: Theme.spaceXs
        HoverHandler {
            id: contentHover
        }

        // The stopped operation. Its own way out is a card in the
        // working-tree pane rather than anything here, and where that
        // sits is the one thing the badge could not say
        // (規約 §進行中の操作から出る).
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
                    // Bisect runs alongside rather than instead, so it is
                    // the one thing that can share this badge. What goes
                    // between the two names is drawn, not typed — a middle
                    // dot would put a full-width cell inside the badge
                    // (規約 §余白).
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
                    // The count is the half a stopped rebase cannot say
                    // without it; a merge steps through nothing and has
                    // none.
                    Label {
                        visible: stateCard.opSteps > 0
                        text: qsTr("%1/%2").arg(stateCard.opStep)
                                           .arg(stateCard.opSteps)
                        color: Theme.warning
                        font.pixelSize: Theme.fontSm
                        font.weight: Font.DemiBold
                    }
                }
            }
            Label {
                text: qsTr("Ways out sit under the commit button")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
        }

        // The conflicts. How many is what the badge had no room for, and
        // it is also the only number that says how much is left to do.
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
                    text: qsTr("CONFLICTS")
                    color: Theme.textOnAccent
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            Label {
                text: qsTr("%n file(s) waiting on a decision", "",
                           stateCard.conflictCount)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
        }

        // Nothing to attribute commits to. The row is pressable where the
        // other two are not: this is the one state of the three the band
        // can do something about on the spot.
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
                    text: qsTr("SET IDENTITY")
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
            // The word the badge stands for, and which of the two ways it
            // came to be standing — a half-landed save leaves an identity
            // that *is* set, and nothing else on screen would say so
            // (規約 §identity).
            Label {
                text: stateCard.identityUnsaved
                      ? qsTr("Name and email were not both saved")
                      : qsTr("No name or email set for commits")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
        }

        // The git on this machine is older than the one this app is built
        // for. Which two versions those are is what the badge had no room
        // for, and it is the whole of what can be done about it — the way
        // out is installing a newer git, which is not in this window.
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
                    text: qsTr("OLD GIT")
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            Label {
                text: qsTr("git %1 is older than the %2 this app is built for")
                          .arg(stateCard.gitVersion).arg(stateCard.minimumGit)
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
        }
    }
}
