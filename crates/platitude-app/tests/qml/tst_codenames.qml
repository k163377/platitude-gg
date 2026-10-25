import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Names set as code (デザイン規約 §ref の種別「文の中の名前」): a chip's ground starts at its mark's seat, which holds
// the mark's air — so chips start on one x whatever their mark — and ends half a gap past the name's last glyph as
// drawn (`WordFace.groundEnd`), where a command's runs half a gap off both ends of its word — in a chip, a menu's
// column, a sentence and a `NameRun`; a menu's names start on one x and its marks stand on one upright axis whatever
// their kind and however they are put in the row — a name behind its mark at the row's end, a name opening a sentence —
// and a name held at its ceiling in a row with room to spare stays there. A `NameRun` is code behind its kind's colour;
// the conflict legend says the branch stood on in its own.
Control {
    id: root
    width: 1800
    height: 440
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd
    // Dark, as the window's grounds are: the name's ink is read off the picture against it.
    background: Rectangle {
        color: Theme.bgBase
    }

    readonly property string farName:
        "bugfix/keep-the-long-branch-names-readable-in-every-menu-card-tooltip-question-bar-notice-and-the-button"

    Rectangle {
        id: plate
        width: 600
        height: 40
        color: Theme.bgBase
        CodeChip {
            id: chip
            x: 20
            anchors.verticalCenter: parent.verticalCenter
            word: "feature/login"
            mark: "branch"
        }
    }
    // A chip per kind, and per letter with the most and the least air past its ink.
    Column {
        y: 200
        x: 20
        Repeater {
            id: kinds
            model: [["t", "tag"], ["v1.0", "tag"], ["origin/fix", "remote"], ["wt-j", "tree"], ["main", "home"],
                    ["hotfix/f", "branch"], ["topic_", "branch"]]
            CodeChip {
                required property var modelData
                word: modelData[0]
                mark: modelData[1]
            }
        }
    }
    // A command's chip: where a chip's ground starts against its head.
    CodeChip {
        id: commandChip
        x: 700
        y: 270
        word: "push"
    }
    // The worktree runs of a tab, a band and a row.
    NameRun {
        id: treeRun
        x: 700
        y: 200
        name: "wt-t"
        pixelSize: Theme.fontSm
    }
    NameRun {
        id: homeRun
        x: 700
        y: 230
        name: "main"
        kind: "home"
        weight: Theme.fontWeightStrong
    }
    // A legend over a merge into the branch stood on.
    ConflictSideLegend {
        id: legend
        x: 900
        y: 200
        width: 600
        oursName: "main"
        theirsName: "topic"
        oursIsBranch: true
        theirsIsBranch: true
        oursTint: Theme.textLink
        oursColor: Theme.accent
        theirsColor: Theme.warning
        nameCap: 300
    }
    // A remote's name too long for its room: in a menu's sentence, and in a heading held at the name ceiling.
    AppMenuItem {
        id: remoteSentenceRow
        x: 1000
        y: 60
        width: 500
        refSentence: "%1 from %2"
        refName: "v1.0"
        refTint: Theme.textPrimary
        refMark: "tag"
        refName2: root.farName
        refMark2: "remote"
    }
    CardText {
        id: remoteHeading
        x: 900
        y: 260
        width: 860
        holdsNames: true
        text: "Replace " + root.farName + " with x?"
        marks: [[8, "remote", undefined, root.farName.length]]
    }
    // Names behind a narrow mark, which stands well inside its seat: a menu row's sentence and a heading's line.
    AppMenuItem {
        id: treeSentenceRow
        x: 1000
        y: 100
        width: 500
        refSentence: "%1 here"
        refName: "wt-j"
        refTint: Theme.textPrimary
        refMark: "tree"
    }
    CardText {
        id: treeHeading
        x: 1000
        y: 140
        width: 500
        holdsNames: true
        text: "Open wt-j here?"
        marks: [[5, "tree", undefined, 4]]
    }
    // A list row's line: words and names as code, cut at the end — under the column of chips, clear of it.
    CodeLine {
        id: codeLine
        x: 20
        y: 400
        width: 160
        height: Theme.rowHeight
        text: "Deleted feature/login here and on origin"
        marks: [[8, "", undefined, 13], [34, "", undefined, 6]]
    }
    // A remote's name cut at its end: the `…` is the last ink the ground holds.
    CodeChip {
        id: cutEndChip
        x: 1500
        y: 340
        width: 120
        cutToWidth: true
        cutAt: "end"
        mark: "remote"
        word: "origin/feature-with-a-long-name_"
    }
    // The run a tab draws after its repository's name (and the panel's name line, and a menu row's trail).
    TabTreeMark {
        id: tabRun
        x: 1300
        y: 340
        gap: Theme.spaceXs
        name: "topic"
    }
    // A pane's line: names behind their marks give way in the middle, the longer first, never the words.
    CodeLine {
        id: sharedLine
        x: 300
        y: 330
        width: 560
        height: Theme.rowHeight
        cutAt: "middle"
        color: Theme.textSecondary
        text: "Deleted on " + root.farName + ", changed on main"
        marks: [[11, "branch", undefined, root.farName.length],
                [11 + root.farName.length + 13, "branch", Theme.textLink, 4]]
    }
    // The code face, for a name's advance.
    TextMetrics {
        id: glyphs
        font.family: Theme.monoFamily
    }

    // A column of names: behind a mark at the row's end (two kinds), and opening a sentence. No chips, so the words
    // of every row start on one x.
    AppMenuItem {
        id: branchRow
        y: 60
        width: 900
        nameMark: "branch"
        codeName: "main"
    }
    AppMenuItem {
        id: remoteRow
        y: 90
        width: 900
        nameMark: "remote"
        codeName: "origin/main"
    }
    AppMenuItem {
        id: sentenceRow
        y: 120
        width: 900
        refSentence: "%1 from %2"
        refName: "v1.0"
        refTint: Theme.textPrimary
        refMark: "tag"
        refName2: "origin"
        refMark2: "remote"
    }
    // The same column with a name held at its ceiling and the row wide enough to spare room past it.
    AppMenuItem {
        id: heldRow
        y: 150
        width: 1700
        nameMark: "remote"
        codeName: root.farName
    }

    // The sentence row's words as rich text lays them out, for where its first name really starts — clear of the
    // chips, whose ink is read off the picture.
    TextEdit {
        id: drawnSentence
        y: 360
        textFormat: TextEdit.RichText
        font.family: Theme.uiFamily
        font.pixelSize: sentenceRow.font.pixelSize
        text: sentenceRow.refWords
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
    /// Where a row's code name's word starts, in the row's x.
    function codeNameStart(row) {
        const named = root.find(row, item => item.word === row.codeName && item.markRoom !== undefined)
        return named.mapToItem(row, named.markRoom, 0).x
    }
    /// Where a sentence row's first name starts, in the row's x.
    function sentenceNameStart(row) {
        const words = root.find(row, item => item.text === row.refWords && item.textFormat === Text.RichText)
        return words.mapToItem(row, row.refLead1, 0).x
    }
    /// Where the middle of a row's `kind` mark's ink stands, in the row's x.
    function markAxis(row, kind) {
        const mark = root.find(row, item => item.inkLeft !== undefined && item.visible && item.kind === kind)
        return mark.mapToItem(row, mark.inkLeft + mark.inkWidth / 2, 0).x
    }
    function groundOf(item) {
        return root.find(item, child => child.radius !== undefined && child.size !== undefined && child.visible)
    }
    /// Where `item`'s ground starts against `item`'s own head.
    function groundStart(item) {
        return root.groundOf(item).mapToItem(item, 0, 0).x
    }
    /// Where a name's ink ends as drawn, in `item`'s x, read off `picture` — outside every metric the code reads: back
    /// from `from` (an x on the ground past the ink) along the `band` of rows the name stands in, to the last column
    /// whose brightest pixel stands over the ground, that column counted by how much of it the ink covers. The ground's
    /// own edges are not read off it: offscreen, a rectangle is drawn out to whole pixels
    /// (`QSGSoftwareInternalRectangleNode::setRect`), which a window does not do.
    function drawnInkEnd(picture, item, from, band) {
        const at = item.mapToItem(root, from, band.y)
        const top = Math.round(at.y)
        const bottom = Math.round(at.y + band.height)
        const columns = []
        for (let x = Math.floor(at.x); x > Math.floor(at.x) - 16; x--) {
            let brightest = 0
            for (let y = top; y < bottom; y++)
                brightest = Math.max(brightest, picture.red(x, y))
            columns.push([x, brightest])
        }
        const ground = columns[0][1]
        const peak = Math.max(...columns.map(column => column[1]))
        for (const [x, brightest] of columns) {
            if (brightest > ground + 2)
                return item.mapFromItem(root, x + (brightest - ground) / (peak - ground), 0).x
        }
        return NaN
    }
    /// The air a chip's ground holds before its mark's ink (the mark's ink box, which `tst_iconink` holds to its
    /// pixels) and after its name's drawn ink.
    function airOf(chip, picture) {
        const ground = root.groundOf(chip)
        const mark = root.find(chip, item => item.inkLeft !== undefined && item.visible)
        const word = root.find(chip, item => item.text === chip.word)
        const right = ground.x + ground.width
        return { before: mark.x + mark.inkLeft - ground.x,
                 after: right - root.drawnInkEnd(picture, chip, right - 1, { y: word.y, height: word.height }) }
    }
    /// The same for a sentence row's first name.
    function sentenceAir(row, picture) {
        const ground = root.groundOf(row)
        const mark = root.find(row, item => item.inkLeft !== undefined && item.visible && item.kind === row.refMark)
        const from = ground.mapToItem(row, 0, 0).x
        const right = from + ground.width
        return { before: mark.mapToItem(row, mark.inkLeft, 0).x - from,
                 after: right - root.drawnInkEnd(picture, row, right - 1, { y: 0, height: row.height }) }
    }
    /// The same for a `NameRun`.
    function runAir(run, picture) {
        const ground = root.groundOf(run)
        const mark = root.find(run, item => item.inkLeft !== undefined)
        const right = ground.x + ground.width
        return { before: mark.x + mark.inkLeft - ground.x,
                 after: right - root.drawnInkEnd(picture, run, right - 1, { y: 0, height: run.height }) }
    }

    TestCase {
        name: "CodeNames"
        when: windowShown

        /// The ground holds the ink: the seat's own air before the mark's (`MarkSeat.air` at least, more before a mark
        /// narrower than the widest) and half a gap after the name's last glyph as drawn.
        function holdsInk(air, size, what) {
            const seat = MarkSeat.air * size
            verify(air.before >= seat - 1,
                   what + ": the seat's air before the mark, " + seat + " at least: " + air.before)
            fuzzyCompare(air.after, Theme.spaceXs / 2, 0.4, what + ": half a gap after the name")
        }

        /// The ground's start against the seat and its end against the ink. Half a gap before the seat, the ground adds
        /// air on top of the seat's own; off the mark's ink, a narrow mark's chip starts further in than a wide one's —
        /// chips opening two lines stand apart; off the name's advance, a `t` or a `j` ends with more after it than an
        /// `n`; off the glyph's box at the drawn size, which FreeType rounds out to whole pixels, most letters end with
        /// most of a pixel more.
        function test_a_chips_ground_starts_off_its_seat_and_ends_off_its_ink() {
            tryVerify(() => chip.width > 0 && kinds.count > 0 && kinds.itemAt(kinds.count - 1).width > 0)
            const picture = grabImage(root)
            holdsInk(root.airOf(chip, picture), chip.size, chip.word)
            fuzzyCompare(root.groundStart(commandChip), -Theme.spaceXs / 2, 0.01, "a command's: half a gap before it")
            for (let i = 0; i < kinds.count; i++) {
                const each = kinds.itemAt(i)
                holdsInk(root.airOf(each, picture), each.size, each.mark + " " + each.word)
                fuzzyCompare(root.groundStart(each), 0, 0.01, each.mark + ": at the seat's head")
            }
        }

        /// A name cut at its end ends its ground off the `…` it draws last, not off the letter it cut away.
        function test_a_chip_cut_at_its_end_ends_its_ground_off_the_cut() {
            tryVerify(() => cutEndChip.cut, undefined, "cut")
            holdsInk(root.airOf(cutEndChip, grabImage(root)), cutEndChip.size, "cut at the end")
        }

        /// A column's chips start on one x, whatever the mark in the middle of the seat and however the name is put
        /// in the row, and each holds its ink.
        function test_a_menus_column_of_chips_starts_on_one_x() {
            tryVerify(() => branchRow.width > 0 && sentenceRow.refLead1 > 0 && drawnSentence.text !== "")
            const picture = grabImage(root)
            const start = root.groundStart(branchRow)
            for (const row of [branchRow, remoteRow]) {
                const named = root.find(row, item => item.word === row.codeName && item.markRoom !== undefined)
                holdsInk(root.airOf(named, picture), row.font.pixelSize, row.nameMark)
                fuzzyCompare(root.groundStart(row), start, 0.5, row.nameMark + ": on one x")
            }
            holdsInk(root.sentenceAir(sentenceRow, picture), sentenceRow.font.pixelSize, "a name opening a sentence")
            fuzzyCompare(root.groundStart(sentenceRow), start, 0.5, "a name opening a sentence: on one x")
        }

        /// A name behind its mark grounds from the head of the mark's seat, which holds the mark's air — in a menu
        /// row's sentence and in a line of words as in a chip — never off the mark's ink, which a narrow mark stands
        /// further in from. To half a pixel: the mark stands on a whole one.
        function test_a_sentence_and_a_line_ground_a_name_from_its_marks_seat() {
            tryVerify(() => treeSentenceRow.refLead1 > 0 && treeHeading.codeGrounds.length > 0)
            const markOf = item => root.find(item, child => child.inkLeft !== undefined && child.visible
                                                         && child.kind === "tree")
            // Read off the ink, the ground would start half a gap before it: a pixel or more past the seat's head.
            const sentenceMark = markOf(treeSentenceRow)
            verify(sentenceMark.inkLeft >= Theme.spaceXs / 2 + 1, "a narrow mark, well inside its seat")
            fuzzyCompare(root.groundOf(treeSentenceRow).x, sentenceMark.x, 0.5, "a menu row's sentence")
            const lineMark = markOf(treeHeading)
            verify(lineMark.inkLeft >= Theme.spaceXs / 2 + 1, "a narrow mark, well inside its seat")
            fuzzyCompare(treeHeading.codeGrounds[0].x, lineMark.x, 0.5, "a line of words")
        }

        function test_a_worktree_run_is_code_behind_its_kinds_colour() {
            tryVerify(() => treeRun.width > 0 && homeRun.width > 0)
            const picture = grabImage(root)
            for (const run of [treeRun, homeRun]) {
                holdsInk(root.runAir(run, picture), run.pixelSize, run.kind)
                fuzzyCompare(root.groundStart(run), 0, 0.01, run.kind + ": the ground from the seat's head")
                const mark = root.find(run, item => item.inkLeft !== undefined)
                verify(Qt.colorEqual(mark.tint, Theme.success), run.kind + ": the mark in its kind's colour")
                const cut = root.find(run, item => item.headText !== undefined)
                const label = root.find(cut, item => item.text === run.name)
                compare(label.font.family, Theme.monoFamily, run.kind + ": the name in the code face")
                verify(Qt.colorEqual(label.color, Theme.textPrimary), run.kind + ": the name white")
            }
        }

        /// A name is white whatever its line's colour, but a step down in a line that is itself a step down (a
        /// heading's second line), its own colour where it says where the reader is, and the line's where that cannot
        /// be pressed.
        function test_a_name_takes_a_step_down_in_a_second_line() {
            verify(Qt.colorEqual(Words.nameInk(Theme.textPrimary), Theme.textPrimary), "plain words")
            verify(Qt.colorEqual(Words.nameInk(Theme.warning), Theme.textPrimary), "a warning's heading")
            verify(Qt.colorEqual(Words.nameInk(Theme.textSecondary), Theme.textSecondary), "a second line")
            verify(Qt.colorEqual(Words.nameInk(Theme.textSecondary, Theme.textLink), Theme.textLink),
                   "the branch stood on, in a second line too")
            verify(Qt.colorEqual(Words.nameInk(Theme.textMuted, Theme.textLink), Theme.textMuted),
                   "a line not pressable")
        }

        /// A list row's line never wraps: words and names side by side, the names as code, the piece the column's edge
        /// falls in cut there and nothing drawn past it; with room, all of it.
        function test_a_code_line_is_cut_at_the_end_and_never_wrapped() {
            compare(codeLine.pieces.map(piece => [piece.text, piece.code]),
                    [["Deleted ", false], ["feature/login", true], [" here and on ", false], ["origin", true]])
            const chipOf = word => root.find(codeLine, item => item.word === word && item.cutAt !== undefined)
            tryVerify(() => chipOf("feature/login").width > 0)
            verify(codeLine.cutting, "160 pixels do not hold it")
            const drawn = root.find(codeLine, item => item.headText !== undefined && item.text === " here and on ")
            const pieceOf = item => item.parent
            verify(pieceOf(drawn).x + pieceOf(drawn).width <= codeLine.width + 0.5, "nothing past the edge")
            verify(!chipOf("origin").visible, "the name past the edge is not drawn")
            compare(chipOf("feature/login").tint, Theme.textPrimary, "the name as code, plain")

            codeLine.width = 600
            tryVerify(() => !codeLine.cutting)
            verify(chipOf("origin").visible, "with room, the whole line")
            codeLine.width = 160
        }

        /// The worktree after a repository's name is a step down — its name, not its mark (デザイン規約 §タブの所作).
        function test_the_tab_run_steps_its_name_down_not_its_mark() {
            const run = root.find(tabRun, item => item.markCell !== undefined && item.nameWidth !== undefined)
            verify(Qt.colorEqual(run.color, Theme.textSecondary), "the name a step down")
            const mark = root.find(run, item => item.kindTint !== undefined)
            verify(Qt.colorEqual(mark.tint, mark.kindTint), "the mark in its kind's colour")
        }

        /// A pane's line (`middle`) keeps every word whole and gives way in its names' middles — the longer first, down
        /// to an even share — each behind its mark, in the line's step down or its own; its hover gives the line back.
        function test_a_code_line_gives_way_in_its_names() {
            const chipOf = word => root.find(sharedLine, item => item.word === word && item.cutAt !== undefined)
            const wordsOf = text => root.find(sharedLine, item => item.headText !== undefined && item.text === text)
            tryVerify(() => chipOf("main") !== null && chipOf("main").width > 0)
            verify(sharedLine.cutting, "560 pixels do not hold it")
            for (const text of ["Deleted on ", ", changed on "]) {
                const piece = wordsOf(text).parent
                compare(piece.width, piece.natural, "the words whole: " + text)
            }
            const far = chipOf(root.farName)
            verify(far.width < far.wantWidth, "the long name gives way")
            compare(far.cutAt, "middle")
            fuzzyCompare(wordsOf(", changed on ").parent.x, far.parent.x + far.drawnWidth, 0.5,
                         "the words after the cut name close up to what it draws")
            compare(chipOf("main").width, chipOf("main").wantWidth, "the short one whole")
            const last = chipOf("main").parent
            verify(last.x + last.width <= sharedLine.width + 0.5, "the line ends in the column")
            verify(far.markRoom > 0 && chipOf("main").markRoom > 0, "each behind its mark")
            verify(Qt.colorEqual(far.tint, Theme.textSecondary), "a step down with the line")
            verify(Qt.colorEqual(chipOf("main").tint, Theme.textLink), "the branch stood on, in its own")
            compare(sharedLine.tip.words, sharedLine.text)
            compare(sharedLine.tipMarks, sharedLine.marks)

            const both = "Deleted on " + root.farName + ", changed on " + root.farName + "-too"
            sharedLine.text = both
            sharedLine.marks = [[11, "branch", undefined, root.farName.length],
                                [11 + root.farName.length + 13, "branch", undefined, root.farName.length + 4]]
            tryVerify(() => chipOf(root.farName + "-too") !== null && chipOf(root.farName + "-too").width > 0)
            fuzzyCompare(chipOf(root.farName).width, chipOf(root.farName + "-too").width, 0.5, "an even share")
        }

        /// A remote's name gives way at its end, its head — the remote — kept (デザイン規約 §寸法「upstream 名だけは
        /// 末尾で切る」): in a menu's sentence, at a row's end, in a heading held at the ceiling. A branch's, in the
        /// middle.
        function test_a_remotes_name_gives_way_at_its_end() {
            tryVerify(() => remoteSentenceRow.refCut)
            const sentence = remoteSentenceRow.refShown2
            verify(sentence.endsWith("…") && root.farName.startsWith(sentence.slice(0, -1)), sentence)

            const held = remoteHeading.held
            const heading = held.text.substr(held.marks[0][0], held.marks[0][3])
            verify(heading.endsWith("…") && root.farName.startsWith(heading.slice(0, -1)), heading)

            const chipIn = row => root.find(row, item => item.word === row.codeName && item.cutAt !== undefined)
            compare(chipIn(heldRow).cutAt, "end", "a remote's name ending a row")
            compare(chipIn(branchRow).cutAt, "middle", "a branch's")
        }

        /// The side that is the branch stood on, in the current branch's colour; the other plain.
        function test_the_legend_says_the_branch_stood_on_in_its_own_colour() {
            tryVerify(() => legend.width > 0)
            const ours = root.find(legend, item => item.word === "main" && item.mark !== undefined)
            const theirs = root.find(legend, item => item.word === "topic" && item.mark !== undefined)
            verify(Qt.colorEqual(ours.tint, Theme.textLink), "ours is stood on")
            verify(Qt.colorEqual(theirs.tint, Theme.textSecondary), "theirs is not: a step down with the legend")
        }

        function test_a_menus_names_start_on_one_x() {
            tryVerify(() => branchRow.width > 0 && sentenceRow.refLead1 > 0)
            const branch = root.codeNameStart(branchRow)
            fuzzyCompare(root.codeNameStart(remoteRow), branch, 0.5, "a cloud's name and a branch's")
            fuzzyCompare(root.sentenceNameStart(sentenceRow), branch, 0.5, "a name opening a sentence")
            // The marks on one upright axis, whatever their width — to a pixel: each stands on a whole one, so two ink
            // middles can each be half a pixel off the cell's, either way.
            const axis = root.markAxis(branchRow, "branch")
            fuzzyCompare(root.markAxis(remoteRow, "remote"), axis, 1, "a cloud's mark and a branch's")
            fuzzyCompare(root.markAxis(sentenceRow, "tag"), axis, 1, "the mark of a name opening a sentence")
        }

        /// The lead the row measures is where rich text puts the name: past the room, as the markup lays it.
        function test_the_lead_is_where_the_name_is_drawn() {
            tryVerify(() => sentenceRow.refLead1 > 0 && drawnSentence.text !== "")
            const at = sentenceRow.refSentence.indexOf("%1") + sentenceRow.refMarkRoom
            fuzzyCompare(drawnSentence.positionToRectangle(at).x, sentenceRow.refLead1, 0.5)
        }

        function test_a_name_at_its_ceiling_keeps_its_place() {
            tryVerify(() => heldRow.width > 0 && remoteRow.width > 0)
            verify(heldRow.nameCut, "held at the ceiling")
            fuzzyCompare(root.codeNameStart(heldRow), root.codeNameStart(remoteRow), 0.5,
                         "the spare room is not handed out in front of the name")
        }
    }
}
