import QtQuick
import QtTest
import platitude.ui

// The details pane's author row and the cards it opens (`CommitAuthorRow` → `DetailsAuthorCards`), under a real
// pointer and the pane's hand (`HandWatch`, as `DetailsPane` keeps it): the parents' count unfolds every parent at
// once and the hand inside the card holds it; a card the reader dismissed is not put back under the same still hand;
// another commit landing takes the card down and holds the hand, so nothing of the new commit's opens until the hand
// has walked (デザイン規約 §右のペインの親, §hover のツールチップ「手が動いていない所へ来た物は開かない」).
Item {
    id: root
    width: 520
    height: 400

    /// The commit's details as the pane hands them over (`DetailsModel`), the two people both the same here.
    readonly property QtObject commit: QtObject {
        id: commit
        property string authorName: "Someone"
        property string authorEmail: "someone@example.com"
        property int authorTime: 1790751420
        property int avatar: 3
        property string avatarUrl: ""
        property string committerName: "Someone"
        property string committerEmail: "someone@example.com"
        property int committerAvatar: 3
        property string committerAvatarUrl: ""
        property int committerTime: 1790751420
        property bool committerDiffers: false
        property bool commitTimeDiffers: false
        property string sha8: "3c1b3390"
        property string shaHex: "3c1b3390aaaabbbbccccddddeeeeffff00001111"
        property var coAuthors: []
        property var parentHexes: []
        property string parentHex: parentHexes.length > 0 ? parentHexes[0] : ""
        /// The pane is told when another commit's details land (`DetailsModel.changed`).
        signal changed()
        function land(sha, parents) {
            commit.shaHex = sha
            commit.sha8 = sha.substring(0, 8)
            commit.parentHexes = parents
            commit.changed()
        }
    }
    function parentsOf(prefix, n) {
        const out = []
        for (let i = 0; i < n; i++)
            out.push(prefix + "a1b2c3d4aaaabbbbccccddddeeeeffff000044" + (10 + i))
        return out
    }

    // The pane's hand, and what the pane does when another commit lands: the hand first, then the cards
    // (`DetailsPane`).
    HandWatch {
        id: hand
    }
    Connections {
        target: commit
        function onChanged() {
            hand.rowsMoved()
            cards.follow()
        }
    }

    CommitAuthorRow {
        id: row
        width: root.width
        details: commit
        paneWidth: row.width
        mateCardInside: cards.matesPointerInside
        authorCardInside: cards.authorPointerInside
        parentsUnrolled: cards.parentCardOpen
        onOpenMateRequested: at => cards.pointMates(at)
        onSettleMateRequested: cards.settleMates()
        onOpenParentsRequested: line => cards.pointParents(line)
        onSettleParentsRequested: cards.settleParents()
        onOpenAuthorRequested: at => cards.pointAuthor(at)
        onSettleAuthorRequested: cards.settleAuthor()
    }
    /// The block holding the row, cut down to what the cards read off it.
    readonly property QtObject block: QtObject {
        id: block
        readonly property var valueRow: row
        readonly property var coAuthorRecords: row.coAuthorRecords
    }
    readonly property DetailsAuthorCards cards: DetailsAuthorCards {
        id: cards
        host: root
        block: block
        details: commit
        hand: hand
        // A parent pressed in the card: the page goes there, and that commit's details land (`DetailsPane`).
        onParentPicked: oidHex => commit.land(oidHex, root.parentsOf("9", 24))
    }

    TestCase {
        name: "AuthorCards"
        when: windowShown

        function init() {
            mouseMove(root, root.width - 1, root.height - 1)
            cards.parentKeep.takeDown("init", false)
            cards.mateKeep.takeDown("init", false)
            cards.authorKeep.takeDown("init", false)
            cards.parentKeep.latchedKey = ""
            cards.mateKeep.latchedKey = ""
            cards.authorKeep.latchedKey = ""
            commit.coAuthors = []
            commit.land("3c1b3390aaaabbbbccccddddeeeeffff00001111", root.parentsOf("1", 24))
            tryVerify(() => row.plate.moreStands, undefined, "the count stands: more parents than the line holds")
            tryVerify(() => !cards.parentCard.visible && !cards.mateCard.visible, undefined,
                      "each case starts with nothing out")
            // The landing above held the hand where the last case left it (in a card, as it may be): a fresh hand.
            hand.stillHeld = false
            hand.heldAt = Qt.point(-1, -1)
        }

        /// The point on the count: the line's right end, where the count is laid (`HashPlate.parentSeatRect`).
        function countPoint() {
            const line = row.plate.parentSeatRect(row)
            return Qt.point(line.x + line.width - 3, line.y + line.height / 2)
        }
        function restOnCount() {
            const p = countPoint()
            mouseMove(row, p.x, p.y)
            tryVerify(() => row.parentsPointed, undefined, "the hand is on the count")
            tryVerify(() => cards.parentCardOpen, undefined, "every parent unfolds at once: " + JSON.stringify({
                "state": cards.parentKeep.state, "pending": cards.parentKeep.pending !== null,
                "latched": cards.parentKeep.latchedKey, "seatStill": cards.parentKeep.stillHeld,
                "handStill": hand.stillHeld, "known": hand.known, "at": [hand.at.x, hand.at.y],
                "heldAt": [hand.heldAt.x, hand.heldAt.y], "covered": hand.covered,
                "parents": row.plate.parents.length, "visible": cards.parentCard.visible,
                "opened": cards.parentCard.opened, "holds": Object.keys(cards.parentKeep.holds) }))
        }

        function test_the_count_unfolds_every_parent_and_the_hand_inside_holds_the_card() {
            restOnCount()
            compare(cards.parentKeep.target.sha, commit.shaHex, "the card is this commit's")
            mouseMove(root, cards.parentCard.x + Theme.spaceMd, cards.parentCard.y + Theme.spaceMd)
            tryVerify(() => cards.parentsPointerInside, undefined, "the hand is in the card")
            // waits(paced): the subject is a card that must **not** go; the wait is the beat it would go on.
            wait(Metrics.hoverKeepMs * 2)
            verify(cards.parentCardOpen, "held by the hand inside it")
        }

        /// The card closing by itself (Escape, a press outside) is a dismissal: the count under the same still hand
        /// asks again, and is refused until the hand has walked away and come back.
        function test_a_dismissed_card_is_not_put_back_under_the_same_still_hand() {
            restOnCount()
            cards.parentCard.close()
            tryVerify(() => !cards.parentCard.visible, undefined, "dismissed")
            // waits(paced): a card that must **not** come back; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!cards.parentCardOpen, "not put back under the same still hand")
            // Off the count and back: a hover of its own.
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !row.parentsPointed)
            restOnCount()
        }

        /// A press elsewhere puts the card down, the hand having gone from the count into the card and out of it to
        /// press: no leave of the count's is left to tell, and the hand is first heard off the count — so back on the
        /// count every parent unfolds again (`HoverSeat.expands`).
        function test_a_card_put_down_by_a_press_elsewhere_comes_back_with_the_hand_on_the_count() {
            restOnCount()
            mouseMove(root, cards.parentCard.x + Theme.spaceMd, cards.parentCard.y + Theme.spaceMd)
            tryVerify(() => cards.parentsPointerInside && !row.parentsPointed, undefined, "the hand is in the card")
            mouseClick(root, 1, root.height - 1)
            tryVerify(() => !cards.parentCard.visible, undefined, "the press outside put the card down")
            mouseMove(root, 2, root.height - 2)
            tryVerify(() => hand.known && !row.parentsPointed, undefined, "the hand is heard off the count")
            restOnCount()
        }

        /// The name's card, put down by a press on the name, stays down for the hand on the name; out of the window
        /// and back — the pane never hears it go, and no menu stands — the hand asks afresh.
        function test_the_names_card_put_down_comes_back_with_the_hand_back_from_outside_the_window() {
            const name = row.fieldFor("author")
            mouseMove(name, name.width / 2, name.height / 2)
            tryVerify(() => cards.authorCardOpen, undefined, "the name's card unfolds at once")
            mouseClick(name, name.width / 2, name.height / 2)
            tryVerify(() => !cards.authorCard.visible, undefined, "the press on the name put it down")
            mouseMove(name, name.width / 2 + 2, name.height / 2)
            // waits(paced): a card that must **not** come back for a move on the name; the beat, twice over.
            wait(Metrics.hoverKeepMs * 2)
            verify(!cards.authorCardOpen, "not put back for a move on the name")
            mouseMove(root, root.width + 10, name.mapToItem(root, 0, name.height / 2).y)
            tryVerify(() => !hand.known && !row.authorPointed, undefined, "the hand is out of the window")
            mouseMove(name, name.width / 2, name.height / 2)
            tryVerify(() => cards.authorCardOpen, undefined, "back on the name, the card unfolds again")
        }

        /// Another commit lands with the hand in the card: the card goes, and nothing of the new commit's opens under
        /// the hand where it stood — the count under it is refused until the hand has walked (`HandWatch.walked`).
        function test_another_commit_landing_takes_the_card_down_and_holds_the_hand() {
            restOnCount()
            // In the card where it stands over the count: once the card has gone, the hand is on the count.
            const count = countPoint()
            mouseMove(row, count.x - 1, count.y)
            tryVerify(() => cards.parentsPointerInside, undefined, "the hand is in the card, over the count")
            commit.land("4d2c4401aaaabbbbccccddddeeeeffff00002222", root.parentsOf("2", 24))
            tryVerify(() => !cards.parentCard.visible, undefined, "the card went with its commit")
            tryVerify(() => row.parentsPointed, undefined, "the hand is on the new commit's count")
            verify(hand.stillHeld, "the hand is held: the content moved under it")
            // waits(paced): a card that must **not** come; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!cards.parentCardOpen, "nothing of the new commit's opens under a hand that did not move")
            // A walk onto the count: the new commit's parents.
            const p = countPoint()
            mouseMove(row, p.x, p.y)
            tryVerify(() => cards.parentCardOpen, undefined, "the hand walked onto the count: the card opens")
            compare(cards.parentKeep.target.sha, commit.shaHex, "and it is the new commit's")
        }

        /// A parent pressed in the card is the reader going to that commit: its details land, the card goes with the
        /// commit it was of, and nothing of the new commit's opens under the hand where it pressed until it walks.
        function test_a_parent_pressed_in_the_card_lands_that_commit_and_takes_the_card_down() {
            restOnCount()
            const first = commit.parentHexes[0]
            mouseClick(root, cards.parentCard.x + cards.parentCard.hashInset + Theme.spaceXs,
                       cards.parentCard.y + cards.parentCard.firstMiddle)
            tryVerify(() => commit.shaHex === first, undefined, "the parent pressed is the commit shown now")
            tryVerify(() => !cards.parentCard.visible, undefined, "the card went with the commit it was of")
            // waits(paced): a card that must **not** come; the rest it would take, twice over.
            wait(Metrics.tipDelayMs * 2)
            verify(!cards.parentCardOpen, "nothing of the new commit's opens under a hand that did not move")
            // The card stood over the count: the hand is on it where it pressed, a tremor from it. Off and back is
            // the walk.
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !row.parentsPointed)
            restOnCount()
            compare(cards.parentKeep.target.sha, first, "the new commit's parents")
        }

        /// The credit line's card is the commit's too: another commit landing while it stands takes it down, and
        /// it does not come back on its own.
        function test_another_commit_landing_takes_the_mates_card_down() {
            commit.coAuthors = [{ "name": "Pair One", "email": "one@example.com", "face": 1 },
                                { "name": "Pair Two", "email": "two@example.com", "face": 2 }]
            tryVerify(() => row.matesSaid !== "-", undefined, "the credit line stands")
            cards.pointMates(Qt.point(0, 0))
            tryVerify(() => cards.matesCardOpen, undefined, "the mates' card is out")
            compare(cards.mateKeep.target.sha, commit.shaHex)
            commit.land("5e3d5512aaaabbbbccccddddeeeeffff00003333", root.parentsOf("3", 24))
            tryVerify(() => !cards.mateCard.visible, undefined, "the card went with its commit")
            // waits(paced): a card that must **not** come back on its own.
            wait(Metrics.tipDelayMs)
            verify(!cards.matesCardOpen, "and stays down")
            cards.settleMates()
        }
    }
}
