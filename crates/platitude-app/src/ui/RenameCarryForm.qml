import QtQuick
import QtQuick.Layouts
import platitude.ui

// The chooser a rename made here asks with, inside the question's bar (デザイン規約 §手元の改名の後のリモート). One
// chooser, not three pills: one answer cannot be taken back, and a button would answer on the press that selects.
ColumnLayout {
    id: carryForm

    /// The three answers, handed in by the flow (`RenameCarryFlow`).
    required property var choices
    /// The one picked — never empty (`RenameCarryFlow.choice`), so there is no placeholder state.
    required property string picked

    signal choicePicked(int index)

    /// Automation only: no injected click opens a popup on the offscreen platform, so a run goes in at the field's
    /// own door (`AppCombo.pressField`).
    property alias pick: pick

    AppCombo {
        id: pick
        pickOnly: true
        Layout.fillWidth: true
        model: carryForm.choices
        wanted: carryForm.picked
        onActivated: index => carryForm.choicePicked(index)
        // The question leaves the keyboard in its form (`AskBar.form`).
        Component.onCompleted: pick.forceActiveFocus()
    }
}
