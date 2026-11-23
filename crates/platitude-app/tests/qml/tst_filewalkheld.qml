import QtQuick
import QtTest
import platitude.ui

// A file list's arrow held down (`FileRowWalk.noteStep` / `keyUp`), the real walk over a list cut down to what it
// reads: a held run reads where it set off and where it stopped — each read is a `git diff` — and the key coming up or
// the keyboard leaving the lists says where. The settle behind a press stands in for neither, and a release with no
// run behind it reads nothing: every press comes up.
Item {
    id: root
    width: 200
    height: 200

    readonly property var names: ["a.rs", "b.rs", "c.rs", "d.rs", "e.rs"]
    ListView {
        id: list
        width: 200
        height: Theme.rowHeight * root.names.length
        model: root.names.length
        delegate: Item {
            width: list.width
            height: Theme.rowHeight
        }
    }
    /// Takes the keyboard off the list.
    Item {
        id: elsewhere
        y: list.height
        width: 10
        height: 10
    }
    /// The model's two slots the walk steps by (`NavSectionModel::step`), over `names` in one bucket.
    QtObject {
        id: files
        function stepFile(bucket, path, way) {
            const at = root.names.indexOf(path) + way
            return at >= 0 && at < root.names.length ? { "bucket": "", "path": root.names[at], "row": at } : null
        }
        function edgeFile(way) {
            return null
        }
        function origOf(path) {
            return ""
        }
    }
    Component {
        id: walkOf
        FileRowWalk {
            id: made
            /// The files read, in order.
            property var reads: []
            sides: [{ "view": list, "model": files }]
            readBucket: ""
            readPath: ""
            // The page reads what landed and hands it back as the file being read (`RepoPage.toggleDiff`).
            onLanded: (bucket, path, origPath) => {
                made.reads = made.reads.concat([path])
                made.readPath = path
            }
        }
    }

    TestCase {
        name: "FileWalkHeld"
        when: windowShown

        /// A walk of its own, reading the first file: a settle left running by another case would hold its press.
        function reading() {
            const walk = createTemporaryObject(walkOf, root)
            walk.readPath = root.names[0]
            return walk
        }

        function test_a_held_run_reads_where_it_set_off_and_where_the_key_came_up() {
            const walk = reading()
            verify(walk.stepFile(1, false))
            compare(walk.reads, ["b.rs"], "the press reads at once")
            verify(walk.stepFile(1, true))
            verify(walk.stepFile(1, true))
            // waits(paced): the settle twice over; a held run weighed against it would read the file it is passing.
            wait(Metrics.keyStepSettleMs * 2)
            compare(walk.reads, ["b.rs"], "the repeats read nothing, however long the key is down")
            walk.keyUp()
            compare(walk.reads, ["b.rs", "d.rs"], "the key coming up reads where the run stopped")
            // waits(paced): the settle behind the release, twice over; a second read there would be the same file's.
            wait(Metrics.keyStepSettleMs * 2)
            compare(walk.reads, ["b.rs", "d.rs"], "and nothing after it")
        }

        function test_the_keyboard_leaving_the_list_reads_where_a_held_run_stopped() {
            list.forceActiveFocus()
            const walk = reading()
            verify(walk.keysHere)
            verify(walk.stepFile(1, false))
            verify(walk.stepFile(1, true))
            elsewhere.forceActiveFocus()
            compare(walk.reads, ["b.rs", "c.rs"], "a key that comes up somewhere else still ends the run")
        }

        function test_a_release_with_no_run_behind_it_reads_nothing() {
            const walk = reading()
            verify(walk.stepFile(1, false))
            walk.keyUp()
            compare(walk.reads, ["b.rs"], "one press, one read")
        }
    }
}
