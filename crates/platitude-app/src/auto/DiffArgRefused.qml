import QtQuick
import platitude

/// The operation of a verb that opens a file's diff and was given no file: it says so and ends there — the line its
/// row asks for is never said, and the run fails on it. Its completion is its own: the verbs that open a file defer
/// theirs (`AutoActCompletion.defersCompletion`). Each family that opens one holds one, under a name of its own.
PageStage {
    id: refused

    onTriggered: refused.complete({ "when": () => ({}),
                                    "report": () => "diff_arg act=" + Harness.autoAct + " named=false" })
}
