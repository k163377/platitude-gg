pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

/// The two cards the commit's author row opens: author and committer, and the co-authors the message credits. They
/// open in the pane's coordinates, so they do not scroll with the block; the row's anchor is mapped on the way in.
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

    /// Whether each card is on screen — the output side, since the input side would read true with the binding cut.
    readonly property bool matesCardOpen: mateCard.opened
    readonly property bool authorCardOpen: authorCard.opened
    /// Whether the pointer is on a card — the row asks, because the two together are one hover.
    readonly property bool matesPointerInside: mateCard.pointerInside
    readonly property bool authorPointerInside: authorCard.pointerInside

    /// Where each card was raised (host coordinates) and how far either may stand. Set on open: `mapToItem` is a
    /// call, so a binding would not see the splitter move.
    property real mateAnchorX: 0
    property real authorAnchorX: 0
    property real leftStop: 0
    property real rightStop: 0

    function openMateCard(at) {
        const records = cards.block.coAuthorRecords
        if (records.length === 0)
            return
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
        if (cards.details.authorName === "")
            return
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
    /// The pointer left the row, or the card: each host closes its own a beat later.
    function settleMates() {
        mateKeep.settle()
    }
    function settleAuthor() {
        authorKeep.settle()
    }

    readonly property CoAuthorCard mateCard: CoAuthorCard {
        id: mateCard
        parent: cards.host
        x: cards.seatX(cards.mateAnchorX, mateCard.width)
    }
    readonly property HoverCardHost mateKeep: HoverCardHost {
        id: mateKeep
        card: mateCard
        pointedAt: cards.block.matesPointed
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
    readonly property HoverCardHost authorKeep: HoverCardHost {
        id: authorKeep
        card: authorCard
        pointedAt: cards.block.authorPointed
    }
}
