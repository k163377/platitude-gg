import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A second input as a stage the generation already reached is refused before it reaches the product, said as the
// breach it is, and the run ends there (`StageTimer.input`).
Item {
    width: 100
    height: 100

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "twice"
        property int presses: 0
        onTriggered: {
            if (!Automation.Awaited.all("twice_ready", { "ready": true }))
                return
            op.input("pressed", () => { op.presses = op.presses + 1 })
            op.input("pressed", () => { op.presses = op.presses + 1 })
        }
    }

    TestCase {
        name: "StageTwice"
        when: windowShown

        function test_a_second_input_as_the_same_stage_is_refused_before_it_is_pressed() {
            op.start()
            tryVerify(() => rig.route.read)
            compare(op.presses, 1)
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=reached pressed twice in one generation"), breaches[0])
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(Stages.field(completed[0], "op"), "none")
        }
    }
}
