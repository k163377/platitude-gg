import QtQuick
import QtTest
import platitude.ui

// One line of words and code names (`CodeLine`) as a list builds it and as its column cuts it: built in the background
// a time slice at a go (a list's row past its edge, `cacheBuffer`), its pieces still take their widths once they are
// built; cut at the end inside a name, nothing after that name is drawn.
Item {
    id: root
    width: 800
    height: 120

    /// Spends more wall clock than one time slice the window gives the builder of its background items, from inside a
    /// binding: the build of whatever holds that binding stops on it (`tst_ink`).
    function spend() {
        const until = Date.now() + 60
        while (Date.now() < until) {}
        return 1
    }

    Component {
        id: lateLine
        Item {
            property int spent: root.spend()
            property alias line: line
            width: 600
            height: 30
            CodeLine {
                id: line
                width: 600
                height: 30
                text: "Deleted feature/login here and on origin"
                marks: [[8, "", undefined, 13], [34, "", undefined, 6]]
            }
        }
    }
    // The column's edge falls inside the first name.
    CodeLine {
        id: cutInName
        y: 60
        width: 90
        height: 30
        text: "Deleted feature/login-with-a-longer-name here and on origin"
        marks: [[8, "", undefined, 32], [53, "", undefined, 6]]
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
        name: "CodeLine"
        when: windowShown

        function test_a_line_built_in_the_background_sizes_its_pieces() {
            const incubator = lateLine.incubateObject(root, {}, Qt.Asynchronous)
            tryVerify(() => incubator.status === Component.Ready)
            const line = incubator.object.line
            const chipOf = word => root.find(line, item => item.word === word && item.cutAt !== undefined)
            tryVerify(() => chipOf("origin") !== null && chipOf("origin").width > 0, undefined,
                      "the last name is drawn once its piece is built")
            verify(!line.cutting, "600 pixels hold the line")
            incubator.object.destroy()
        }

        function test_a_name_cut_at_the_end_has_nothing_drawn_after_it() {
            const chip = root.find(cutInName, item => item.word === "feature/login-with-a-longer-name"
                                                      && item.cutAt !== undefined)
            tryVerify(() => chip.width > 0 && chip.cut, undefined, "the edge falls in the name")
            const after = root.find(cutInName, item => item.headText !== undefined && item.text === " here and on ")
            verify(!after.visible, "nothing after the cut name")
            verify(!root.find(cutInName, item => item.word === "origin" && item.cutAt !== undefined).visible)
        }
    }
}
