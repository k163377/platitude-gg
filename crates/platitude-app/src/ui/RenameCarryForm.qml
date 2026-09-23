import QtQuick
import QtQuick.Layouts
import platitude.ui

// What a rename made here asks back: which of the three things happens on the remote
// (デザイン規約 §手元の改名の後のリモート). It stands inside the question's bar, which is why it is built from a
// `Component`.
//
// **One chooser, not three pills.** Two of the three answers reach past this machine and one of those cannot be
// taken back, so the reader has to read them beside each other before picking one — a row of buttons would answer on
// the press that selects.
ColumnLayout {
    id: carryForm

    /// The three answers in the words they are picked by. **Handed in**: what they say names the two refs, and which
    /// refs those are is the flow's (`RenameCarryFlow`).
    required property var choices
    /// The one picked. **Never empty**: the question opens on the answer that takes nothing away
    /// (`RenameCarryFlow.choice`), so there is no placeholder state for this field to stand in.
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
        // The question leaves the keyboard in its form, the way every other question with one does (`AskBar.form`).
        Component.onCompleted: pick.forceActiveFocus()
    }
}
