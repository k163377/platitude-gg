import QtQuick
import platitude

/// One wait for the frame that shows a change made before `ask()` (`PagePerfDriver`, which feeds it the window's
/// three signals). The render loop says them in an order it never breaks (`QSGThreadedRenderLoop::polishAndSync` /
/// `QSGRenderThread::syncAndRender`, and the basic and software loops' `renderWindow`): `afterAnimating` on the GUI
/// thread once the window's items are polished, which then asks for the sync and is held until it is done;
/// `afterSynchronizing` from that sync; `frameSwapped` once the frame is out, before the next sync is taken up. On
/// the threaded loop the last two reach the GUI thread queued, in the order the render thread said them. So the swap
/// that follows a pass's own sync is that pass's frame or a later one, and a swap that arrives before it is a frame
/// the change is not in — one already on its way, or one the render thread drew for an animator while the GUI
/// thread was busy.
///
/// What cannot be read off the order is whose sync a delivery is: the pass before may not have had its own
/// delivered yet when the next begins. `passed` counts that one in. A pass whose sync the render thread could not
/// make (a window it cannot sync says no `afterSynchronizing`) reads the same way, and the wait after it takes the
/// frame of the pass after its own — over a still scene, the pass the driver's next beat asks for. A sync with no
/// pass before it (a window grab) leaves a wait that has not had its pass alone, since a wait is owed nothing until
/// a pass has been made for it; one that lands after the pass cannot be told from the pass's own, and would answer
/// the wait a frame early — nothing grabs the window while a wait stands. `tst_perfframewait.qml`.
QtObject {
    id: wait

    /// "" (not waiting), "asked", "polished" (a pass has been made since), "synced" (that pass's sync is in).
    property string step: ""
    /// Syncs still to be delivered before the pass's own has been.
    property int syncsLeft: 0
    /// When the frame was asked for (`PerfProbe.clockMs`), and how many frames were swapped while this waited that
    /// the change was not in.
    property real askedAt: 0
    property int passedOver: 0
    /// Whether a pass has been made since this began listening, and how many syncs have been delivered since the
    /// latest one.
    property bool passSeen: false
    property int syncsThisPass: 0

    function ask() {
        wait.step = "asked"
        wait.syncsLeft = 0
        wait.passedOver = 0
        wait.askedAt = PerfProbe.clockMs()
    }
    /// `afterAnimating`, every one of them: the pass is what the next delivery is read against, waiting or not.
    function passed() {
        const behind = wait.passSeen && wait.syncsThisPass === 0
        wait.passSeen = true
        wait.syncsThisPass = 0
        if (wait.step !== "asked")
            return
        wait.step = "polished"
        wait.syncsLeft = behind ? 2 : 1
    }
    /// `afterSynchronizing`, every one of them.
    function synced() {
        wait.syncsThisPass++
        if (wait.step === "polished" && --wait.syncsLeft <= 0)
            wait.step = "synced"
    }
    /// `frameSwapped`: whether the frame just swapped is the one. Answers yes once, and the wait is over.
    function swapped() {
        if (wait.step === "synced") {
            wait.step = ""
            return true
        }
        if (wait.step !== "")
            wait.passedOver++
        return false
    }
}
