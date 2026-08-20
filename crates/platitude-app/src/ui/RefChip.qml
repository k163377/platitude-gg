import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// One aggregated chip: primary name + "+N". There are two things to read off it and they get one channel each — the
// frame carries the kind (local = accent, remote = secondary grey, detached HEAD = warning, which is a state rather
// than a kind, tag = refTag with a fill behind it), and the name carries where the ref is: ordinary text for one that
// is in this repository, grey for one that is only on the remote (デザイン規約 §ref の種別). The sidebar already reads that way
// — names in textPrimary, kind in the section icon — and on a graph row the name is the thing most worth reading, so it
// is not the place to spend a colour on something the frame is already saying. The kind is the first record's own, not
// something the caller sets: a row hands over everything on it in one list, branches ahead of tags, and the chip shows
// the head of that list. The one icon is the remote/PR badge, same mark and same single slot as the sidebar rows — for
// tags too, which is how "this one is only here" reads.
Rectangle {
    id: chip
    property var records: []
    property real maxWidth: Metrics.labelColW
    // Nowhere to go from here (the branch already under the working tree): §無効 — the words drop to the muted colour,
    // frame included, since the frame is how a branch chip is read at all.
    property bool muted: false

    visible: records.length > 0
    // As tall as the commit it names — the node itself, not a size that happens to equal it today, so the two cannot
    // drift apart (2026-08-20 ユーザー判断). A row then carries one band instead of two heights. It is also exactly one
    // `fontMdLine`, which is what lets the name inside be the row's own text size: a branch name is what the row is
    // read for, not meta about it (規約 §タイポグラフィ: `fontMd` = 一覧 / グラフ行の本文).
    height: Metrics.nodeIcon
    width: Math.min(chipContent.implicitWidth + 2 * Theme.spaceXs, maxWidth)
    radius: Theme.radiusSm
    clip: true

    readonly property string rec: records.length > 0 ? records[0] : "L0001"
    readonly property string recKind: rec[0]
    readonly property bool recHead: rec[1] === "1"
    readonly property bool recRemote: rec.length > 2 && rec[2] === "1"
    readonly property bool recPr: rec.length > 3 && rec[3] === "1"
    readonly property bool recHere: rec.length > 4 && rec[4] === "1"
    readonly property bool tagStyle: recKind === "T"
    // Name, and the remotes it was read from when it was not read here. The separator is absent whenever there are
    // none, so the name runs to the end of the record (see encode.rs).
    readonly property var recFields: rec.substring(5).split("\u001E")
    readonly property string recName: chip.recFields[0]
    readonly property string recWhere: chip.recFields.length > 1 ? chip.recFields[1] : ""
    // One slot, one mark: on the remote, or on the remote with a PR open (規約 §グラフ行のダブルクリック — the two never stack).
    readonly property bool hasBadge: recRemote || recPr
    // A tag this repository does not hold keeps the tag hue and only drops a step (§暗く落とした段): still a tag, read
    // somewhere else. Only tags dim, because only tags need it — every other kind says where it is in its own frame
    // colour (a remote branch is grey) or in its name (`origin/main` carries the remote in the name itself).
    readonly property color kindColor: muted ? Theme.textMuted
                                       : tagStyle ? (recHere ? Theme.refTag : Theme.refTagDim)
                                       : recKind === "R" ? Theme.textSecondary
                                       : recKind === "H" ? Theme.warning
                                       : Theme.accent
    // Where it is, not what it is. Grey is the name of something this repository does not hold — a remote branch, or a
    // tag only a remote has. Dropping further, to textMuted, would claim it cannot be reached, and a double-click on a
    // remote branch row goes there (§無効 is for what is actually unavailable). The detached HEAD marker keeps its state
    // colour in the name too: it is the one chip whose colour is not a kind. The branch the working tree stands on is
    // the nearest answer this colour has — "here" — and the sidebar already writes it that way, so the chip does too.
    readonly property color nameColor: muted ? Theme.textMuted
                                       : recKind === "H" ? Theme.warning
                                       : recHead ? Theme.textLink
                                       : !recHere ? Theme.textSecondary
                                       : Theme.textPrimary

    color: tagStyle ? Theme.bgElevated : "transparent"
    border.color: kindColor
    border.width: Theme.borderWidth

    /// Where the ink starts inside the frame, in whole pixels off the top of the chip: the room the border leaves,
    /// halved, **with the odd pixel going up**.
    ///
    /// The frame is the commit's height rather than the type's, so what stands in it is placed by its ink and not by
    /// the line box the family hands out: a family keeps more room above its ascender than below its descender, and
    /// centring the box spent that room inside it and sat the descenders of `g` and `/` on the border (2026-08-20
    /// ユーザー報告). The amount is the family's own, so a fixed lift squares one and opens a gap under the other: this asks.
    /// Three of the eighteen pixels inside are left over (both families put fifteen of ink in it), and three will not
    /// halve; the pixel goes above, where every ascender is, rather than below the one descender a name may not even
    /// have (実測: 2/1 both on screen with Yu Gothic UI and in the Ubuntu container with Noto Sans CJK JP — and **not off
    /// the headless picture**, which is drawn in neither, see the verify-ui skill).
    function inkTop(ink) {
        const room = chip.height - 2 * Theme.borderWidth - ink.tightBoundingRect.height
        return Theme.borderWidth + Math.ceil(room / 2)
    }
    /// Where a label goes to put its ink there. Whole pixels: a label laid out on a half one spreads its antialiasing
    /// into a row it does not own, and that row is the margin. **Each label asks for itself** — the two here are
    /// different sizes, and one offset for the row hung the smaller from the taller one's top edge (2026-08-20 ユーザー報告).
    function inkY(label, ink) {
        return Math.round(chip.inkTop(ink) - label.baselineOffset - ink.tightBoundingRect.y)
    }

    // Measured off a probe reaching every extreme Latin ink has, rather than off the name itself, so a chip does not
    // stand differently from its neighbour because the name it carries happens to have no descender in it.
    TextMetrics {
        id: nameInk
        font: nameLabel.font
        text: "Hbxp"
    }
    TextMetrics {
        id: countInk
        font: countLabel.font
        text: "Hbxp"
    }

    Row {
        id: chipContent
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.leftMargin: Theme.spaceXs
        spacing: Theme.spaceXs
        Label {
            id: nameLabel
            y: chip.inkY(nameLabel, nameInk)
            text: chip.recName
            color: chip.nameColor
            font.pixelSize: Theme.fontMd
            font.weight: chip.recHead ? Font.DemiBold : Font.Normal
            elide: Text.ElideRight
            width: Math.min(implicitWidth,
                            chip.maxWidth - 2 * Theme.spaceXs - (chip.records.length > 1 ? Theme.spaceLg : 0)
                            - (chip.hasBadge ? Theme.iconSm + Theme.spaceXs : 0))
        }
        // How many more names the card has, which is meta about the row rather than one of the names — the colour the
        // row's other meta (author, date) is written in.
        Label {
            id: countLabel
            y: chip.inkY(countLabel, countInk)
            visible: chip.records.length > 1
            text: "+" + (chip.records.length - 1)
            color: chip.muted ? Theme.textMuted : Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        // Remote / PR badge: reserved width above, so it survives any elision.
        NavIcon {
            visible: chip.hasBadge
            anchors.verticalCenter: parent.verticalCenter
            kind: chip.recPr ? "pr" : "remote"
            tint: chip.muted ? Theme.textMuted : chip.recPr ? Theme.success : Theme.textSecondary
            width: Theme.iconSm
            height: Theme.iconSm
        }
    }
}
