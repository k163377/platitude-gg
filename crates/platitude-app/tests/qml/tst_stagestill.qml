import QtQuick
import QtQuick.Layouts
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A layout wait stands on a polish pass of the window begun after what it saw — never on a frame said swapped, which
// on a threaded render loop can be one begun before. In a window that cannot render no pass runs: the wait holds, and
// the input it guards and the witness it takes are held with it; once the window shows, a pass lays the seat out and
// the input goes in on the seat as laid out. A seat the layout leaves with no width stands too (`StageTimer.still`,
// `StageTimer.laidOutSince`, `Awaited.polishes`).
Item {
    id: root

    width: 100
    height: 100

    /// The seat's width as the press read it, -1 before any press.
    property real pressedAt: -1
    property bool asked: false
    property int beats: 0

    Window {
        id: stage
        width: 400
        height: 300
        visible: false

        RowLayout {
            anchors.fill: parent
            spacing: 0
            Item {
                id: seat
                Layout.fillHeight: true
                Layout.preferredWidth: root.asked ? 120 : 10
            }
            Item {
                id: none
                Layout.fillHeight: true
                Layout.preferredWidth: 0
                Layout.maximumWidth: 0
            }
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
        }
    }

    StageRig {
        id: rig
    }
    // The window's frames counted as the harness counts them (`WindowHarness`).
    Connections {
        target: stage
        function onFrameSwapped() {
            Automation.Awaited.frames = Automation.Awaited.frames + 1
        }
    }
    Automation.PageStage {
        id: op
        name: "still"
        onTriggered: {
            if (!op.at("start"))
                return
            if (!op.still("seat", seat.width + "x" + seat.height) || !op.still("none", none.width)
                    || !op.laidOutSince("asked"))
                return
            op.witness("laid", seat.width)
            op.input("pressed", () => {
                root.pressedAt = seat.width
                return true
            })
        }
    }
    /// Beats while the operation waits: what "held this long" is counted on.
    Automation.PageStage {
        id: clock
        name: "clock"
        running: true
        onTriggered: root.beats = root.beats + 1
    }

    TestCase {
        name: "StageStill"
        when: windowShown

        function test_a_layout_wait_holds_until_a_pass_begun_after_it() {
            Automation.Awaited.window = stage
            root.asked = true
            const from = Automation.Awaited.polishes
            op.start()
            const started = root.beats
            tryVerify(() => root.beats >= started + 2)
            // Enough notices for each wait to have one counted past its own mark, the waits being asked in turn.
            for (let i = 0; i < 6; i++) {
                // A frame notice said after the change — a frame begun before it, its swap arriving late — is no pass.
                stage.frameSwapped()
                const at = root.beats
                tryVerify(() => root.beats >= at + 2)
            }
            compare(Automation.Awaited.polishes, from, "the window cannot render: no pass")
            compare(root.pressedAt, -1, "the input is held with the wait")
            verify(!Harness.said.some(line => line.startsWith("witness ") && line.includes("name=laid")),
                   "and the witness")
            stage.visible = true
            tryVerify(() => root.pressedAt >= 0)
            verify(Automation.Awaited.polishes > from)
            compare(root.pressedAt, 120, "pressed on the seat as the pass laid it out")
            compare(none.width, 0, "a seat laid out with no width stood too")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
