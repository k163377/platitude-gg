pragma Singleton

import QtQuick

// How many marks in this window have been made but not yet drawn; `AutoShotDriver` holds the picture until this is back
// to zero (rules-refs/app-ui.md「`Canvas` は生まれたフレームにインクを持たない」).
QtObject {
    /// Marks waiting for the paint that first puts them on screen (`InkCanvas`).
    property int owed: 0
}
