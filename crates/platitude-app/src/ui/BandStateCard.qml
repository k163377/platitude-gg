pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The band's state badges, opened out (デザイン規約 §ウィンドウの縁): one row per badge, in its own shape, beside
// it the one thing it had no room for. A card, not a tooltip: a tooltip holds one string in one colour, and these
// badges mean their colour. The sentences are `CardText`, to be taken away.
//
// Owned by the group, not a badge or the mark: a popup parented to something that can be laid out away goes with it.
AppCard {
    id: stateCard

    /// What the working tree is in the middle of, handed in by the group.
    property string opText: ""
    property string opAlso: ""
    property int opStep: 0
    property int opSteps: 0
    property int conflictCount: 0
    property bool identityUnsaved: false
    /// The two halves of the old-git row's sentence: what answered, and what this app is built for.
    property string gitVersion: ""
    property string minimumGit: ""
    /// Which rows stand — the same flags the mark reads, so the two cannot disagree.
    property bool opShown: false
    property bool conflictShown: false
    property bool identityShown: false
    property bool oldGitShown: false
    property bool staleShown: false
    property bool lfsShown: false
    /// How many pending files need Git LFS — the one thing the `NO LFS` badge has no room for.
    property int lfsNeeded: 0
    /// True: the walk stopped part-way and rows are missing; false: every row is drawn but out of date. True wins
    /// where both hold — it is the heavier, and the one with words to show.
    property bool staleStopped: false
    /// What was said about the walk that gave up, where anybody said anything (`GraphModel.error`).
    property string staleWhy: ""

    /// The identity row is pressable (デザイン規約 §ウィンドウの縁「押せる行は identity だけ」).
    signal identityRequested()

    /// One width for the badge column, so the sentences start on one line.
    readonly property real badgeRun: Math.max(stateCard.opShown ? opBadge.implicitWidth : 0,
                                               stateCard.conflictShown ? conflictBadge.implicitWidth : 0,
                                               stateCard.identityShown ? identityBadge.implicitWidth : 0,
                                               stateCard.staleShown ? staleBadge.implicitWidth : 0,
                                               stateCard.lfsShown ? lfsBadge.implicitWidth : 0,
                                               stateCard.oldGitShown ? oldGitBadge.implicitWidth : 0)

    /// Automation: which rows are standing, in band order, read off the rows themselves. Standing precedes drawn —
    /// wait for `laidOut` first (`badges-hover` の `rows=`).
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
        if (lfsRow.visible)
            out.push("lfs")
        if (oldGitRow.visible)
            out.push("old-git")
        return out.join(",")
    }

    /// Automation: what the card came out to (`badges-hover` の `card=`). A `ColumnLayout` has no `forceLayout()`, so
    /// this is measured after the fact, not before opening
    /// (rules-refs/app-ui.md「hover で開くものは出す前に採寸する」) — which holds here because the card opens under the
    /// mark with the hand still on it.
    readonly property string laidOutSize: Math.round(width) + "x" + Math.round(height)

    /// Automation: whether the readings above are of the settled card. A `ColumnLayout` settles on polish, so a row
    /// that begins to stand is counted a frame before it is placed (piled at the top). Settled: each standing row
    /// sits below the one before, and together they fill the content exactly.
    readonly property bool laidOut: stateCard.rowsSettled()
    function rowsSettled() {
        const all = [opRow, conflictRow, identityRow, staleRow, lfsRow, oldGitRow]
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

    // Escape only: the pointer leaving is what closes it (`BandStateGroup.settleStateCard`).
    closePolicy: Popup.CloseOnEscape
    // Both halves of `AppCard.pointerInside`, for the same reason.
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start (規約 §hover のツールチップ); the badges are not text
    // (規約 §右のペインの字は掴める「バッジと印も対象外」).
    textContent: rows

    contentItem: ColumnLayout {
        id: rows
        spacing: Theme.spaceXs
        HoverHandler {
            id: contentHover
        }

        // The stopped operation, and where its way out sits (規約 §進行中の操作から出る).
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
                    // Bisect runs alongside and shares the badge. The dot is drawn — a typed one would put a
                    // full-width cell inside the badge (規約 §余白).
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

        // The conflicts, and how many.
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
                // A sentence for each count — `file(s)` would reach the reader as written (デザイン規約 §タイポグラフィ).
                text: stateCard.conflictCount === 1
                      ? qsTr("%n file waiting on a decision", "", stateCard.conflictCount)
                      : qsTr("%n files waiting on a decision", "", stateCard.conflictCount)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // No identity to commit as — the one pressable row.
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
            // Which of the two reasons it stands for — a half-landed save leaves an identity that *is* set
            // (デザイン規約 §ウィンドウの縁).
            CardText {
                text: stateCard.identityUnsaved
                      ? qsTr("Name and email were not both saved")
                      : qsTr("No name or email set for commits")
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // The graph and the repository disagree; this line says which way, and only the stopped walk quotes git
        // (デザイン規約 §ウィンドウの縁「カードの行が 2 つの状態を言い分ける」).
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

        // Pending files need Git LFS and git cannot run it: how many.
        RowLayout {
            id: lfsRow
            visible: stateCard.lfsShown
            spacing: Theme.spaceSm
            Rectangle {
                id: lfsBadge
                color: "transparent"
                border.color: Theme.warning
                border.width: Theme.borderWidth
                radius: Theme.radiusSm
                implicitHeight: Theme.iconLg
                implicitWidth: lfsLabel.implicitWidth + 2 * Theme.spaceXs
                Layout.preferredWidth: stateCard.badgeRun
                Label {
                    id: lfsLabel
                    anchors.centerIn: parent
                    text: Words.badgeNoLfs
                    color: Theme.warning
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                }
            }
            CardText {
                // A sentence for each count, as the conflicts row's; "uncommitted" is the files pane's own word for
                // the list they are counted from. That LFS is missing is the badge's to say, not this line's.
                text: stateCard.lfsNeeded === 1
                      ? qsTr("%n uncommitted file needs Git LFS", "", stateCard.lfsNeeded)
                      : qsTr("%n uncommitted files need Git LFS", "", stateCard.lfsNeeded)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
        }

        // The git on this machine is older than this app is built for: the two versions.
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
