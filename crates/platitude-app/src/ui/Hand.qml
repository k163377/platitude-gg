// Whether a hand is over this window at all: a headless run's cursor rests on the window's top left and would light
// it for the whole run (rules-refs/app-ui.md「ヘッドレスの窓には手が乗っている」). Set only by the harness; false in
// every window a person opens.
pragma Singleton

import QtQuick

QtObject {
    /// No hand over this window, so a pointer lights nothing. Read wherever a real pointer would
    /// (`HoverToolButton.lit`).
    property bool away: false
}
