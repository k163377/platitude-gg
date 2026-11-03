import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The question bar's two columns (デザイン規約 §立っている質問は 1 か所で聞く): the words on the left, packed from the
// top; the ✕ at the top of the right column and the pill at its foot, their right edges on one line. Every shape a
// question can take — a form or not, a line under the heading or not — keeps the same seats, but a heading alone that
// would stand on one line in the two columns, which puts the heading, the pill and the ✕ on one row.
//
// A control, to hand the bar the window's font as `Main` does: the boxes' widths are that face's, fractions and all.
Control {
    id: root
    width: 640
    height: 400
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    /// A heading that stands whole on one row at the root's width.
    readonly property string shortHeading: "all uncommitted changes and switch to main?"
    /// One that wraps in the two columns of a 560 bar.
    readonly property string longHeading: "all uncommitted changes including conflicts and switch to feature/clash?"

    AskBar {
        id: bar
        width: root.width
        code: "stash"
        label: root.shortHeading
        accept: ""
    }
    /// The ✕'s mark as the bar asks it for its air (`AskBar.askColumns.closeInk`).
    NavIcon {
        id: closeInk
        visible: false
        kind: "close"
        width: bar.closeButton.markSize
        height: bar.closeButton.markSize
    }
    Component {
        id: publishForm
        PublishForm {
            choices: ["origin"]
            remote: "origin"
            branch: "feature/new-thing"
        }
    }

    /// A bar no form has stood in yet, at the graph's floor.
    Component {
        id: freshBar
        AskBar {
            width: 335
            code: "push"
            label: "Push it where?"
            accept: ""
        }
    }

    TestCase {
        name: "AskSeats"
        when: windowShown

        function init() {
            bar.open = false
            tryVerify(() => bar.shut)
            bar.width = root.width
            bar.code = "stash"
            bar.label = root.shortHeading
            bar.detailMarks = []
        }

        /// Left, top, right and bottom of `item` in the bar's own coordinates.
        function edges(item) {
            const p = item.mapToItem(bar, 0, 0)
            return { left: p.x, top: p.y, right: p.x + item.width, bottom: p.y + item.height }
        }

        function test_every_shape_keeps_the_two_columns_data() {
            return [
                { tag: "a form and a line", line: "origin/feature/new-thing does not exist yet; this makes it.",
                  form: true },
                { tag: "a form alone", line: "", form: true },
                { tag: "a line alone", line: "The cherry-pick ends here.", form: false },
                // Wrapped in the two columns anyway, so not one row (`askColumns.oneRow`).
                { tag: "a heading alone that wraps in the two columns", line: "", form: false,
                  heading: root.longHeading, width: 560 },
            ]
        }

        function test_every_shape_keeps_the_two_columns(data) {
            if (data.heading !== undefined)
                bar.label = data.heading
            if (data.width !== undefined)
                bar.width = data.width
            bar.detail = data.line
            bar.form = data.form ? publishForm : null
            bar.open = true
            tryVerify(() => bar.settled)

            const words = edges(bar.words)
            const pill = edges(bar.pill)
            const close = edges(bar.closeButton)
            // The ✕ draws its mark centred in its seat.
            const markRight = close.right - (bar.closeButton.width - bar.closeButton.markSize) / 2

            compare(pill.right, markRight, "the pill's right edge is the ✕'s mark's")
            verify(close.bottom <= pill.top, "the ✕'s seat ends at " + close.bottom + ", over the pill at " + pill.top)
            verify(words.right + Theme.spaceMd <= pill.left,
                   "the words end at " + words.right + ", clear of the pill at " + pill.left)
            verify(words.right + Theme.spaceMd <= markRight - bar.closeButton.markSize,
                   "and of the ✕'s mark")
            compare(words.top, Theme.spaceMd, "the words start at the top")
            compare(pill.bottom + Theme.spaceMd, bar.height, "the pill stands at the foot, the bar's margin under it")
            verify(words.bottom + Theme.spaceMd <= bar.height, "the words fit inside the bar")
            if (data.form) {
                const form = edges(bar.formItem)
                compare(form.bottom, pill.bottom, "the pill shares the form's line")
                compare(form.bottom - form.top, pill.bottom - pill.top, "at the form's height")
            }
        }

        /// Words shorter than the right column leave the height to it: the pill `spaceXs` under the ✕'s seat, and the
        /// ✕'s mark up in the corner, its top on the heading's.
        function test_a_heading_and_a_line_take_the_height_of_the_close_and_the_pill() {
            bar.detail = "The cherry-pick ends here."
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            const close = edges(bar.closeButton)
            compare(edges(bar.pill).top, close.bottom + Theme.spaceXs, "the pill spaceXs under the ✕'s seat")
            compare(close.top + (bar.closeButton.height - bar.closeButton.markSize) / 2, edges(bar.heading).top,
                    "the ✕'s mark's top on the heading's")
        }

        /// Where the heading's first line and the pill's word stand, in the bar's own y.
        function headingBase() {
            return bar.headingWords.mapToItem(bar, 0, bar.headingWords.baselineOffset).y
        }
        function pillBase() {
            return bar.pillWord.mapToItem(bar, 0, bar.pillWord.base).y
        }

        /// A heading alone, on one line in the two columns: the three on one row, the heading on the pill's word's
        /// baseline, the ✕'s mark still in the corner, on the pill's middle, and the pill a step short of its ink.
        function test_a_heading_alone_stands_on_one_row_with_the_pill_and_the_close() {
            bar.detail = ""
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            const words = edges(bar.words)
            const heading = edges(bar.heading)
            const pill = edges(bar.pill)
            const close = edges(bar.closeButton)
            const markRight = close.right - (bar.closeButton.width - bar.closeButton.markSize) / 2
            const inkLeft = markRight - bar.closeButton.markSize + (closeInk.width - closeInk.inkRight)
            const middle = edge => (edge.top + edge.bottom) / 2
            compare(markRight + Theme.spaceMd, bar.width, "the ✕'s mark keeps its corner")
            compare(pill.right + Theme.spaceSm, inkLeft, "the pill a step short of the ✕'s ink")
            verify(words.right + Theme.spaceMd <= pill.left,
                   "the words end at " + words.right + ", clear of the pill at " + pill.left)
            verify(Math.abs(headingBase() - pillBase()) <= 0.5,
                   "the heading's baseline " + headingBase() + " on the pill's word's " + pillBase())
            verify(Math.abs(middle(close) - middle(pill)) <= 0.5,
                   "the ✕'s middle " + middle(close) + " on the pill's " + middle(pill))
            compare(bar.height, Math.max(heading.bottom, pill.bottom) + Theme.spaceMd, "one row high")
        }

        /// A heading one line long in the two columns but not beside the ✕ as well: still one row, the heading on two
        /// lines and the pill and the ✕ on its first — never the pill a row under a single line.
        function test_a_heading_the_close_wraps_keeps_one_row() {
            // Long, so the width it asks for stays clear of the words' floor beside the pill (`pillUnder`).
            bar.label = root.longHeading
            bar.detail = ""
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            // Two pixels inside the two columns' words: the ✕'s room on the row takes more than that.
            bar.width = bar.heading.implicitWidth + 2 * Theme.spaceMd + bar.pill.width + Theme.spaceMd + 2
            const heading = () => edges(bar.heading)
            tryVerify(() => heading().bottom - heading().top > bar.pill.height, undefined,
                      "the heading wraps on the row")
            const pill = edges(bar.pill)
            const close = edges(bar.closeButton)
            const markRight = close.right - (bar.closeButton.width - bar.closeButton.markSize) / 2
            const inkLeft = markRight - bar.closeButton.markSize + (closeInk.width - closeInk.inkRight)
            const middle = edge => (edge.top + edge.bottom) / 2
            compare(pill.right + Theme.spaceSm, inkLeft, "the pill still beside the ✕")
            verify(Math.abs(headingBase() - pillBase()) <= 0.5,
                   "the pill's word " + pillBase() + " on the heading's first line " + headingBase())
            verify(Math.abs(middle(close) - middle(pill)) <= 0.5, "and the ✕'s mark on the pill's middle")
        }

        /// At the graph's floor a heading alone takes the floor's shape like any question — the pill under the words —
        /// however short: no row beside the pill keeps the words their floor (`pillUnder` ahead of `oneRow`).
        function test_a_heading_alone_at_the_floor_puts_the_pill_under_it() {
            bar.width = 335
            bar.code = "push"
            bar.label = "Push it where?"
            bar.detail = ""
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            const words = edges(bar.words)
            const pill = edges(bar.pill)
            verify(bar.heading.implicitWidth + 2 * Theme.spaceMd + bar.pill.width + Theme.spaceMd <= bar.width,
                   "the fixture: the heading would fit one line beside the pill")
            compare(pill.top, words.bottom + Theme.spaceSm, "the pill a form's gap under the heading")
            compare(words.right + Theme.spaceMd, bar.width, "the words take the whole width")
        }

        /// The graph at its floor: the form's two boxes on their floor do not fit beside the pill, so the pill stands
        /// on a line of its own under the form, its right edge still the ✕'s, and the line over the form runs the
        /// whole width — nothing stands beside it any more.
        function test_a_form_too_wide_beside_the_pill_puts_the_pill_under_it() {
            bar.width = 335
            bar.detail = "origin/feature/new-thing does not exist yet; this makes it."
            bar.form = publishForm
            bar.open = true
            tryVerify(() => bar.settled)
            const form = edges(bar.formItem)
            const pill = edges(bar.pill)
            const close = edges(bar.closeButton)
            const markRight = close.right - (bar.closeButton.width - bar.closeButton.markSize) / 2
            compare(pill.top, form.bottom + Theme.spaceSm, "the pill a form's gap under the form")
            compare(pill.right, markRight, "the pill's right edge is still the ✕'s mark's")
            compare(pill.bottom + Theme.spaceMd, bar.height, "the pill at the foot, the bar's margin under it")
            compare(edges(bar.words).right + Theme.spaceMd, bar.width, "the words take the whole width")
            verify(edges(bar.heading).right + Theme.spaceMd <= markRight - bar.closeButton.markSize,
                   "but the heading keeps clear of the ✕")
            // The boxes give way to fit the row (`PublishForm.askRow.narrow`): nothing is cut at the bar's edge.
            verify(edges(bar.formItem.branchField).right <= edges(bar.words).right + 0.5,
                   "the name box ends at " + edges(bar.formItem.branchField).right + ", inside the words at "
                   + edges(bar.words).right)
        }

        /// Widening the bar from the graph's floor past where the pill moves up beside the words never narrows the box
        /// the name is typed in: past the two floors the destination comes back to its floor, and where the pill moves
        /// up the destination gives the pill's width, down to its own name's (`PublishForm.askRow`).
        function test_widening_the_bar_never_narrows_the_typed_name() {
            bar.detail = ""
            bar.form = publishForm
            bar.width = 335
            bar.open = true
            tryVerify(() => bar.settled && bar.formItem !== null)
            verify(edges(bar.pill).top >= edges(bar.formItem).bottom, "the pill starts under the form")
            let was = null
            for (let width = 335; width <= 520; width += 4) {
                bar.width = width
                tryVerify(() => Math.abs(edges(bar.formItem.branchField).right - edges(bar.words).right) <= 1)
                const name = bar.formItem.branchField.width
                const remote = bar.formItem.remotePick
                verify(remote.width >= remote.wantedWidth - 1, "at " + width + " the destination cuts its own name")
                if (was !== null)
                    verify(name >= was - 4, "at " + width + " the name's box went from " + was + " to " + name)
                was = name
            }
            verify(edges(bar.pill).left > edges(bar.words).right, "and ends beside the words")
        }

        /// A form arriving in a bar already laid out without one is handed the column's width as it loads
        /// (`formLoader`): sized by itself, it grows a pixel a pass and Qt gives up on the loop.
        function test_a_form_loaded_at_the_floor_takes_the_columns_width() {
            failOnWarning(/polish\(\) loop/)
            const fresh = createTemporaryObject(freshBar, root)
            fresh.open = true
            tryVerify(() => fresh.settled)
            fresh.form = publishForm
            tryVerify(() => fresh.settled && fresh.formItem !== null && fresh.formItem.width === fresh.words.width)
        }

        /// No form, the same: a code chip and a wide pill beside the words would leave the heading too little to
        /// keep its words whole, so the pill stands under the line, and the heading and the line take the width.
        function test_words_too_narrow_beside_the_pill_put_the_pill_under_them() {
            bar.width = 335
            bar.code = "rebase --abort"
            bar.detail = "Nothing it did since it started is kept."
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            const words = edges(bar.words)
            const pill = edges(bar.pill)
            const close = edges(bar.closeButton)
            const markRight = close.right - (bar.closeButton.width - bar.closeButton.markSize) / 2
            compare(pill.top, words.bottom + Theme.spaceSm, "the pill a form's gap under the line")
            compare(pill.right, markRight, "its right edge the ✕'s mark's")
            compare(words.right + Theme.spaceMd, bar.width, "the words take the whole width")
            compare(pill.bottom + Theme.spaceMd, bar.height, "the pill at the foot")
        }

        /// No form, no room for one: the words end at their last line, so the pill's foot is not pushed under it by
        /// the column's spacing.
        function test_words_with_no_form_end_at_their_last_line() {
            bar.detail = "The cherry-pick ends here."
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            compare(edges(bar.words).bottom, edges(bar.detailLine).bottom, "the words' foot is the line's")
        }

        /// The line under the heading cuts a name past the name ceiling as the heading does (`CardText.holdsNames`).
        function test_the_line_cuts_a_name_past_the_ceiling() {
            const name = "feature/" + "a-branch-whose-name-is-far-too-long-".repeat(3) + "end"
            bar.detail = name + " loses 1 commit."
            bar.detailMarks = [[0, "branch", Theme.accent, name.length]]
            bar.form = null
            bar.open = true
            tryVerify(() => bar.settled)
            compare(bar.detailLine.held.cuts.length, 1, "the name is cut: " + bar.detailLine.held.text)
        }

        /// The pill's tip opens under the pill: over it stand the heading and the ✕ (`TipAsk.under`).
        function test_the_pills_tip_opens_under_it() {
            verify(bar.pill.pillTip.under)
        }
    }
}
