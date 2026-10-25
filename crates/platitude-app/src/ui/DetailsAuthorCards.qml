pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

/// The cards the commit's author row opens: author and committer, the co-authors the message credits, and every parent
/// of a line with more than it has room for. They open in the pane's coordinates, so they do not scroll with the block;
/// the row's anchor is mapped on the way in.
///
/// A `QtObject`, not an `Item`: the pane is a `ColumnLayout`, where any item is a row (rules-refs/app-ui.md).
QtObject {
    id: cards

    /// The pane the cards open in and the anchor is mapped into.
    required property Item host
    /// The block holding the row: the anchor's coordinates, the records, and the hover that keeps the cards open.
    required property var block
    /// The commit whose two people the author card names.
    required property var details
    /// The pane's hand (`HandWatch`), which the seats read: another commit landing under a still hand opens nothing
    /// until the hand moves (`DetailsPane`), and a card dismissed under it stays down until the hand leaves its line
    /// and comes back (`HoverSeat.expands`).
    property HandWatch hand: null
    /// A right-click menu of the page's stands (`RepoPage.menuStanding`): the cards are behind it — what is out goes
    /// the moment it stands, and nothing opens while it does (デザイン規約 §メニュー「メニューが立っている間、その裏の
    /// hover は引っ込む」); an ask made meanwhile is kept for it going.
    property bool menuStanding: false
    readonly property string blocker: cards.menuStanding ? "menu" : ""

    /// Whether each card is on screen — the output side, since the input side would read true with the binding cut.
    readonly property bool matesCardOpen: mateCard.opened
    readonly property bool authorCardOpen: authorCard.opened
    /// Whether the pointer is on a card — the row asks, because the two together are one hover.
    readonly property bool matesPointerInside: mateCard.pointerInside
    readonly property bool authorPointerInside: authorCard.pointerInside
    readonly property bool parentCardOpen: parentCard.opened
    readonly property bool parentsPointerInside: parentCard.pointerInside

    /// A row of the parents' card was pressed: the pane goes there, as the parent line's own links do.
    signal parentPicked(string oidHex)

    /// The pointer came onto the count that opens every parent, or left it. Each card's seat (`HoverSeat`) answers:
    /// what unfolds opens at once; the hand inside the card holds it; a card the reader dismissed stays down until
    /// the hand leaves the count and comes back (the card stands over the count, so Qt hands the count its hover
    /// again the moment the card is gone). The target is the commit's (`targetOf`): another commit landing takes the
    /// card down (`follow`), and the count asks again for the new one's.
    function pointParents(line) {
        parentKeep.point(cards.targetOf("parents", { "line": line }), true)
    }
    function settleParents() {
        parentKeep.unpoint(parentKeep.keyOf(target => target.kind === "parents"))
    }
    /// A target of `kind` on the commit shown now, with `fields` (where it was asked from): alive while that commit
    /// is the one shown, so a rest run out on the one before opens nothing.
    function targetOf(kind, fields) {
        const sha = cards.details.shaHex
        return Object.assign({ "key": kind + ":" + sha, "kind": kind, "sha": sha,
                               "alive": () => cards.details.shaHex === sha }, fields)
    }
    /// Another commit landed (`DetailsPane`, after telling the hand): a card out, or asked for, on the one before
    /// goes — the owner's close, not a dismissal.
    function follow() {
        const sha = cards.details.shaHex
        for (const seat of [mateKeep, parentKeep, authorKeep])
            seat.forget(target => target.sha !== sha, "commit")
    }
    /// **One of the three is out at a time**, and the one opening is that one: the hand has gone from the line the
    /// others hang off to another, so they go now, not after their beat (デザイン規約 §hover のツールチップ「1 つの
    /// ポインタが開けるものは 1 つ」). Each keeps its own beat for the hand walking into itself.
    function giveWay(opening) {
        for (const seat of [mateKeep, parentKeep, authorKeep]) {
            if (seat !== opening && (seat.shown || seat.waiting))
                seat.takeDown("handover", false)
        }
    }
    /// Opens every parent on the seat a single parent's hash has — the line's right end (`line`, in the row's
    /// coordinates), which the line keeps while it stands as one parent's (`HashPlate.unrolled`; デザイン規約
    /// §右のペインの親).
    function openParentCard(line) {
        const row = cards.block.valueRow
        if (row.plate.parents.length < 2) {
            parentKeep.refuse()
            return
        }
        const at = row.mapToItem(cards.host, line.x + line.width, line.y)
        parentCard.parents = row.plate.parents
        parentCard.x = Math.round(at.x - parentCard.hashWidth - parentCard.hashInset)
        parentCard.y = Math.round(at.y + line.height / 2 - parentCard.firstMiddle)
        // The pane's floor is the ceiling; its rows scroll past it.
        parentCard.listRoom = cards.host.height - parentCard.y - Theme.spaceXs
        parentCard.open()
    }
    /// Where each card was raised (host coordinates) and how far either may stand. Set on open: `mapToItem` is a
    /// call, so a binding would not see the splitter move.
    property real mateAnchorX: 0
    property real authorAnchorX: 0
    property real leftStop: 0
    property real rightStop: 0

    function openMateCard(at) {
        const records = cards.block.coAuthorRecords
        if (records.length === 0) {
            mateKeep.refuse()
            return
        }
        const p = cards.block.valueRow.mapToItem(cards.host, at.x, at.y)
        mateCard.records = records
        cards.takeRoom()
        mateCard.maxWidth = cards.rightStop - cards.leftStop
        cards.mateAnchorX = p.x
        // Flush against the underline: in a gap the pointer touches neither, and the card closes.
        mateCard.y = p.y
        mateCard.open()
    }
    function openAuthorCard(at) {
        if (cards.details.authorName === "") {
            authorKeep.refuse()
            return
        }
        const p = cards.block.valueRow.mapToItem(cards.host, at.x, at.y)
        cards.takeRoom()
        authorCard.maxWidth = cards.rightStop - cards.leftStop
        cards.authorAnchorX = p.x
        authorCard.y = p.y
        authorCard.open()
    }
    /// How far a card may reach on either side — the window's room, not the pane's, which is too narrow for an
    /// ordinary forge address (規約 §hover のツールチップ).
    function takeRoom() {
        // `spaceXxl` short of the window on the far side, like every floating card (`AppMenu.roomForRows` /
        // `RefListPopup.chipRoom`), and the pane's own inset on the near side.
        cards.leftStop = Theme.spaceXxl - cards.host.mapToItem(null, 0, 0).x
        cards.rightStop = cards.host.width - 2 * Theme.spaceXs
    }
    /// Where a card of that width sits: at the anchor, backed out of the pane when the row cannot hold it — the seat
    /// gives way, not the address. Read by a binding: a popup's final width comes a frame after its rows
    /// (rules-refs/app-ui.md).
    function seatX(anchorX, cardWidth) {
        return Math.max(cards.leftStop, Math.min(anchorX, cards.rightStop - cardWidth))
    }
    /// The pointer came onto the credit line or the name, or left it (as `pointParents`).
    function pointMates(at) {
        mateKeep.point(cards.targetOf("mates", { "at": at }), true)
    }
    function settleMates() {
        mateKeep.unpoint(mateKeep.keyOf(target => target.kind === "mates"))
    }
    function pointAuthor(at) {
        authorKeep.point(cards.targetOf("author", { "at": at }), true)
    }
    function settleAuthor() {
        authorKeep.unpoint(authorKeep.keyOf(target => target.kind === "author"))
    }

    readonly property CoAuthorCard mateCard: CoAuthorCard {
        id: mateCard
        parent: cards.host
        x: cards.seatX(cards.mateAnchorX, mateCard.width)
    }
    readonly property HoverSeat mateKeep: HoverSeat {
        id: mateKeep
        hand: cards.hand
        card: mateCard
        // The three cards open the moment the hand is on their line: a hold lifts with the hand's move, a dismissal
        // with the hand leaving the line and coming back.
        expands: true
        blockedBy: cards.blocker
        onOpenAsked: target => {
            cards.giveWay(mateKeep)
            cards.openMateCard(target.at)
        }
    }

    readonly property ParentListCard parentCard: ParentListCard {
        id: parentCard
        parent: cards.host
        onPicked: oidHex => cards.parentPicked(oidHex)
    }
    readonly property HoverSeat parentKeep: HoverSeat {
        id: parentKeep
        hand: cards.hand
        card: parentCard
        expands: true
        blockedBy: cards.blocker
        onOpenAsked: target => {
            cards.giveWay(parentKeep)
            cards.openParentCard(target.line)
        }
    }

    readonly property AuthorCard authorCard: AuthorCard {
        id: authorCard
        parent: cards.host
        x: cards.seatX(cards.authorAnchorX, authorCard.width)
        authorName: cards.details.authorName
        authorEmail: cards.details.authorEmail
        authorFace: cards.details.avatar
        authorFaceUrl: cards.details.avatarUrl
        authoredAt: cards.details.authorTime
        committerName: cards.details.committerName
        committerEmail: cards.details.committerEmail
        committerFace: cards.details.committerAvatar
        committerFaceUrl: cards.details.committerAvatarUrl
        committedAt: cards.details.committerTime
        committerDiffers: cards.details.committerDiffers
        timeDiffers: cards.details.commitTimeDiffers
    }
    readonly property HoverSeat authorKeep: HoverSeat {
        id: authorKeep
        hand: cards.hand
        card: authorCard
        expands: true
        blockedBy: cards.blocker
        onOpenAsked: target => {
            cards.giveWay(authorKeep)
            cards.openAuthorCard(target.at)
        }
    }
}
