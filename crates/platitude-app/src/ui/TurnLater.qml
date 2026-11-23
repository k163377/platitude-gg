import QtQuick

// A call a turn later: `triggered` comes once, a turn after the last `restart()` — `Qt.callLater`'s queue, with its
// coalescing (asked again in the same turn, it moves to the queue's end and still comes once). Shaped as `FrameLater`
// is (`restart` / `triggered`).
//
// **The answer is a signal, never a QML function handed to `Qt.callLater`**: a tab's page is taken down with its call
// still queued. The page's `Loader` clears every context in the page at once and deletes the objects only later
// (`QQuickLoader::setActive(false)`, as `QQuickLoaderPrivate::clear` on a new source: `clearContextRecursively`, then
// `deleteLater`), and `Qt.callLater` holds back a call only for an object deleted or flagged for deletion
// (`QQmlDelayedCallQueue::DelayedFunctionCall::execute` reads `QQmlData::isQueuedForDeletion`, which JS `destroy()`
// sets and `deleteLater` does not) — so a queued QML function still runs, and the first QML function it calls on the
// cleared page answers "not a function" (`QQmlVMEMetaObject::method`: `attempted to evaluate a function in an invalid
// context`). A signal's handlers in a cleared context do not run (`QQmlBoundSignal_callback` reads the handler's
// engine, which the clear took), and the handler is the owner's (`onTriggered`), so the work goes with the page. A
// call that carries its own arguments each time (no coalescing) keeps the same rule: the closure handed to
// `Qt.callLater` only emits a signal of its owner (`WipBucketPane.eolAsked`, `FrameLater.heard`).
//
// A `QtObject`: held as a property's value where it is used (rules-refs/app-ui.md「Item の中に置く `QtObject` 製の部品」).
QtObject {
    id: later

    signal triggered()

    function restart() {
        Qt.callLater(later.triggered)
    }
}
