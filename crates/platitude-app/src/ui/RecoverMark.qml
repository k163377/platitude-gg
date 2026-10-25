import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// The discard log's mark for an entry or a part (`RecoverEntries`): what kind of thing it took, in its section's
// tint, and where (P3-確認事項 §破棄記録と復元):
// - here alone: the kind's mark
// - a remote's alone: for a tag, the tag a step down with the cloud on its shoulder (`ShoulderBadge`) — a remote's
//   branch is the cloud itself, REMOTES' mark
// - here and on a remote at once: the kind over the cloud across a slash — two things taken, two marks — a fraction
//   like `ActionButtonSeat`'s in `iconXs` halves, wider than one mark by `pairSpread`
Item {
    id: mark

    /// `NavIcon`'s kind, and its tint.
    property string kind: ""
    property color tint: Theme.textSecondary
    /// The cloud on the shoulder: what was taken was a remote's.
    property bool badged: false
    /// The kind over the cloud: taken here and on a remote at once.
    property bool paired: false
    /// The pixel size of the words the mark heads: a lone mark is drawn to their letters, the box's middle their
    /// capitals' middle (デザイン規約 §アイコンの席 の文頭記号) — the caller stands the box's middle there.
    property real size: Theme.fontMd
    /// How much wider the pair stands than one mark: what a row moves its name over by.
    readonly property int pairSpread: Theme.iconSm - Theme.spaceXs

    implicitWidth: mark.paired ? Theme.iconXs + mark.pairSpread : Theme.iconSm
    implicitHeight: mark.paired ? Theme.iconSm + Theme.spaceXs : Theme.iconSm

    // A step taller than the box, on the box's middle: the branch and the tree run past the capitals both ways and
    // come out as tall as the box, and a canvas only the box's height cuts what lands past it.
    NavIcon {
        visible: !mark.paired
        width: Theme.iconSm
        height: Theme.iconSm + Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        set: "lead"
        size: mark.size
        wordBase: (Theme.iconSm + Theme.spaceXs) / 2 + WordFace.capitalMiddle(mark.size)
        kind: mark.kind
        tint: mark.tint
        ShoulderBadge {
            visible: mark.badged
            kind: "remote"
            tint: Theme.textSecondary
            // On the box's shoulder, not the taller canvas's.
            anchors.topMargin: -Theme.spaceXs / 2
        }
    }
    // The halves a step under the mark they stand for, at opposite corners of the pair's box across the slash.
    Item {
        visible: mark.paired
        anchors.fill: parent
        NavIcon {
            x: 0
            y: 0
            width: Theme.iconXs
            height: Theme.iconXs
            kind: mark.kind
            tint: mark.tint
        }
        Label {
            anchors.centerIn: parent
            text: "/"
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        NavIcon {
            x: parent.width - width
            y: parent.height - height
            width: Theme.iconXs
            height: Theme.iconXs
            kind: "remote"
            tint: Theme.textSecondary
        }
    }
}
