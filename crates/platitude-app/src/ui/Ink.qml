pragma Singleton

import QtQuick

// How many marks in this window have been made but not yet drawn.
//
// A `Canvas` has no ink in the frame its item is born in: Qt creates the 2D context on a queued call, so the first
// `onPaint` cannot land before the turn after. Everything else about the row is there in that first frame — the layout
// has run and the name's text node is built in the same polish — so what a picture taken in between shows is a row
// with its name and an **empty seat where its mark goes**.
//
// A headless run lands in that gap on purpose: the working tree's buckets stand up when the status
// arrives, which is the same edge the verbs wait on, so the rows are built in the very turn the verb completes in and
// `AutoShotDriver` asks for the picture. It holds the picture until this is back to nothing.
//
// **Only the first ink is counted.** A repaint draws over ink that is already on screen, and a mark that redraws itself
// every frame — the lanes under a scroll, a spinner — would otherwise never let a picture be taken at all.
QtObject {
    /// Marks waiting for the paint that first puts them on screen (`InkCanvas`).
    property int owed: 0
}
