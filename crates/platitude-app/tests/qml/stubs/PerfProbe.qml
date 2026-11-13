pragma Singleton

import QtQuick

// Stands in for the app's `PerfProbe` under `qmltestrunner` (staged like `GitFacts`): the harness's machinery reads
// its clock to say how long a wait has stood (`Awaited.note`) and to mark when a frame was asked for
// (`PerfFrameWait.askedAt`), neither of which any test judges.
QtObject {
    function clockMs() {
        return 0
    }
}
