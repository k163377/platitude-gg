pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

/// The two cards the commit's author row opens: the whole of who authored and who committed, and the people the
/// message credits.
///
/// **They stand in the pane's coordinates, not the row's.** `CommitAuthorRow` owns the hover state and raises an
/// anchor point; the cards open here so they do not scroll away with the block the row sits on. The anchor arrives in
/// the row's coordinates and is mapped on the way in.
///
/// A `QtObject` holding two popups and their keepers by name, **not an `Item`**: the pane is a `ColumnLayout`, and an
/// item declared in one is a row of it however small it is made (measured — an anchored host there draws "anchors on
/// an item that is managed by a layout" and the pane lays out around it). A popup is not a layout child, so the two
/// stand where they are told to; each names the pane as its `parent` so that "where" is the pane's coordinates.
QtObject {
    id: cards

    /// The pane the cards open in: what the anchor is mapped into, and what is left for a card's rows is measured
    /// against.
    required property Item host
    /// The block holding the row, for the anchor's own coordinates, the records to show, and the hover the cards are
    /// kept open by.
    required property var block
    /// The commit whose two people the author card names.
    required property var details

    /// Whether each card is on screen — the output side, since the input side would read true with the binding cut.
    readonly property bool matesCardOpen: mateCard.opened
    readonly property bool authorCardOpen: authorCard.opened
    /// Whether the pointer is on a card rather than on the row that opened it — the row asks, because the two
    /// together are one hover.
    readonly property bool matesPointerInside: mateCard.pointerInside
    readonly property bool authorPointerInside: authorCard.pointerInside

    /// The row raised an anchor in its own coordinates: map it here and open. Set on open rather than bound — the
    /// answer only matters at the moment it is asked.
    function openMateCard(at) {
        const records = cards.block.coAuthorRecords
        if (records.length === 0)
            return
        const p = cards.block.valueRow.mapToItem(cards.host, at.x, at.y)
        mateCard.records = records
        // Measured from where it opens, not from the pane: the card starts
        // partway across, so the pane's width is not what is left for it.
        mateCard.maxRowWidth = cards.host.width - p.x - 2 * Theme.spaceXs
        mateCard.x = p.x
        // Flush against the underline: a gap is a band the pointer crosses
        // while touching neither, and the card closes under it. Same rule
        // the ref list follows.
        mateCard.y = p.y
        mateCard.open()
    }
    function openAuthorCard(at) {
        if (cards.details.authorName === "")
            return
        const p = cards.block.valueRow.mapToItem(cards.host, at.x, at.y)
        authorCard.maxRowWidth = cards.host.width - p.x - 2 * Theme.spaceXs
        authorCard.x = p.x
        authorCard.y = p.y
        authorCard.open()
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
    }
    readonly property HoverCardHost mateKeep: HoverCardHost {
        id: mateKeep
        card: mateCard
        pointedAt: cards.block.matesPointed
    }

    readonly property AuthorCard authorCard: AuthorCard {
        id: authorCard
        parent: cards.host
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
