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

    /// Where each card was raised, in the host's coordinates, and how far either may stand to the sides. Set on open
    /// rather than bound — the answer only matters at the moment it is asked, and `mapToItem` is a call, so a
    /// binding on one would not see the splitter move.
    property real mateAnchorX: 0
    property real authorAnchorX: 0
    property real leftStop: 0
    property real rightStop: 0

    /// The row raised an anchor in its own coordinates: map it here and open.
    function openMateCard(at) {
        const records = cards.block.coAuthorRecords
        if (records.length === 0)
            return
        const p = cards.block.valueRow.mapToItem(cards.host, at.x, at.y)
        mateCard.records = records
        cards.takeRoom()
        mateCard.maxWidth = cards.rightStop - cards.leftStop
        cards.mateAnchorX = p.x
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
        cards.takeRoom()
        authorCard.maxWidth = cards.rightStop - cards.leftStop
        cards.authorAnchorX = p.x
        authorCard.y = p.y
        authorCard.open()
    }
    /// How far a card may reach on either side.
    ///
    /// **The room is the window's, not the pane's.** These cards float over the whole window the way a menu and the
    /// chip's list do, and the pane they hang off is the narrowest column in it: measured from the name's shoulder
    /// to the pane's own edge, an ordinary forge address does not fit on one line and the card wraps it with the
    /// graph lying empty beside it. What is over the window's room still wraps — that part is the card's own rule
    /// (規約 §hover のツールチップ).
    function takeRoom() {
        // `spaceXxl` short of the window on the far side, the stop every floating card in the app keeps
        // (`AppMenu.roomForRows` / `RefListPopup.chipRoom`), and the pane's own inset on the near side, where these
        // cards have been standing all along.
        cards.leftStop = Theme.spaceXxl - cards.host.mapToItem(null, 0, 0).x
        cards.rightStop = cards.host.width - 2 * Theme.spaceXs
    }
    /// Where a card of that width sits: at the shoulder of the stretch that raised it, and backed up out of the pane
    /// when what is left of the row cannot hold it — **the seat is what gives way, not the address**, which is the
    /// thing the card was opened to show. A binding rather than a seat assigned on open: a popup does not have its
    /// final width in the frame it is handed its rows (app-ui.md).
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
