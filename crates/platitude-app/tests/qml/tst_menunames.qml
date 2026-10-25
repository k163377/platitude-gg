import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Names in a menu row (デザイン規約 §メニュー「入りきらない行は名前だけを真ん中で縮める」, §寸法 `nameMaxChars`): a name
// in the sentence held under the name ceiling and cut in the middle; two names in a short room, the longer giving way
// first down to an even share; the room read off the row's bids; a row whose words are one name, the far column's name
// and the worktree after the words held under the ceiling, and the hover's line seating every name behind its mark.
Control {
    id: root
    width: 2400
    height: 300
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    readonly property string farName:
        "bugfix/keep-the-long-branch-names-readable-in-every-menu-card-tooltip-question-bar-notice-and-the-button"
    readonly property string otherFar:
        "upstream-mirror-of-the-shared-build-server-in-the-eu-west-data-centre-number-two/the-same-long-branch"

    // The names' own face: a name in a menu is set as code (デザイン規約 §ref の種別「文の中の名前」).
    TextMetrics {
        id: capRuler
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontMd
        text: "n".repeat(Theme.nameMaxChars)
    }
    // …and the row's face, for words that are one name (`textNames`).
    TextMetrics {
        id: wordsCapRuler
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontMd
        text: "n".repeat(Theme.nameMaxChars)
    }
    TextMetrics {
        id: farRuler
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontMd
        text: root.farName
    }

    // A sentence with one name, given all the room it could want.
    AppMenuItem {
        id: oneName
        width: 2200
        code: "merge"
        refSentence: "into %1"
        refName: root.farName
        refTint: Theme.textPrimary
        refMark: "branch"
    }
    // Two names, one short, in a room too short for both whole.
    AppMenuItem {
        id: twoNames
        y: 30
        width: 520
        code: "push --delete"
        refSentence: "%1 from %2"
        refName: "v1.0"
        refTint: Theme.textPrimary
        refMark: "tag"
        refName2: root.otherFar
        refTint2: Theme.textSecondary
        refMark2: "remote"
    }
    // Two long names in a room shorter than either.
    AppMenuItem {
        id: bothFar
        y: 60
        width: 420
        refSentence: "Replace %1 with %2"
        refName: root.farName
        refMark: "remote"
        refName2: root.otherFar
        refMark2: "remote"
    }
    // A row whose words are one name, the worktree after it and the branch in the far column, all past the ceiling.
    AppMenuItem {
        id: nameRow
        y: 90
        width: 2200
        text: root.farName
        textNames: true
        trail: root.otherFar + "-worktree-folder"
        sideName: root.farName + "-on-the-side"
    }
    // A row that is one code name with no mark of its own, as the WORKTREE card's (its title wears the mark), past the
    // ceiling, the branch it has out in the far column.
    AppMenuItem {
        id: bareRow
        y: 120
        width: 2200
        codeName: root.farName
        textNames: true
        sideName: "side/branch"
    }

    // A row that says no name, as most of a card's are (a branch card builds one per branch).
    AppMenuItem {
        id: plainRow
        y: 150
        width: 400
        text: "Fetch"
    }

    /// How many items under `item` the test holds true of.
    function count(item, holds) {
        let n = 0
        for (const child of item.children)
            n += (holds(child) ? 1 : 0) + root.count(child, holds)
        return n
    }
    /// The first item under `item` the test holds true of, or null.
    function find(item, holds) {
        for (const child of item.children) {
            if (holds(child))
                return child
            const found = root.find(child, holds)
            if (found !== null)
                return found
        }
        return null
    }

    TestCase {
        name: "MenuNames"
        when: windowShown

        function test_a_name_in_the_sentence_is_held_under_the_ceiling() {
            verify(farRuler.advanceWidth > capRuler.advanceWidth, "the name runs past the ceiling")
            const cut = oneName.refCuts[0]
            verify(cut >= 0 && cut <= capRuler.advanceWidth + 0.5, "held: " + cut + " of " + capRuler.advanceWidth)
            verify(oneName.refShown.indexOf("…") > 0, "cut in the middle: " + oneName.refShown)
            verify(oneName.refCut, "the hover gives the line back")
            verify(oneName.wordsWant < farRuler.advanceWidth, "the bid is held too: " + oneName.wordsWant)
        }

        function test_the_longer_name_gives_way_first() {
            tryVerify(() => twoNames.refRoom > 0, undefined, "laid out")
            compare(twoNames.refCuts[0], -1, "the short name whole")
            verify(twoNames.refCuts[1] > 0, "the long one cut: " + twoNames.refCuts[1])
            verify(twoNames.refShown2.indexOf("…") > 0, twoNames.refShown2)
        }

        function test_two_long_names_share_the_room_evenly() {
            tryVerify(() => bothFar.refRoom > 0, undefined, "laid out")
            verify(bothFar.refCuts[0] > 0 && bothFar.refCuts[1] > 0)
            fuzzyCompare(bothFar.refCuts[0], bothFar.refCuts[1], 0.5, "an even share")
        }

        /// The words' room is the row less what it bids beside them — never the words' label, whose width answers the
        /// text the room cuts.
        function test_the_room_is_the_row_less_its_bids() {
            tryVerify(() => twoNames.refRoom > 0, undefined, "laid out")
            fuzzyCompare(twoNames.refRoom, twoNames.width - twoNames.besideWords - 1, 0.5)
        }

        function test_a_name_row_and_its_runs_are_held_under_the_ceiling() {
            verify(nameRow.implicitWidth - nameRow.besideWords <= Math.ceil(wordsCapRuler.advanceWidth) + 1,
                   "the words bid at most the ceiling")
            // …and are drawn no wider, cut in the middle, however wide the row.
            const words = root.find(nameRow, item => item.text === root.farName && item.truncated !== undefined)
            tryVerify(() => words.truncated, undefined, "the name is cut at the ceiling")
            verify(words.width <= Math.ceil(wordsCapRuler.advanceWidth) + words.rightPadding + 0.5,
                   "drawn no wider than the ceiling: " + words.width)
            compare(words.elide, Text.ElideMiddle)
            tryVerify(() => nameRow.trailCut, undefined, "the worktree run cut")
            tryVerify(() => nameRow.sideCut, undefined, "the far column's name cut")
        }

        /// A row that is one code name bids the ceiling in the code face — not the `labelColW` a name beside words
        /// bids — draws no mark, cuts in the middle, and its hover gives it back as bare code.
        function test_a_bare_code_name_row_bids_the_ceiling() {
            const chip = root.find(bareRow, item => item.word === root.farName && item.markRoom !== undefined)
            verify(chip !== null && chip.visible, "drawn as a code chip")
            compare(chip.markRoom, 0, "no mark")
            const bid = bareRow.implicitWidth - bareRow.leftPadding - bareRow.rightPadding - bareRow.sideWidth
                        - Theme.spaceLg
            verify(bid > Metrics.labelColW, "past the column a name beside words bids: " + bid)
            verify(bid <= Math.ceil(capRuler.advanceWidth) + 1, "held at the ceiling: " + bid)
            tryVerify(() => bareRow.nameCut, undefined, "cut")
            compare(chip.cutAt, "middle")
            const side = root.find(bareRow, item => item.word === "side/branch" && item.markRoom !== undefined)
            compare(side.mark, "branch", "the far column's branch behind its mark")
            verify(Qt.colorEqual(side.tint, Theme.textSecondary), "a step down with the column")
            verify(chip.mapToItem(bareRow, chip.width, 0).x < side.mapToItem(bareRow, 0, 0).x,
                   "the name ahead of the far column")
            fuzzyCompare(side.mapToItem(bareRow, side.width, 0).x, bareRow.width - bareRow.rightPadding, 1,
                         "the far column at the row's end")
            compare(bareRow.Accessible.name, root.farName)
            const marks = bareRow.tipMarks
            compare(marks.length, 2)
            compare(marks[0][1], "", "bare")
            compare(bareRow.tip.words.substr(marks[0][0], marks[0][3]), root.farName)
        }

        /// A row that says no name builds nothing that draws one: its own heading and seat marks, and no name's mark,
        /// ground or chip (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」).
        function test_a_row_with_no_name_builds_no_name_parts() {
            const names = ["branch", "remote", "tag", "tree", "home", "stash"]
            compare(root.count(plainRow, item => item.wordCell !== undefined && item.set === "sign"
                                                 && names.indexOf(item.kind) >= 0), 0, "no name's mark")
            compare(root.count(plainRow, item => item.wantWidth !== undefined), 0, "no chip")
            compare(root.count(plainRow, item => item.radius !== undefined && item.size !== undefined), 1,
                    "no name's ground: the one is the command's, the row's own slot (`code`)")
        }

        /// The hover's line seats every name it says behind its mark: the worktree's tree, the far column's branch.
        function test_the_hover_seats_every_name_behind_its_mark() {
            const line = nameRow.tip.words
            const marks = nameRow.tipMarks
            compare(marks.length, 2)
            compare(marks[0][1], "tree")
            compare(line.substr(marks[0][0], marks[0][3]), nameRow.trail)
            compare(marks[1][1], "branch")
            compare(line.substr(marks[1][0], marks[1][3]), nameRow.sideName)
        }
    }
}
