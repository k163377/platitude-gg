pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The diff pane's presses run one after another until the file's side is empty: where the pane lands on the press
/// that empties it (`RepoPage.landAhead`). Each verb here opens its own file and owns its waits, so none is started
/// from `AutoActDiffVerbs.stageRowTimer`.
// `Item` because `QtObject` has no default property to hold the timers below.
Item {
    id: runs

    /// `var`: naming its type would be a cycle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var diffPane: driver.diffPane

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn.
    function run(act, arg) {
        if (act !== "hunk-run")
            return false
        page.showWip()
        hunkRunTimer.order = arg === "apart" ? ["opened", "pressed", "last"] : ["opened", "pressed"]
        hunkRunTimer.start()
        return true
    }
    /// The run's writes have been through and the pane stopped reading again (`RepoPage.diffSettling`).
    function wroteAndSettled() {
        return driver.wroteAndSettled() && !page.diffSettling
    }

    // hunk-run together|apart (`--preset manyhunks`: `notes.txt`, one file changed on one side alone, twenty hunks):
    // hunk after hunk staged off the heading's word (`RepoPage.stageSelection`) until the side is empty and there is
    // no neighbour to walk to. On the last press's own turn (`pressed_*`) the pane is on the file's staged side
    // (`pressed_diff=`, `RepoPage.landAhead`) and never on the emptied side's `No changes to show`
    // (`pressed_nothing=`); its marks wait (`pressed_open=`). `together`: every hunk pressed in one turn, before any
    // answer — the read on screen is still the whole change, so its rows go to the staged side as they are
    // (`pressed_same=`, against the rows the file opened with). `apart`: all but the last pressed in one turn, then the
    // last once git has answered them and the pane has read the file again — that read holds the last hunk alone, the
    // file is on both sides, and the pane waits blank (`pressed_blank=`, `pressed_waiting=`). Each press after the
    // first moves on from the one before without waiting it out, said out loud (`AutoActDriver.letWriteGo`). Either
    // way the run ends on the staged side's own read: the whole change, marks live (`landed=`, `same=`, `open=`).
    PageStage {
        id: hunkRunTimer
        name: "hunk_run"
        readonly property string file: "notes.txt"
        /// Presses hunk 0 of the drawing — each press takes it out, so the next one is hunk 0 again — while the pane
        /// stands on the file's unstaged side and takes a press; with `keepLast`, the press that would take the last of
        /// it is held back. Answers how many went in.
        function pressHunks(keepLast) {
            let went = 0
            while (page.diffKey === "unstaged:" + hunkRunTimer.file && page.diffPressable
                   && !(keepLast && diffPane.diffModel.takesAll(0, -1))
                   && went < hunkRunTimer.witnessed("rows0")) {
                if (went > 0)
                    runs.driver.letWriteGo()
                if (!runs.driver.pressWrite("hunk-run", () => {
                    page.stageSelection(0, -1)
                    return true
                }))
                    break
                went++
            }
            return went
        }
        /// Where the last press left the pane, read on its own turn.
        function landing() {
            const model = diffPane.diffModel
            return "pressed_diff=" + page.diffKey
                   + " pressed_same=" + (diffPane.view.count === hunkRunTimer.witnessed("rows0"))
                   + " pressed_blank=" + (diffPane.view.count === 0)
                   + " pressed_waiting=" + model.loading
                   // The next frame's `No changes to show`, from the count as it is now: the notices hear of the rows
                   // a polish later (`DiffFileNotices.headersOnly`).
                   + " pressed_nothing=" + (diffPane.view.count === 0 && !model.loading && model.modeFrom === "")
                   + " pressed_open=" + page.diffPressable
        }
        onTriggered: {
            const op = hunkRunTimer
            const apart = Harness.autoActArg === "apart"
            if (op.at("start")) {
                if (!Awaited.all("hunk_run_row", { "row": unstagedModel.holdsPath("unstaged", op.file) }))
                    return
                op.input("opened", () => page.openDiff("unstaged", op.file, ""))
                return
            }
            if (op.at("opened")) {
                if (!Awaited.all("hunk_run_open", { "open": page.diffKey === "unstaged:" + op.file,
                                                    "rows": diffPane.view.count > 0,
                                                    "settled": !page.diffSettling }))
                    return
                op.witness("rows0", diffPane.view.count)
                op.input("pressed", () => {
                    const went = op.pressHunks(apart)
                    op.witness("presses", went)
                    if (!apart)
                        op.witness("landing", op.landing())
                    return went > 0
                })
                return
            }
            if (apart && op.at("pressed")) {
                // The pane has read what git left: the last hunk alone, on a file now on both sides.
                if (!Awaited.all("hunk_run_left", { "written": runs.wroteAndSettled(),
                                                    "open": page.diffKey === "unstaged:" + op.file,
                                                    "rows": diffPane.view.count > 0,
                                                    "last": diffPane.diffModel.takesAll(0, -1) }))
                    return
                op.input("last", () => {
                    const went = op.pressHunks(false)
                    op.witness("landing", op.landing())
                    return went > 0
                })
                return
            }
            op.complete({
                "name": "hunk_run_landed",
                "when": () => ({ "written": runs.wroteAndSettled(), "rows": diffPane.view.count > 0,
                                 "page": PageSettled.settled(page) }),
                "report": () => "hunk_run form=" + Harness.autoActArg
                                + " " + op.witnessed("landing")
                                + " landed=" + page.diffKey
                                + " same=" + (diffPane.view.count === op.witnessed("rows0"))
                                + " open=" + page.diffPressable })
        }
    }
}
