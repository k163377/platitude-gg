import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One assigned avatar, as the settings card lists them: the picture, the name in the shared column, the address, and
// the Remove the row is held down by. The row writes as it is worked (`AppBackend.removeAvatar`) — the card has no
// Save to wait for.
RowLayout {
    id: avatarRow

    /// The assignment this row shows ({ email, name, url } — `SettingsAppPane.assigned`) and its place in the list.
    /// Declared, not inherited: a delegate in a file of its own is only handed the roles it names.
    required property var modelData
    required property int index
    /// The one column every row's name is laid into, as wide as the widest of them (`avatarRepeater.nameColW`).
    property real nameColW: 0
    /// Stands in for the pointer on one row's Remove, which headless cannot inject (`SettingsAppPane.pointedAtRow`).
    property int pointedAtRow: -1

    spacing: Theme.spaceSm
    /// The hand is on this row's Remove: the pointer on the
    /// button itself, the keyboard's focus (the hold's other
    /// hand — デザイン規約 §長押し), or headless having put it
    /// there for a shot.
    ///
    /// The button rather than the row it sits in: red says
    /// what the hand is about to lose, and a word that
    /// reddens while the pointer is still three columns of
    /// data away is about none of them (observed).
    readonly property bool lit: unsetButton.hovered || unsetButton.activeFocus
        || avatarRow.pointedAtRow === avatarRow.index
    /// Automation reads and works the row through these two rather than reaching inside it: the
    /// picture the list is photographed for, and the hold it has no hand to make.
    function pictureReady() {
        return rowFace.pictureReady()
    }
    function holdRemove() {
        unsetButton.completeHold()
    }
    IdentIcon {
        id: rowFace
        imageUrl: avatarRow.modelData.url
        width: Theme.iconLg
        height: Theme.iconLg
        Layout.preferredWidth: Theme.iconLg
        Layout.preferredHeight: Theme.iconLg
    }
    /// The width this row asks the shared name column to
    /// hold — its own glyphs, and nothing for the column's
    /// spare width, which stays air.
    readonly property real nameSeat: nameLabel.implicitWidth
    Label {
        id: nameLabel
        text: avatarRow.modelData.name
        color: Theme.textPrimary
        font.pixelSize: Theme.fontMd
        Layout.preferredWidth: avatarRow.nameColW
        elide: Text.ElideRight
    }
    Label {
        Layout.fillWidth: true
        text: avatarRow.modelData.email
        elide: Text.ElideRight
        color: Theme.textMuted
        font.pixelSize: Theme.fontSm
    }
    // Held, not clicked: this card writes as it is worked
    // rather than on a Save, so the gesture is the only
    // thing standing between a stray click and a picture
    // that has to be found again (デザイン規約 §長押し).
    //
    // Red only where the hand is (デザイン規約 §状態): a
    // standing state colour is for saying that the usual
    // move is not available, and everything a settings card
    // does is the usual move — a red word per row says
    // nothing and thins out the warnings that mean it. The
    // frame is what carries "this is a control" at rest:
    // the other three columns are data, and a bare word
    // among them reads as a fourth one. `*Dim` belongs on
    // that frame rather than on the word, which is the one
    // use §暗く落とした段 allows for it. Kept in the layout
    // either way, so the address beside it does not
    // re-elide as the pointer crosses the list.
    ActionButton {
        id: unsetButton
        text: qsTr("Remove")
        font.pixelSize: Theme.fontMd
        tone: avatarRow.lit ? Theme.danger : Theme.textSecondary
        frameColor: avatarRow.lit ? Theme.dangerDim : Theme.borderSubtle
        holdMs: Metrics.holdMs
        holdTone: Theme.danger
        onHeld: AppBackend.removeAvatar(avatarRow.modelData.email)
    }
}
