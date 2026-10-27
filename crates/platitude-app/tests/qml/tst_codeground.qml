import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A code ground holds the ink of the word it is under, over and under (デザイン規約 §git 用語のコード表記): the code
// face's tallest letters and `/` and its deepest descenders stand clear of the ground's top and foot — wherever a
// ground is laid (a chip, a chip a reader can drag over, a worktree's run, a menu's command and the name in a menu's
// sentence, a line of words, a toolbar button, a stopped operation's way out) and at each size and weight a word is set
// in there. Every one of them is a `CodeGround`; in a text of several lines it leaves half a gap to the next line's,
// and under a button's word it keeps a quarter of a gap.
Control {
    id: root
    width: 760
    height: 340
    // Dark, as the window's grounds are: the ink is read off the picture against it.
    background: Rectangle {
        color: Theme.bgBase
    }

    /// A name reaching the code face's extremes on each OS: `f` and `/` the tallest, `y`, `g` and `j` the deepest.
    readonly property string name: "fly/jog"
    /// How far the ink stands clear of the ground's top and foot at least, in pixels: an edge row the ink covers half
    /// of or more reads as ink on the edge.
    readonly property real clear: 0.5

    Column {
        x: 20
        y: 20
        spacing: Theme.spaceMd
        Repeater {
            id: chips
            model: [[Theme.fontMd, Font.Normal], [Theme.fontSm, Font.Normal], [Theme.fontMd, Theme.fontWeightStrong]]
            CodeChip {
                required property var modelData
                word: root.name
                mark: "branch"
                size: modelData[0]
                weight: modelData[1]
            }
        }
        // The version git printed, in the settings: a field a reader can drag over.
        CodeChip {
            id: grabbedChip
            word: "git 2.30.0"
            grabbable: true
        }
        // A tab's worktree run, and the panel's repository line.
        NameRun {
            id: tabRun
            name: root.name
            pixelSize: Theme.fontSm
        }
        NameRun {
            id: homeRun
            name: root.name
            kind: "home"
            weight: Theme.fontWeightStrong
        }
    }
    // A notice's detail: names that fall on three lines, one under another.
    CardText {
        id: smallLines
        x: 20
        y: 250
        width: 200
        holdsNames: true
        pixelSize: Theme.fontSm
        text: "Replace origin/feature/jolly-gypsy-fly with fly/jog and keep gypsy/jig?"
        marks: [[8, "remote", undefined, 30], [44, "branch", undefined, 7], [61, "branch", undefined, 9]]
    }
    AppMenuItem {
        id: commandRow
        x: 320
        y: 20
        width: 400
        code: "apply"
    }
    AppMenuItem {
        id: sentenceRow
        x: 320
        y: 60
        width: 400
        refSentence: "%1 here"
        refName: root.name
        refTint: Theme.textPrimary
        refMark: "branch"
    }
    CardText {
        id: line
        x: 320
        y: 110
        width: 400
        holdsNames: true
        text: "Open " + root.name + " here?"
        marks: [[5, "branch", undefined, root.name.length]]
    }
    ActionButton {
        id: pushButton
        x: 320
        y: 160
        text: "push"
        code: true
    }
    // The commit button's phrase: its command and its branch are chips on a button.
    ActionButton {
        id: phraseButton
        x: 410
        y: 160
        width: 320
        text: "files to"
        phraseHead: "commit"
        phraseCount: "2"
        phraseTail: root.name
        phraseTailMark: "branch"
    }
    // Lines 16 apart, as Yu Gothic UI sets them at 12px: the offscreen database does not load that face, so the pitch
    // is given.
    CodeGround {
        id: tightLine
        x: 600
        width: 40
        size: Theme.fontSm
        wordBaseline: 300
        linePitch: 16
    }
    // A row of a stopped operation's card.
    OpExitRow {
        id: exitRow
        x: 320
        y: 220
        width: 400
        code: "cherry-pick --skip"
        text: "Leave this commit out"
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
    /// What paints a code ground under `item`, `CodeGround` or not: rounded, in the ground's wash.
    function isGround(item) {
        return item.radius === Theme.radiusSm && Qt.colorEqual(item.color, Theme.bgHover) && item.visible
    }
    function groundOf(item) {
        return root.find(item, root.isGround)
    }
    function groundsOf(item) {
        const found = []
        for (const child of item.children) {
            if (root.isGround(child))
                found.push(child)
            found.push(...root.groundsOf(child))
        }
        return found
    }
    /// How far the ink over `ground` stands clear of its top (`over`) and its foot (`under`), in pixels. The ground's
    /// edges are read off its geometry — offscreen draws a rectangle out to whole pixels
    /// (`QSGSoftwareInternalRectangleNode::setRect`) — and the ink off `picture`: a row's ink is how far its
    /// brightest pixel over the ground's columns stands over its darkest, against the most any row has; the first and
    /// the last rows with ink count by how much of them it covers. Off the rounded corners; a row past either edge, for
    /// ink hanging out, and no further: the next line of a text stands there.
    function clearance(picture, ground) {
        const at = ground.mapToItem(root, 0, 0)
        const left = Math.ceil(at.x) + Theme.radiusSm
        const right = Math.floor(at.x + ground.width) - Theme.radiusSm
        const rows = []
        for (let y = Math.floor(at.y) - 1; y < Math.ceil(at.y + ground.height) + 1; y++) {
            let darkest = 255
            let brightest = 0
            for (let x = left; x < right; x++) {
                const value = Math.max(picture.red(x, y), picture.green(x, y), picture.blue(x, y))
                darkest = Math.min(darkest, value)
                brightest = Math.max(brightest, value)
            }
            rows.push({ y: y, lift: brightest - darkest })
        }
        const full = Math.max(...rows.map(row => row.lift))
        const inked = rows.filter(row => row.lift > full / 10)
        const first = inked[0]
        const last = inked[inked.length - 1]
        return { over: first.y + 1 - first.lift / full - at.y,
                 under: at.y + ground.height - (last.y + last.lift / full) }
    }

    TestCase {
        name: "CodeGround"
        when: windowShown

        function test_the_ground_holds_the_ink_over_and_under() {
            tryVerify(() => chips.count === 3 && chips.itemAt(2).width > 0 && grabbedChip.grabbed
                            && line.codeGrounds.length > 0 && smallLines.codeGrounds.length > 0
                            && pushButton.width > 0 && phraseButton.width > 0 && exitRow.width > 0)
            const picture = grabImage(root)
            const places = [[chips.itemAt(0), "a chip"], [chips.itemAt(1), "a small chip"],
                            [chips.itemAt(2), "a strong chip"], [grabbedChip, "a chip as a field"],
                            [tabRun, "a tab's run"], [homeRun, "a strong run"], [commandRow, "a menu's command"],
                            [sentenceRow, "a name in a menu's sentence"], [line, "a name in a line of words"],
                            [smallLines, "a name in small lines of words"], [pushButton, "a toolbar button"],
                            [phraseButton, "a button's phrase"], [exitRow, "a stopped operation's way out"]]
            for (const [place, what] of places) {
                const ground = root.groundOf(place)
                verify(ground.wordBaseline !== undefined, what + ": a CodeGround")
                const air = root.clearance(picture, ground)
                verify(air.over >= root.clear, what + ": clear of the ground's top, " + air.over.toFixed(2))
                verify(air.under >= root.clear, what + ": clear of the ground's foot, " + air.under.toFixed(2))
            }
        }

        /// A button's word stands on a ground whose top is a chip's and whose foot stands a quarter of a gap higher
        /// (`CodeGround.buttonWord`) — the button's own word and the chips of its phrase alike. Read against the word's
        /// baseline, at one size.
        function test_a_buttons_ground_keeps_a_quarter_gap_under_its_word() {
            tryVerify(() => pushButton.width > 0 && phraseButton.width > 0 && chips.count === 3
                            && chips.itemAt(0).width > 0)
            const reach = ground => {
                const baseline = ground.parent.mapToItem(root, 0, ground.wordBaseline).y
                const top = ground.mapToItem(root, 0, 0).y
                return { over: baseline - top, under: top + ground.height - baseline }
            }
            const chip = root.groundOf(chips.itemAt(0))
            for (const [button, what] of [[pushButton, "a button's word"], [phraseButton, "a button's phrase"]]) {
                const ground = root.groundOf(button)
                compare(ground.size, chip.size, what + ": one size")
                fuzzyCompare(reach(ground).over, reach(chip).over, 0.5, what + ": the top over the baseline")
                fuzzyCompare(reach(ground).under, reach(chip).under - Theme.spaceXs / 4, 0.5,
                             what + ": the foot under the baseline")
            }
        }

        /// Names that fall on lines one under another keep their grounds apart: none taller than the lines' pitch less
        /// half a gap, so half a gap stands between one line's ground and the next's — and where the lines stand as
        /// close as Yu Gothic UI sets them, too.
        function test_a_texts_lines_keep_their_grounds_apart() {
            tryVerify(() => smallLines.codeGrounds.length > 2 && smallLines.lineHeight > 0)
            const grounds = root.groundsOf(smallLines).sort((a, b) => a.y - b.y)
            verify(grounds.length > 2, "names on three lines")
            for (const ground of grounds) {
                verify(ground.height <= smallLines.lineHeight - Theme.spaceXs / 2,
                       "a ground " + ground.height + " tall in lines " + smallLines.lineHeight.toFixed(2) + " apart")
            }
            for (let i = 1; i < grounds.length; i++) {
                const above = grounds[i - 1]
                if (grounds[i].y - above.y < 1)
                    continue
                const gap = grounds[i].y - (above.y + above.height)
                verify(gap >= Theme.spaceXs / 2, "a line's ground " + gap.toFixed(2) + " from the one above")
            }
            verify(tightLine.height <= tightLine.linePitch - Theme.spaceXs / 2,
                   "a ground " + tightLine.height + " tall in lines " + tightLine.linePitch + " apart")
        }
    }
}
