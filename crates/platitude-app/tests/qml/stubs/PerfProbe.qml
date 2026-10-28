pragma Singleton

import QtQuick

// Stands in for the app's `PerfProbe` under `qmltestrunner` (staged like `GitFacts`): the harness's machinery reads
// its clock only to say how long a wait has stood (`Awaited.note`), which no test judges.
QtObject {
    function clockMs() {
        return 0
    }
}
