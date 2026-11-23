import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The corner's line short of room (`BuildLine`): the word `git`, then the build's commit down to its first three
// digits, then the git's version down to its major — each only once the one before it is at its least — and the line
// laid out inside its room down to that least, each cut sized to the head it keeps and its mark. The builds as the
// corner names them, set by hand; where the room comes from is the right pane's (`corner`).
//
// A control, to hand the line the window's font as `Main` does: the pieces are priced in the faces they draw in.
Control {
    id: root
    width: 600
    height: 60
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    BuildLine {
        id: line
        name: ""
        hash: ""
        version: ""
        room: root.width
    }

    TestCase {
        name: "BuildLine"
        when: windowShown

        /// A line is priced again before its fields hold a new value, and Qt warns at every place a field is asked for
        /// that it does not have (`LineText.edgeAt`).
        function init() {
            failOnWarning(/QTextCursor::setPosition/)
        }
        /// The two fields, ahead of the rulers among the line's children.
        function nameField() {
            return line.children[0]
        }
        function gitField() {
            return line.children[1]
        }
        /// Waits for the layout to hand the fields the widths the line priced and to set them end to end, with no gap
        /// between them a press could land in: until then either can stand where the last room put it, and a check of
        /// the width would read a line not yet laid out.
        function laid(note) {
            tryVerify(() => Math.abs(nameField().width - line.nameWidth) < 0.5
                            && Math.abs(gitField().width - line.gitWidth) < 0.5
                            && Math.abs(gitField().x - nameField().width) < 0.5
                            && Math.abs(line.width - (gitField().x + gitField().width)) < 0.5, undefined,
                      note + ": the line never took the widths it priced")
        }
        function build(name, hash, version) {
            line.room = root.width
            line.name = name
            line.hash = hash
            line.version = version
            tryVerify(() => line.nameWhole > 0 && line.gitWhole > 0)
        }
        /// How many of the version's characters the second field shows before its mark, read off where the field
        /// drew the mark (`LineText.markAt`).
        function versionKept() {
            const field = gitField()
            const before = field.text.length - line.version.length
            let kept = 0
            while (kept < line.version.length && field.edgeAt(before + kept + 1) <= field.markAt + 0.5)
                kept++
            return kept
        }
        /// The widths the line priced at each room, swept down a pixel at a time from the whole line: a room reached
        /// in one step has to price the same.
        function sweptWidths() {
            const widths = {}
            for (let room = Math.ceil(line.nameWhole + line.gitWordedWhole) + 2; room >= Math.ceil(line.leastWidth);
                 room--) {
                line.room = room
                laid("swept at " + room)
                widths[room] = [line.nameWidth, line.gitWidth]
            }
            return widths
        }

        /// Sweeps the room from the whole line down to its least, a pixel at a time: each step gives only once the one
        /// before it is at its least, nothing comes back as the room shrinks, the line stays inside its room, and a
        /// cut leaves no ground between its mark and what follows. Answers the steps seen, in order.
        function sweep(what) {
            const seen = []
            let nameWas = line.nameWhole
            let gitWas = line.gitWordedWhole
            let worded = true
            const top = Math.ceil(line.nameWhole + line.gitWordedWhole) + 2
            for (let room = top; room >= Math.ceil(line.leastWidth); room--) {
                line.room = room
                const note = what + " at " + room
                verify(line.fits, note + ": the line does not fit over its least")
                verify(!line.worded || worded, note + ": the word came back")
                worded = line.worded
                verify(line.nameWidth <= nameWas + 0.5, note + ": the commit came back")
                verify(line.gitWidth <= gitWas + 0.5, note + ": the version came back")
                nameWas = line.nameWidth
                gitWas = line.gitWidth
                if (line.nameWidth < line.nameWhole - 0.5)
                    verify(!line.worded, note + ": the commit cut beside the word")
                if (!line.worded && line.gitWidth < line.gitWhole - 0.5)
                    verify(Math.abs(line.nameWidth - line.nameLeast) < 0.5,
                           note + ": the version cut while the commit stood over its least")
                laid(note)
                verify(line.width <= line.room + 0.5,
                       note + ": the line ran " + (line.width - line.room) + "px past its room")
                for (const field of [nameField(), gitField()]) {
                    if (field.clipped)
                        verify(field.width - field.markAt - field.markWidth < 1,
                               note + ": " + (field.width - field.markAt - field.markWidth)
                               + "px of ground after the mark")
                }
                // A value is cut once the commit or the version gives; the word goes whole.
                compare(line.cut, line.gives === "hash" || line.gives === "version", note + ": the tip's reason")
                if (gitField().clipped) {
                    const kept = versionKept()
                    verify(line.separators.indexOf(line.version.charAt(kept - 1)) < 0,
                           note + ": the version cut just past a separator (" + line.version.substring(0, kept) + ")")
                }
                if (seen.length === 0 || seen[seen.length - 1] !== line.gives)
                    seen.push(line.gives)
            }
            line.room = Math.floor(line.leastWidth) - 1
            verify(!line.fits, what + ": the line says it fits under its least")
            // The copy stays whole whatever the room cut (規約 §右のペインの字は掴める), and so do the tip's words.
            compare(nameField().text, line.name)
            verify(gitField().text.endsWith(line.version), "the second field holds the whole version")
            compare(line.wholeTip.words, line.name + "/git " + line.version)
            return seen
        }

        // A worktree's build on Windows, whose git names its platform: every step, in order.
        function test_a_a_seat_build_gives_every_step_in_order() {
            build("v0.0.0-a-03f8bc64", "03f8bc64", "2.55.0.windows.3")
            compare(sweep("seat build").join(","), "none,word,hash,version")
        }

        // At its least the commit keeps its first three digits and the version its major, each with the mark.
        function test_b_the_least_keeps_three_digits_and_the_major() {
            build("v0.0.0-a-03f8bc64", "03f8bc64", "2.55.0.windows.3")
            line.room = Math.ceil(line.leastWidth)
            laid("least")
            compare(line.nameWidth, line.cutWidth(nameField(), "v0.0.0-a-03f".length))
            compare(versionKept(), "2".length)
            verify(nameField().clipped && gitField().clipped, "both fields are cut at the least")
        }

        // A hand over a line cut asks for the whole line in its tip, with the word the line gave up; off it, nothing.
        function test_e_a_cut_line_asks_for_its_tip_on_the_hover() {
            build("v0.0.0-a-03f8bc64", "03f8bc64", "2.55.0.windows.3")
            line.room = Math.ceil(line.leastWidth)
            laid("least")
            mouseMove(line, 4, line.height / 2)
            tryVerify(() => line.wholeTip.asked, undefined, "the cut line never asked for its tip")
            compare(line.wholeTip.words, "v0.0.0-a-03f8bc64/git 2.55.0.windows.3")
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !line.wholeTip.asked, undefined, "the tip stood past the hand")
        }

        // The room reached in one step — a window put back, a fast drag — prices the line as the sweep does: from
        // the whole line with the word straight into the version's cut, and back.
        function test_f_a_room_reached_in_one_step_prices_as_the_sweep() {
            build("v0.0.0-a-03f8bc64", "03f8bc64", "2.55.0.windows.3")
            const swept = sweptWidths()
            const top = Math.ceil(line.nameWhole + line.gitWordedWhole) + 2
            for (let room = Math.ceil(line.leastWidth); room < Math.ceil(line.nameWhole + line.gitWhole); room++) {
                line.room = top
                laid("top before " + room)
                line.room = room
                laid("jumped to " + room)
                compare(line.nameWidth, swept[room][0], "the commit at " + room + ", reached in one step")
                compare(line.gitWidth, swept[room][1], "the version at " + room + ", reached in one step")
            }
        }

        // The version comes after the line stands (git is asked once the window is built), on a room the line is
        // short of: the line is priced again before its field holds the new version, asks only places the field has,
        // and settles on what the same room prices with the version in place.
        function test_g_a_version_that_comes_late_asks_only_places_the_field_has() {
            build("v0.0.0-a-03f8bc64", "03f8bc64", "2.55.0.windows.3")
            const room = Math.ceil(line.leastWidth)
            line.room = room
            laid("before")
            const settled = [line.nameWidth, line.gitWidth]
            line.version = ""
            line.version = "2.55.0.windows.3"
            laid("late version")
            compare([line.nameWidth, line.gitWidth], settled, "the line priced against a field without the version")
        }

        // A release has no commit in its name: the word goes, then the version, the name whole throughout.
        function test_c_a_release_keeps_its_name_whole() {
            build("PGG-v0.1.0", "", "2.55.0.windows.3")
            compare(sweep("release").join(","), "none,word,version")
            compare(line.nameLeast, line.nameWhole)
        }

        // A version with spaces and brackets in it, one that names git itself: the word goes as the label it is, the
        // version's own `Git` stays, and the version gives at its separators like any other.
        function test_d_a_version_naming_git_gives_at_its_separators() {
            build("v0.0.0-03f8bc64", "03f8bc64", "2.39.5 (Apple Git-154)")
            line.room = Math.ceil(line.nameWhole + line.gitWhole)
            compare(line.gives, "word")
            laid("apple git")
            compare(gitField().text, "/2.39.5 (Apple Git-154)")
            verify(!gitField().clipped && !line.cut, "the version stands whole once the word is gone")
            sweep("apple git")
        }
    }
}
