import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A press that throws may have put part of its input in: it ends the run as a breach, and the beat after does not
// press it again (`StageTimer.input`).
Item {
    id: root

    width: 100
    height: 100

    property int presses: 0

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "throws"
        onTriggered: op.input("pressed", () => {
            root.presses = root.presses + 1
            throw new Error("half the input")
        })
    }

    TestCase {
        name: "StageThrows"
        when: windowShown

        function test_a_press_that_throws_is_a_breach_and_is_not_pressed_again() {
            op.start()
            tryVerify(() => rig.route.read)
            compare(root.presses, 1)
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=a press that threw: Error: half the input"), breaches[0])
        }
    }
}
