import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A tick for a signature, a bang for a bad one. The seat is the caller's (規約 §署名の表示): on the shoulder of the
// committer's name (`CommitAuthorRow`), or on the corner of a face where no name stands to hang it off
// (`AvatarButton`).
//
// git's eight answers fold to two shapes in three colours (`hub::sink` maps them): the shape says a signature is
// there, the colour who vouched for it. `G` — a tick in `success`; `E` / `U` / `X` / `Y` / `R` — a tick in
// `textSecondary`, a signature read but not judged; `B` — a bang in `danger` with the words `Bad signature` beside it;
// `N` — nothing at all, so an answer arriving late never flickers. **Green is kept for verified**: SSH without
// `allowedSigners` drops every signature to the second row, and greened that would show "verified" on the screen of
// everyone who has not set it up. The grey tick is no disabled look (規約 §無効 dims words; this is a mark). git's own
// reason (a key expired, nobody vouched for it) is the hover's one line.
Item {
    id: signatureMark

    /// What git makes of the signature: "" (unsigned, or not answered yet), "verified", "signed" or "bad". `code` is
    /// git's own `%G?` letter, which the tooltip needs: the mark says one of three things, the reasons are more.
    property string kind: ""
    property string code: ""
    property string signer: ""
    /// Stands in for the pointer, so the tooltip can be photographed (PGG_AUTO_ACT=signature-tip).
    property bool pointedAt: false
    /// The ink of the mark itself. A smaller face hands in a smaller ink (`AvatarButton.badgeInk`).
    property int ink: Theme.iconSm
    /// Line weight: the seat on a name's shoulder hands in the `iconSm` box's line, the grid's ratio of `iconStroke`
    /// (デザイン規約 §アイコンの席 の右肩); the badge on a face keeps the default.
    property real stroke: Metrics.iconStroke

    /// The tooltip is up, read off the seat's own answer — the output side, so a cut binding cannot read as green.
    readonly property bool tipShown: markTip.shown
    /// Whether the pointer is on the badge itself, which the face under it reads to keep its own tooltip out of the way
    /// (規約 §hover のツールチップ「1 つのポインタが開けるものは 1 つ」).
    readonly property bool pointed: signatureHover.hovered || signatureMark.pointedAt

    /// The air this mark's square holds past its ink on each side, for the caller to take off its seat — the seat is
    /// the ink (規約 §余白). The ink stands in the middle of the square (`NavIcon.reaches`), so left and right are one.
    /// The air differs per kind (tick and bang), so it is read off this mark's own ink — `inkWidth` for how wide it
    /// is, `inkRight` / `inkTop` for where it is, line and mitred corners included — never one number for both
    /// kinds, which leaves the bad signature alone off its seat.
    readonly property real inkAirLeft: mark.inkRight - mark.inkWidth
    readonly property real inkAirRight: mark.width - mark.inkRight
    readonly property real inkAirTop: mark.inkTop

    readonly property bool broken: signatureMark.kind === "bad"
    readonly property color tone: signatureMark.kind === "verified" ? Theme.success
        : signatureMark.broken ? Theme.danger : Theme.textSecondary
    /// One line, conclusion first (デザイン規約 §hover のツールチップ). git's verdict by default; the commit editor's
    /// badge is a setting and hands its own sentence in.
    property string tip:
        signatureMark.code === "G"
        ? (signatureMark.signer !== ""
           ? qsTr("Signed by %1").arg(signatureMark.signer)
           : qsTr("Signed by a key you trust"))
        : signatureMark.code === "B"
          ? qsTr("The content changed after it was signed")
        : signatureMark.code === "U"
          // git names no signer for this one: it cannot put a name to the key.
          ? qsTr("The key is not one you have vouched for")
        : signatureMark.code === "X"
          ? qsTr("The signature has expired")
        : signatureMark.code === "Y"
          ? qsTr("The signing key has expired")
        : signatureMark.code === "R"
          ? qsTr("The signing key was revoked")
        : qsTr("Cannot be checked — the key is not here")

    visible: signatureMark.kind !== ""
    implicitWidth: signatureMark.ink
    implicitHeight: signatureMark.ink

    // Bare, unlike the pen: on a face it stands out past the corner the round face leaves empty, on the ground beside
    // it, where a disc would only be a second small frame (`AvatarButton`).
    NavIcon {
        id: mark
        anchors.fill: parent
        kind: signatureMark.broken ? "bang" : "check"
        tint: signatureMark.tone
        stroke: signatureMark.stroke
    }
    readonly property TipAsk markTip: TipAsk {
        id: markTip
        target: signatureMark
        asked: signatureMark.pointed
        words: signatureMark.tip
    }
    // A handler, being passive: the face under this keeps the hover that raises its own badge (規約 §hover のツールチップ).
    HoverHandler { id: signatureHover }
}
