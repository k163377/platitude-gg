import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// A signature that holds is a tick and nothing more; only one that
// contradicts the content spends words (規約 §署名の表示). The broken
// case reads as the error message it is, and being the one wide thing
// on the row is how an error should read.
//
// Green stays with the signatures git actually vouched for. One it
// could read but not judge gets the same tick in textSecondary: the
// shape says a signature is there, the colour says nobody here
// checked it.
RowLayout {
    id: signatureMark

    /// What git makes of the signature: "" (unsigned, or not answered
    /// yet), "verified", "signed" or "bad". The letter behind it is
    /// git's own `%G?` code, which is what the tooltip needs — the mark
    /// says one of three things, but the reason a signature could not
    /// be judged is one of five.
    property string kind: ""
    property string code: ""
    property string signer: ""
    /// Stands in for the pointer, so the tooltip can be photographed
    /// (PG_AUTO_ACT=signature-tip) — hover cannot be injected.
    property bool pointedAt: false

    /// The tooltip is up. Reported through the ToolTip's own visible —
    /// the output side, so a cut binding cannot read as green.
    readonly property bool tipShown: signatureMark.ToolTip.visible

    readonly property bool broken: signatureMark.kind === "bad"
    readonly property color tone: signatureMark.kind === "verified" ? Theme.success
        : signatureMark.broken ? Theme.danger : Theme.textSecondary
    /// Conclusion first, one line (デザイン規約 §hover のツールチップ).
    readonly property string tip:
        signatureMark.code === "G"
        ? (signatureMark.signer !== ""
           ? qsTr("Signed by %1").arg(signatureMark.signer)
           : qsTr("Signed by a key you trust"))
        : signatureMark.code === "B"
          ? qsTr("The content changed after it was signed")
        : signatureMark.code === "U"
          // git names no signer for this one: the key it read is not one
          // it can put a name to.
          ? qsTr("The key is not one you have vouched for")
        : signatureMark.code === "X"
          ? qsTr("The signature has expired")
        : signatureMark.code === "Y"
          ? qsTr("The signing key has expired")
        : signatureMark.code === "R"
          ? qsTr("The signing key was revoked")
        : qsTr("Cannot be checked — the key is not here")

    visible: signatureMark.kind !== ""
    spacing: Theme.spaceXs
    NavIcon {
        kind: signatureMark.broken ? "bang" : "check"
        tint: signatureMark.tone
        width: Theme.iconSm
        height: Theme.iconSm
        Layout.alignment: Qt.AlignVCenter
    }
    Label {
        visible: signatureMark.broken
        text: qsTr("Bad signature")
        color: signatureMark.tone
        font.pixelSize: Theme.fontSm
        Layout.alignment: Qt.AlignVCenter
    }
    ToolTip.visible: signatureHover.hovered || signatureMark.pointedAt
    ToolTip.delay: Metrics.tipDelayMs
    ToolTip.text: signatureMark.tip
    // A handler, not a MouseArea: an item inside a layout is sized by
    // the layout, and anchoring one to fill its parent is undefined
    // behaviour.
    HoverHandler { id: signatureHover }
}
