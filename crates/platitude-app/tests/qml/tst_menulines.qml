import QtQuick
import QtTest
import platitude
import platitude.ui
import platitude.ui.words

// A menu drawn off core's answer (`MenuLines`): the rows declared in any order stand in the answer's, parted where the
// group changes, the cards' titles at their seats; standing again with the menu up moves only what each row says, and
// keeps the row a hand is pressing; and when a standing menu stands again.
Item {
    id: root
    width: 360
    height: 360

    /// Why the menu last stood again (`MenuLines.standAgain`), and the answer the opener hands it then.
    property string stood: ""
    property var next: undefined
    /// The rows not on show the keys stood on (`TestCase.noteStop`), a card's title row too — each a row Return would
    /// run.
    property var hiddenStops: []
    /// The row of `keyMenu` its last press picked, which takes it off offer; and the rows Return ran there.
    property string picked: ""
    property var ran: []
    /// Whether the row of `heldMenu`'s card outside the type is on offer — what `GitHubMenu.stillApplies` says of its
    /// links.
    property bool outsideApplies: true

    readonly property MenuLines lines: MenuLines {
        menu: top
        onStandAgain: again => {
            root.stood = again
            if (root.next !== undefined)
                root.lines.answer = root.next
        }
    }
    readonly property MenuLines heldLines: MenuLines {
        menu: heldMenu
    }

    /// Core's answer for `rows` — each row's word to why it greys ("" while live), in drawing order —, seated as core
    /// seats it (the test double's table, `GitFacts.seated`).
    function answer(rows) {
        const words = Object.keys(rows)
        return GitFacts.seated({ "rows": words, "whys": words.map(word => rows[word]) })
    }
    readonly property var everyRow: ({ "branch-here": "", "reach": "", "merge": "", "reset-soft": "",
                                       "delete-branch": "", "tag-here": "", "delete-tag": "" })

    AppMenu {
        id: top
        // Declared out of drawing order on purpose: the answer orders them.
        AppMenuItem {
            id: mergeRow
            code: "merge"
            word: "merge"
        }
        AppMenuItem {
            text: "Create branch here…"
            word: "branch-here"
        }
        AppMenuItem {
            id: deleteRow
            code: "branch --delete"
            word: "delete-branch"
            holdPresses: 3
            premise: "topic"
        }
        AppMenu {
            word: "tag"
            title: "TAG"
            AppMenuItem {
                code: "tag --delete"
                word: "delete-tag"
            }
            AppMenuItem {
                text: "Create tag here…"
                word: "tag-here"
            }
        }
        AppMenu {
            id: resetCard
            word: "reset"
            title: "reset"
            AppMenuItem {
                code: "--soft"
                word: "reset-soft"
            }
        }
        AppMenuItem {
            id: reachRow
            code: "switch"
            word: "reach"
        }
        AppMenuSeparator {}
        AppMenuItem {
            text: "Copy path"
            startsCommand: false
        }
    }

    // A menu outside the type, for the keys: rows that take themselves off offer as they are picked — `TopBar`'s
    // REPOSITORY card, whose row for a tab goes off offer as the tab it picks becomes the current one — and a card
    // whose first row declared is off show.
    AppMenu {
        id: keyMenu
        AppMenuItem {
            text: "one"
            offered: root.picked !== "one"
            onTriggered: root.picked = "one"
        }
        AppMenuItem {
            text: "two"
            offered: root.picked !== "two"
            onTriggered: root.picked = "two"
        }
        AppMenu {
            id: keyCard
            title: "CARD"
            AppMenuItem {
                text: "first"
                offered: false
                onTriggered: root.ran.push(text)
            }
            AppMenuItem {
                text: "second"
                onTriggered: root.ran.push(text)
            }
        }
    }

    // Two cards of rows none of which can be chosen: one of commands, and one listing places to go to, as `TopBar`'s
    // WORKTREE card does.
    AppMenu {
        id: titledMenu
        AppMenu {
            id: commandsCard
            title: "COMMANDS"
            AppMenuItem {
                code: "merge"
                doable: false
            }
        }
        AppMenu {
            id: placesCard
            title: "PLACES"
            greysWithRows: false
            AppMenuItem {
                text: "here"
                doable: false
            }
        }
    }

    // A menu of the type carrying a card outside it, as `RefRowMenu` carries the GITHUB card.
    AppMenu {
        id: heldMenu
        AppMenuItem {
            code: "merge"
            word: "merge"
        }
        AppMenu {
            title: "OUTSIDE"
            AppMenuItem {
                id: outsideRow
                text: "Copy link"
                startsCommand: false
                offered: root.outsideApplies
            }
        }
    }

    /// The follow a move asks for, a turn later (`MenuLines.followLater`).
    SignalSpy {
        id: followCame
        target: root.lines.followLater
        signalName: "triggered"
    }

    TestCase {
        name: "MenuDrawnOffTheAnswer"
        when: windowShown

        function cleanup() {
            top.dismiss()
            keyMenu.dismiss()
            heldMenu.dismiss()
            root.heldLines.release()
            root.outsideApplies = true
            root.lines.waiting = false
            root.stood = ""
            root.next = undefined
            root.hiddenStops = []
            root.picked = ""
            root.ran = []
            followCame.clear()
        }
        /// ↓ and ↑ over every row the level holds, on show or not, noting each stop.
        function walk(level) {
            for (let i = 0; i < level.count; i++) {
                keyClick(Qt.Key_Down)
                noteStop(level)
            }
            for (let i = 0; i < level.count; i++) {
                keyClick(Qt.Key_Up)
                noteStop(level)
            }
        }
        /// The row the keys stand on in `level` (its `currentIndex`), noted where it is not on show.
        function noteStop(level) {
            const row = level.currentIndex >= 0 ? level.itemAt(level.currentIndex) : null
            if (row !== null && row.present === false)
                root.hiddenStops.push(row.subMenu ? row.subMenu.title : row.word !== "" ? row.word : row.text)
        }
        /// A fresh stand on `rows`, as an opener makes it.
        function standOn(rows) {
            root.lines.release()
            root.lines.answer = root.answer(rows)
        }
        /// The level as drawn: each row on show by its kind (a card's title by its card, a row outside the type by its
        /// words), `|` for a divider that draws.
        function shown(level) {
            const out = []
            for (let i = 0; i < level.count; i++) {
                const item = level.itemAt(i)
                if (item.codeColSeat === undefined) {
                    if (item.dividing)
                        out.push("|")
                } else if (item.present) {
                    out.push(item.subMenu ? item.subMenu.word : item.word !== "" ? item.word : item.text)
                }
            }
            return out.join(" ")
        }
        function cardOf(word) {
            for (let i = 0; i < top.count; i++) {
                const item = top.itemAt(i)
                if (item.subMenu && item.subMenu.word === word)
                    return item.subMenu
            }
            return null
        }
        /// One press, as a hand makes it.
        function pressOnce(row) {
            row.holdDrive.begin()
            row.holdDrive.letUp()
        }

        function test_rows_stand_in_the_answers_order_parted_by_group() {
            standOn(root.everyRow)
            compare(shown(top), "branch-here reach | merge reset | delete-branch | tag | Copy path",
                    "the RESET card's title in its seat beside merge, the row outside the type last")
            compare(shown(cardOf("tag")), "tag-here | delete-tag", "a card is drawn by its own groups")
            compare(shown(cardOf("reset")), "reset-soft")
        }

        function test_a_row_the_answer_has_not_is_not_on_offer() {
            standOn({ "branch-here": "", "reach": "", "delete-branch": "", "tag-here": "" })
            compare(shown(top), "branch-here reach | delete-branch | tag | Copy path",
                    "merge out, and the RESET card with no row out with its title")
            verify(!mergeRow.offered)
            compare(shown(cardOf("tag")), "tag-here")
        }

        function test_a_row_greys_with_the_types_reason() {
            standOn(Object.assign({}, root.everyRow, { "merge": "same-commit" }))
            compare(mergeRow.why, "same-commit")
            verify(mergeRow.blocked)
            compare(mergeRow.blockedReason, Words.menuWhy("same-commit"))
            verify(!reachRow.blocked)
            compare(reachRow.blockedReason, "")
        }

        /// What greys a row of the type is its `why`; the words only say it. Words that come out empty — here a place
        /// whose kind the sentence does not know — leave the row greyed all the same, not pressable.
        function test_a_row_greys_by_the_types_reason_whatever_its_words_come_to() {
            mergeRow.whyNames = { "place": { "taken": "", "name": "" } }
            standOn(Object.assign({}, root.everyRow, { "merge": "taken" }))
            compare(mergeRow.blockedReason, "", "the words came out empty")
            verify(mergeRow.blocked, "the type greyed it")
            verify(mergeRow.greyed)
            mergeRow.tipForced = true
            verify(!mergeRow.tip.asked, "and it asks for no box with nothing to say in it")
            top.heldReason = "Wait for it"
            compare(mergeRow.tip.words, "Wait for it", "the menu's own line says it where the row's has no words")
            verify(mergeRow.tip.asked)
            top.heldReason = ""
            mergeRow.tipForced = false
            // The names its reason seats go after the reason: words with none of them throw.
            standOn(root.everyRow)
            mergeRow.whyNames = ({})
        }

        /// A row of the answer the menu has no part for — on its level, or in the card its seat names — stands nowhere,
        /// and the stand counts and says it (`MenuLines.undrawn`): a menu lacking a part its class has is a fault, not
        /// a row quietly left out.
        function test_a_row_with_no_part_is_counted_and_said() {
            ignoreWarning(new RegExp("MenuLines: a row with no part: pull, tag/delete-tag-both$"))
            standOn({ "branch-here": "", "pull": "", "tag-here": "", "delete-tag-both": "" })
            compare(root.lines.undrawn, ["pull", "tag/delete-tag-both"])
            compare(shown(top), "branch-here | tag | Copy path")
            standOn({ "branch-here": "" })
            compare(root.lines.undrawn, [])
        }

        function test_a_fresh_stand_draws_its_own_dividers() {
            standOn(root.everyRow)
            standOn({ "merge": "", "reset-soft": "" })
            compare(shown(top), "merge reset | Copy path", "no divider left over from the stand before")
            let drawn = 0
            for (let i = 0; i < top.count; i++)
                drawn += top.itemAt(i).drawn === true ? 1 : 0
            compare(drawn, 0, "one group draws no divider of its own")
        }

        /// Standing again with the menu up: a row gone greys where it stands, one come waits for the menu to be raised
        /// again, nothing moves, and the row under the hand is the one it was, its presses still counted.
        function test_standing_again_moves_only_what_the_rows_say() {
            standOn({ "branch-here": "", "merge": "", "delete-branch": "", "tag-here": "" })
            verify(top.offer())
            const before = shown(top)
            pressOnce(deleteRow)
            compare(deleteRow.holdDrive.count, 1)

            root.next = root.answer({ "branch-here": "", "reach": "", "delete-branch": "", "tag-here": "" })
            root.lines.again("wait")
            compare(root.stood, "wait")
            verify(mergeRow.withdrawn, "merge gone from the answer greys where it stood")
            verify(!reachRow.present, "switch come waits for the menu to be raised again")
            compare(shown(top), before, "no row moved, no divider came or went")
            compare(deleteRow.holdDrive.count, 1, "the press under way still counts")
            pressOnce(deleteRow)
            compare(deleteRow.holdDrive.count, 2, "and the next press counts on from it")
        }

        /// A divider the menu declares draws only with a row on show on each side: with nothing of the type on offer,
        /// the one ahead of `Copy path` has nothing above it, and draws nothing (`AppMenuSeparator.dividing`).
        function test_a_declared_divider_with_an_empty_side_draws_nothing() {
            standOn({})
            compare(shown(top), "Copy path")
        }

        /// A card whose rows the answer takes away with the menu up keeps its title row as it was opened: greyed with
        /// its rows, and still opening, where each says why (`AppMenuItem.cardOut`) — not withdrawn, which would shut
        /// it.
        function test_a_card_stood_away_keeps_its_title_row_opening() {
            standOn(root.everyRow)
            verify(top.offer())
            const without = Object.assign({}, root.everyRow)
            delete without["reset-soft"]
            root.next = root.answer(without)
            root.lines.again("wait")
            verify(cardOf("reset").applies, "what applies is the fresh stand's")
            let title = null
            for (let i = 0; i < top.count; i++) {
                if (top.itemAt(i).subMenu === cardOf("reset"))
                    title = top.itemAt(i)
            }
            verify(title.present)
            verify(!title.withdrawn, "the title row still opens the card")
            verify(title.cardOut, "greyed with the rows in it")
        }

        /// A card's title greys with a card whose rows none can be chosen, and still opens it — but not over a list of
        /// places to go to, whose title is a heading of contents (`AppMenu.greysWithRows`).
        function test_a_title_greys_with_its_rows_unless_its_card_lists_places() {
            const titleOf = card => {
                for (let i = 0; i < titledMenu.count; i++) {
                    if (titledMenu.itemAt(i).subMenu === card)
                        return titledMenu.itemAt(i)
                }
                return null
            }
            verify(titleOf(commandsCard).cardOut)
            verify(titleOf(commandsCard).greyed)
            verify(!titleOf(commandsCard).blocked, "it still opens its card")
            verify(!titleOf(placesCard).cardOut)
            verify(!titleOf(placesCard).greyed, "a heading of contents stays live")
        }

        /// A card outside the type holds its shape with the menu carrying it: its row going off offer with the menu up
        /// greys where it stands, saying to raise the menu again, rather than leave the card; a fresh stand lets it go.
        function test_a_card_outside_the_type_holds_its_shape_with_the_menu() {
            root.heldLines.release()
            root.heldLines.answer = root.answer({ "merge": "" })
            verify(heldMenu.offer())
            root.heldLines.again("follow")
            root.outsideApplies = false
            verify(outsideRow.present, "still on show")
            verify(outsideRow.withdrawn, "greyed where it stands")
            compare(outsideRow.blockedWhy, outsideRow.withdrawnWhy)
            root.heldLines.release()
            verify(!outsideRow.present, "a fresh stand has it off offer")
        }

        /// The keys stop only on rows on show: one the answer has not is no stop for ↓ or ↑, so Return can never run
        /// it. Here the rows not on offer stand together after the drawn ones, ahead of `Copy path`.
        function test_the_keys_stop_only_on_rows_on_show() {
            standOn({ "branch-here": "", "delete-branch": "", "tag-here": "" })
            verify(top.offer())
            walk(top)
            compare(root.hiddenStops, [], "no row off the menu took the keys")
        }

        /// A row taken off show while the keys are on it gives them up — an item keeps taking the focus by tab while it
        /// holds the active focus (`AppMenuItem.enabled`): here a fresh stand under the open menu leaving out the row
        /// the keys stand on.
        function test_a_row_stood_off_show_under_the_keys_is_no_stop_after() {
            standOn(root.everyRow)
            verify(top.offer())
            keyClick(Qt.Key_Down)
            standOn({ "reach": "", "delete-branch": "", "tag-here": "" })
            walk(top)
            compare(root.hiddenStops, [], "the row the keys stood on is no stop once off the menu")
        }

        /// …and a row its own press takes off offer, from the keys: no stop on the next menu raised.
        function test_a_row_its_press_takes_off_offer_is_no_stop_on_the_next_menu() {
            verify(keyMenu.offer())
            keyClick(Qt.Key_Down)
            keyClick(Qt.Key_Return)
            compare(root.picked, "one")
            verify(keyMenu.offer())
            walk(keyMenu)
            compare(root.hiddenStops, [], "the row its press took off offer is no stop on the next menu")
        }

        /// → opens a card on its first row on show: one declared ahead of it and off show takes neither the focus nor
        /// the Return that follows.
        function test_a_card_opened_by_the_keys_starts_on_a_row_on_show() {
            verify(keyMenu.offer())
            for (let i = 0; i < 3; i++) {
                keyClick(Qt.Key_Down)
                noteStop(keyMenu)
            }
            keyClick(Qt.Key_Right)
            tryVerify(() => keyCard.opened)
            noteStop(keyCard)
            compare(root.hiddenStops, [], "the card opens on a row on show")
            keyClick(Qt.Key_Return)
            compare(root.ran, ["second"])
        }

        function test_a_menu_down_does_not_stand_again() {
            standOn(root.everyRow)
            root.lines.again("wait")
            compare(root.stood, "")
        }

        /// What the rows read moving stands a standing menu again a turn later, while nothing is out; the command the
        /// menu stood under ending does it too.
        function test_a_menu_up_follows_what_its_rows_read() {
            standOn(root.everyRow)
            verify(top.offer())
            root.lines.moved = "one"
            tryCompare(root, "stood", "follow")
            root.stood = ""
            root.lines.waiting = true
            followCame.clear()
            root.lines.moved = "two"
            tryCompare(followCame, "count", 1)
            compare(root.stood, "", "nothing follows while a command is out")
            root.lines.waiting = false
            compare(root.stood, "follow", "its end stands the menu on what it left")
        }

        /// A fresh stand in the turn of a move has read it: the follow the move asked for finds nothing due
        /// (`MenuLines.release`).
        function test_a_fresh_stand_leaves_its_turns_move_nothing_to_follow() {
            standOn(root.everyRow)
            verify(top.offer())
            root.lines.moved = "three"
            standOn(root.everyRow)
            tryCompare(followCame, "count", 1)
            compare(root.stood, "", "the fresh stand read it already")
        }
    }
}
