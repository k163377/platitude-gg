import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A menu row's side: the hold fill reaches past it out to the card's frame (`HoldFill.bleed`), drawn there because the
// card's list clips only while it scrolls (`AppMenu`); a heading's mark centred on the hold seat — the cloud, the
// widest — stays inside the row, under the row's wash (`AppMenuItem.markX`).
Item {
    id: root
    width: 400
    height: 300

    AppMenu {
        id: menu
        AppMenuItem {
            id: held
            text: "drop"
            holdPresses: Metrics.holdPressesWarned
            holdTone: Theme.warning
        }
        AppMenu {
            title: "OTHER REMOTES"
            titleKind: "remote"
            // A hue the card's grounds never wear, to find the mark's ink by.
            titleTint: Theme.danger
            AppMenuItem {
                text: "one"
            }
        }
    }

    TestCase {
        name: "MenuEdge"
        when: windowShown

        function cleanup() {
            menu.dismiss()
            tryVerify(() => !menu.visible)
        }

        /// The heading's mark: the head-set icon the row shows.
        function markOf(row) {
            for (const child of row.children) {
                if (child.set === "head" && child.visible)
                    return child
            }
            return null
        }
        /// How many pixels of `shot` over the row `row`, from scene x `from` up to `to`, `inked` says are ink.
        function inkIn(shot, row, from, to, inked) {
            const top = Math.ceil(row.mapToItem(root, 0, 0).y)
            let found = 0
            for (let y = top; y < top + Math.floor(row.height); y++)
                for (let x = Math.max(0, Math.floor(from)); x < Math.floor(to); x++)
                    if (inked(shot.pixel(x, y)))
                        found++
            return found
        }

        function test_a_headings_mark_stays_in_its_row() {
            verify(menu.offer())
            tryVerify(() => menu.opened)
            const heading = menu.itemAt(1)
            verify(heading.titled)
            verify(menu.holdIndent > 0, "the held row opens the seat")
            const mark = markOf(heading)
            verify(mark !== null)
            tryVerify(() => mark.inked)
            const edge = heading.mapToItem(root, 0, 0).x
            const inkStart = mark.mapToItem(root, mark.inkLeft, 0).x
            verify(inkStart > edge - 0.01, "the mark's ink starts inside the row: " + inkStart + " >= " + edge)
            const shot = grabImage(root)
            const red = c => c.r - Math.max(c.g, c.b) > 0.25
            verify(inkIn(shot, heading, edge, edge + Theme.iconMd, red) > 0, "the mark is drawn")
            compare(inkIn(shot, heading, edge - Theme.spaceSm, edge, red), 0, "and none of it past the row's side")
        }

        function test_the_hold_fill_reaches_the_cards_frame() {
            verify(menu.offer())
            tryVerify(() => menu.opened)
            mouseClick(held)
            tryVerify(() => held.holdProgress > 0)
            const edge = held.mapToItem(root, 0, 0).x
            const amber = c => c.r > 0.5 && c.g > 0.25 && c.b < 0.2
            verify(inkIn(grabImage(root), held, edge - Theme.spaceSm, edge, amber) > 0,
                   "the fill runs past the row's side")
        }
    }
}
