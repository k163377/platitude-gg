import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A stage past the next one its operation declared is refused: the operation skipped one, and the run ends there.
Item {
    width: 100
    height: 100

    StageRig {
        id: rig
    }
    Automation.PageStage {
        id: op
        name: "skips"
        order: ["first", "second"]
        onTriggered: {
            if (!Automation.Awaited.all("skips_ready", { "ready": true }))
                return
            op.advance("second")
        }
    }

    TestCase {
        name: "StageSkip"
        when: windowShown

        function test_a_stage_past_the_next_declared_is_refused() {
            op.start()
            tryVerify(() => rig.route.read)
            compare(Stages.chain(Harness.said, "page", "skips", 1), [])
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=skipped first on its way to second"), breaches[0])
        }
    }
}
