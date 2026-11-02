import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The word half of a toolbar action: the wording, a command's flag with drawn dashes, the command's chip, and the
// failure mark. Measured into the widest wording the button ever says, so the toolbar holds still across states.
Item {
    id: btnLabel

    property string text: ""
    /// A git command in git's own spelling, wearing the chip (デザイン規約 §git 用語のコード表記).
    property bool code: false
    /// The line box where this word is one line of a button written on two (`ActionButton.stacked`): the two families
    /// answer a line box differently at one step, so left to the family the mark under it sits a row off its
    /// neighbours. Zero takes the family's own.
    property int lineBox: 0
    /// Heavier only beside a command's mono chips (`TopBar`'s find button): a UI face at the same size reads lighter.
    property int wordWeight: Font.Normal
    /// The wording the box is measured for, and whether it is a command (the two families measure differently).
    property string widestText: ""
    property bool widestCode: false
    /// The button is down to its mark (`ActionButton.folded`). The cell keeps its height: folding gives up the width
    /// alone (`ActionButton.foldedDepth`).
    property bool folded: false
    readonly property int wordCeiling: 240
    /// The whole wording's width, off a hidden label with typed dashes — what a band-laid button folds against
    /// (`ActionButton.foldWidth`): a word there is drawn whole or not at all (`ActionButton.folded`). Not off what is
    /// drawn: the head and the drawn flag change width with the fold, and reading them back is a ring.
    readonly property real wantWidth: btnLabel.phrased ? phraseRow.implicitWidth
        : Math.min(wanted.implicitWidth, btnLabel.wordCeiling)
    /// What the chip is drawn round and the `!` stands past: the ink. Not the shared box, which would leave a tail
    /// with no word in it.
    readonly property real inkWidth: btnLabel.headRun + btnLabel.flagRoom
    readonly property real headRun: headText.width
    /// The button's own `fg`.
    property color tint: Theme.textPrimary
    /// The last go at this button's action failed.
    property bool alert: false
    property color alertTone: btnLabel.tint
    /// How high and how far right the mark's ink may stand, in this cell's coordinates: half a gap inside the frame
    /// the cell sits in (`ActionButton.alertTopEdge`). A shoulder that would reach past either is pulled back in.
    property real alertCeiling: Number.NEGATIVE_INFINITY
    property real alertWall: Number.POSITIVE_INFINITY
    /// The word the mark rides: where its letters end and where its ground starts. A word with no chip takes the mark
    /// half a gap into its trailing bearing, the air a chip's ground stands in (`NameCell`'s mark likewise).
    readonly property real alertWordEnd: btnLabel.inkWidth - (btnLabel.code ? 0 : Theme.spaceXs / 2)
    readonly property real alertWordTop: ground.y
    property int fontSize: Theme.fontMd

    // The widest wording's ink and nothing else; the air to the frame is `ActionButton.slack`'s. A trailing gap added
    // here turns a 1px win between wordings into a 5px difference in width, and the pair grows silently when the font
    // changes. Measured from the font even where the flag is drawn, so it stays put whatever rule the flag uses.
    readonly property real box: widest.implicitWidth
    /// A command's flag, set apart so its dashes can be drawn: a mono dash fills its cell, leaving the mark no room
    /// past the widest wording (デザイン規約 §git 用語のコード表記). Drawn as `Rectangle`s — another dash glyph frees
    /// not a pixel; not a wider box to open the room either, which would widen the pair for every state.
    readonly property int flagAt: btnLabel.code ? btnLabel.text.indexOf(" -") : -1
    readonly property bool splitFlag: btnLabel.flagAt > 0
    /// Gap and drawn rules included, off the flag's own row.
    readonly property real flagRoom: btnLabel.splitFlag ? flagRow.implicitWidth : 0
    readonly property string head: btnLabel.splitFlag
        ? btnLabel.text.substring(0, btnLabel.flagAt) : btnLabel.text
    readonly property string flagRest: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).replace(/^-+/, "") : ""
    readonly property int dashCount: btnLabel.splitFlag
        ? btnLabel.text.substring(btnLabel.flagAt + 1).length - btnLabel.flagRest.length : 0

    /// A phrase: a command chip, words, and a second chip in its own colour — the commit button's (デザイン規約
    /// §git 用語のコード表記). Empty takes the single-wording path. Not `code`, which is for a label that is the
    /// command whole; a phrase puts words between. Outside the toolbar's shared box (`widestText`): a phrased button
    /// has nobody to share a width with.
    property string phraseHead: ""
    /// The phrase's one number, apart so the words can give way without it.
    property string phraseCount: ""
    property string phraseTail: ""
    property color phraseTailTint: btnLabel.tint
    /// The mark the tail's name wears in front of it, on the chip in the name's ink (デザイン規約 §ref の種別
    /// 「名前の印」); empty for none.
    property string phraseTailMark: ""
    /// A short warning after the phrase's words, in the note's step and colour (`AppMenuItem.note`). Inside the phrase:
    /// a line of its own above a pane-filling button reads as the pane's heading (デザイン規約 §フル interactive rebase).
    property string phraseNote: ""
    property color phraseNoteTint: Theme.warning
    /// The hold's mark on a held phrase button, ahead of the first chip (§回数押し): the seat stands at the row's edge,
    /// far from a centred phrase.
    property int phraseHoldPresses: 0
    property real phraseHoldProgress: 0
    /// At rest what pressing through costs (`ActionButton.holdHue`), a step down while [`phraseHoldOff`]; the words'
    /// over the fill and while waiting.
    property color phraseHoldTint: btnLabel.tint
    property bool phraseHoldOff: false
    /// The face the action is attributed to, at the phrase's end (`… by <face>`); -1 draws none
    /// (デザイン規約 §アバターを与える).
    property int phraseFace: -1
    property string phraseFaceUrl: ""
    /// How many more people the action credits, as the co-author line's `+N`; zero draws none.
    property int phraseMates: 0
    /// The face's signature mark (`SignatureMark.kind`) and its tooltip line; empty draws none.
    property string phraseSignature: ""
    property string phraseSignatureTip: ""
    /// Stands in for the pointer on that mark (hover cannot be injected).
    property bool phraseSignaturePointedAt: false
    /// Whether that line is on screen — the tooltip's own visible, so a cut binding cannot read as green.
    readonly property bool phraseSignatureTipShown: phraseAvatar.signatureTipShown
    readonly property bool phrased: btnLabel.phraseHead !== ""
    /// The width this cell was given, not the phrase's own: a request for the full width is a floor the pane cannot go
    /// under, and the column would grow with the branch name (`details-fit`). No ring: nothing here feeds it back.
    readonly property real phraseRoom: btnLabel.phrased ? btnLabel.width : 0
    // What gives when the phrase is short of room, in this order (user decision — ask before changing): `+N`, then the
    // app's words, then the ref. `+N` first: the one part whose loss leaves where the commit lands unchanged. The count
    // stands apart on the fixed side — the phrase's one number, almost free to keep — as do the command's chip, `by`,
    // the face and the signature mark (each its own fixed length) and the note (`%n already pushed`: a count and set
    // words, nothing to grow). The ref is cut in the middle, since branch names differ at the tail (`feature/…-a` /
    // `feature/…-b`). A remote-tracking name is to be cut at the end, keeping its remote — no phrase shows one today,
    // and the phrase's chips keep `CodeChip.cutAt`'s default (`middle`), so a phrase that adds one sets it. Nothing
    // is cut before the phrase has taken the frame's whole width.
    /// The parts that never give, each with its following gap — per part, since a `Row` charges a gap only for a
    /// visible child.
    readonly property real phraseFixed:
        (btnLabel.phraseHoldPresses > 0 ? phraseHoldSeat.width + phraseRow.spacing : 0)
        + (phraseAlert.visible ? phraseAlert.width + phraseRow.spacing : 0)
        + headChip.implicitWidth + phraseRow.spacing
        + (countWord.visible ? countWord.implicitWidth + phraseRow.spacing : 0)
        + (noteWord.visible ? noteWord.implicitWidth + phraseRow.spacing : 0)
        + (btnLabel.phraseFace >= 0 ? byWord.implicitWidth + phraseAvatar.width + 2 * phraseRow.spacing : 0)
    readonly property real phraseFree: Math.max(0, btnLabel.phraseRoom - btnLabel.phraseFixed)
    /// The ref gets what the weaker two do not need, down to a floor: elided past it a branch name says nothing.
    readonly property real tailCap:
        btnLabel.phraseRoom <= 0 ? 0 : Math.max(Theme.buttonMinWidth,
                                                btnLabel.phraseFree - btnLabel.midCap - btnLabel.mateRoom)
    /// The floor for anything that gives: a cut leaves a `…` behind (user decision — ask before changing). A part gone
    /// in silence reaches the reader as a sentence that never said it; words and `+N` alike keep one mark's width
    /// (`by <face> …` says there are more names).
    readonly property real cutMark: ellipsis.implicitWidth
    readonly property real midCap:
        btnLabel.phraseRoom <= 0 || btnLabel.text === "" ? 0
        : Math.max(btnLabel.cutMark, Math.min(midWords.implicitWidth,
                                              btnLabel.phraseFree - tailWord.wantWidth - btnLabel.mateRoom
                                              - phraseRow.spacing))
    readonly property bool mateFits:
        btnLabel.phraseFree - tailWord.wantWidth - midWords.implicitWidth
        >= mateFull.implicitWidth + phraseRow.spacing
    readonly property string mateWord: qsTr("+%1").arg(btnLabel.phraseMates)
    readonly property real mateRoom:
        btnLabel.phraseMates <= 0 ? 0
        : (btnLabel.mateFits ? mateFull.implicitWidth : btnLabel.cutMark) + phraseRow.spacing

    /// Automation: where the plain word's baseline stands, in `item`'s y (`ActionButton.wordBase`).
    function wordBase(item) {
        return headText.mapToItem(item, 0, headText.baselineOffset).y
    }
    /// The word's baseline in this label's y, as a property so a binding follows it: the seat beside the word stands
    /// its mark on it (`ActionButtonSeat.wordBase`).
    readonly property real headBase: headText.y + headText.baselineOffset

    implicitWidth: btnLabel.phrased ? phraseRow.implicitWidth
                   : btnLabel.folded ? 0
                   : btnLabel.headRun + (btnLabel.splitFlag ? flagRow.width : 0)
    implicitHeight: btnLabel.phrased ? phraseRow.implicitHeight
                    : btnLabel.lineBox > 0 ? btnLabel.lineBox : headText.implicitHeight
    Layout.maximumWidth: btnLabel.phrased ? Number.POSITIVE_INFINITY : btnLabel.wordCeiling
    // Its own width, not the shared box's (the button's `slack` holds the rest), so the chip and the mark measured off
    // this cell sit on the word. A phrase asks for nothing (see `phraseRoom`).
    Layout.fillWidth: btnLabel.phrased
    Layout.preferredWidth: btnLabel.phrased ? 0 : btnLabel.implicitWidth
    Layout.alignment: Qt.AlignVCenter

    // Measured only. A Label, not TextMetrics, which reports a few pixels tighter than the visible Label. A hidden
    // ruler wears every font setting the drawn word wears — the chip's `wordSpacing`, the weight — or the box comes out
    // wider than the word.
    PlainTextLabel {
        id: widest
        visible: false
        text: btnLabel.widestText
        font.family: btnLabel.widestCode ? Theme.monoFamily : Theme.uiFamily
        font.wordSpacing: btnLabel.widestCode ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    // `wantWidth`'s ruler.
    PlainTextLabel {
        id: wanted
        visible: false
        text: btnLabel.text
        font.family: btnLabel.code ? Theme.monoFamily : Theme.uiFamily
        // The drawn weight, or the want is measured in a face the word is not set in (as `widest` above).
        font.weight: btnLabel.wordWeight
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
    }
    // Rulers for the cut mark and the whole `+N`: both are read to work out who gives, so neither can be an item whose
    // width that answer sets.
    Label {
        id: ellipsis
        visible: false
        text: "…"
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    PlainTextLabel {
        id: mateFull
        visible: false
        text: btnLabel.mateWord
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    // The plain word's line, which `lineBox` is a box for.
    Label {
        id: lineRef
        visible: false
        text: "Ag"
        font.family: Theme.uiFamily
        font.pixelSize: btnLabel.fontSize
    }
    // The chips hang their ground half a gap past their glyphs, so the row's gaps are plain word spacing.
    Row {
        id: phraseRow
        visible: btnLabel.phrased
        // Centred here: `centred`'s spacers cannot centre a cell that fills the row.
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.spaceXs
        // The hold mark is a letter of the phrase (デザイン規約 §アイコンの席 の名指し): as tall as the plain words'
        // capitals and on the middle they are drawn to (`lineRef`, centred in the row as each word is), in the middle
        // of the seat of their letters, the button's side given back — the padding already parts the mark from the
        // frame (`MarkSeat.leadWidth` / `leadX`).
        Item {
            id: phraseHoldSeat
            visible: btnLabel.phraseHoldPresses > 0
            width: MarkSeat.leadWidth(btnLabel.fontSize, phraseRow.spacing,
                                      headChip.word !== "" && !btnLabel.alert ? Theme.spaceXs / 2 : 0)
            height: phraseHold.height
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: WordFace.drawnCapitalDrop(lineRef)
            HoldIcon {
                id: phraseHold
                x: MarkSeat.leadX(btnLabel.fontSize, phraseHold.width)
                size: btnLabel.fontSize
                progress: btnLabel.phraseHoldProgress
                presses: btnLabel.phraseHoldPresses
                tint: btnLabel.phraseHoldTint
                off: btnLabel.phraseHoldOff
            }
        }
        // A phrase's `!` stands at its left end as the button's icon: inside, it would split the sentence, and the
        // right end is the face's corner, where the signature mark stands (デザイン規約 §git 用語のコード表記「`!` の席」);
        // on the right end's shoulder it would sit by the signature tick and read as one mark, `✓!` (user decision).
        // It heads the words (デザイン規約 §アイコンの席 の文頭記号), on the plain words' line.
        NavIcon {
            id: phraseAlert
            visible: btnLabel.alert
            kind: "bang"
            set: "lead"
            size: btnLabel.fontSize
            wordBase: lineRef.baselineOffset
            tint: btnLabel.alertTone
            width: Theme.iconSm
            height: lineRef.height
            anchors.verticalCenter: parent.verticalCenter
        }
        CodeChip {
            id: headChip
            word: btnLabel.phraseHead
            tint: btnLabel.tint
            size: btnLabel.fontSize
            buttonWord: true
            anchors.verticalCenter: parent.verticalCenter
        }
        // Invisible when empty: a `Row` charges a gap for a visible child of no width.
        PlainTextLabel {
            id: countWord
            visible: btnLabel.phraseCount !== ""
            text: btnLabel.phraseCount
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        PlainTextLabel {
            id: midWords
            visible: btnLabel.midCap > 0
            width: Math.min(implicitWidth, btnLabel.midCap)
            elide: Text.ElideRight
            text: btnLabel.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        PlainTextLabel {
            id: noteWord
            visible: btnLabel.phraseNote !== ""
            text: btnLabel.phraseNote
            color: btnLabel.phraseNoteTint
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
            anchors.verticalCenter: parent.verticalCenter
        }
        // Capped (`tailCap`): the one part of unbounded length — git takes a branch name of any length. The mark on the
        // chip's own ground, so mark and name give way as one (`CodeChip.mark`).
        CodeChip {
            id: tailWord
            word: btnLabel.phraseTail
            tint: btnLabel.phraseTailTint
            mark: btnLabel.phraseTailMark
            size: btnLabel.fontSize
            buttonWord: true
            cap: btnLabel.tailCap
            anchors.verticalCenter: parent.verticalCenter
        }
        Label {
            id: byWord
            visible: btnLabel.phraseFace >= 0
            text: qsTr("by")
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
        // No address, so nothing here is pressable — no pen, no tooltip, only the signature mark: a button holds one
        // target, the phrase. `iconLg`, the graph's node step beside a `fontMd` subject.
        AvatarButton {
            id: phraseAvatar
            visible: btnLabel.phraseFace >= 0
            face: btnLabel.phraseFace
            faceUrl: btnLabel.phraseFaceUrl
            signatureKind: btnLabel.phraseSignature
            signatureTip: btnLabel.phraseSignatureTip
            signaturePointedAt: btnLabel.phraseSignaturePointedAt
            // A small face drops its badge a step too, so the badge keeps its share of the face (`AvatarButton`).
            badgeInk: Theme.iconXs
            // Clear of the round face's corner, its ink's top right a pixel short of the frame beside it: how far out
            // it may stand is the neighbour's to say (`AvatarButton.badgeTopOut`).
            badgeTopOut: Theme.spaceXs - Theme.borderWidth
            badgeRightOut: Theme.spaceXs - Theme.borderWidth
            width: Theme.iconLg
            height: Theme.iconLg
            // A picture beside words, on their capitals' middle (デザイン規約 §アイコンの席).
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: WordFace.capitalDrop(lineRef)
        }
        PlainTextLabel {
            id: mateCount
            visible: btnLabel.phraseMates > 0
            text: btnLabel.mateFits ? btnLabel.mateWord : ellipsis.text
            color: btnLabel.tint
            font.family: Theme.uiFamily
            font.pixelSize: btnLabel.fontSize
            anchors.verticalCenter: parent.verticalCenter
        }
    }
    PlainTextLabel {
        id: headText
        visible: !btnLabel.phrased && !btnLabel.folded
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        // Where this word is one of two, onto the plain word's baseline row (`lineRef`), not its own family's centre.
        anchors.verticalCenterOffset: btnLabel.lineBox > 0
            ? Math.floor((btnLabel.lineBox - lineRef.height) / 2 + lineRef.baselineOffset)
              - Math.floor((btnLabel.lineBox - headText.height) / 2 + headText.baselineOffset)
            : 0
        // Not the cell's width, which comes from this (a loop).
        width: Math.min(implicitWidth, btnLabel.wordCeiling)
        text: btnLabel.head
        color: btnLabel.tint
        font.family: btnLabel.code ? Theme.monoFamily : Theme.uiFamily
        font.weight: btnLabel.wordWeight
        // A chip's words close up by a gap (`CodeGround`): a mono space would stand the flag off as a second word.
        font.wordSpacing: btnLabel.code ? -Theme.spaceXs : 0
        font.pixelSize: btnLabel.fontSize
        elide: Text.ElideRight
    }
    // The flag: the mono space's air, a drawn rule per dash (the font's own dash is `borderWidth` thick) with a hair of
    // air after it, and the letters in the font.
    Row {
        id: flagRow
        visible: btnLabel.splitFlag && !btnLabel.folded
        anchors.left: headText.left
        anchors.leftMargin: btnLabel.headRun
        anchors.verticalCenter: headText.verticalCenter
        spacing: 0
        Item {
            width: Theme.spaceXs
            height: headText.height
        }
        Repeater {
            model: btnLabel.dashCount
            Item {
                width: Theme.spaceXs + Theme.borderWidth
                height: headText.height
                Rectangle {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.spaceXs
                    height: Theme.borderWidth
                    color: btnLabel.tint
                }
            }
        }
        PlainTextLabel {
            text: btnLabel.flagRest
            color: btnLabel.tint
            font.family: Theme.monoFamily
            font.pixelSize: btnLabel.fontSize
        }
    }
    // The chip, as wide as the ink rather than the shared box, hanging half a gap off either end
    // (デザイン規約 §git 用語のコード表記).
    CodeGround {
        id: ground
        z: -1
        visible: btnLabel.code && !btnLabel.folded
        x: -Theme.spaceXs / 2
        width: btnLabel.inkWidth + Theme.spaceXs
        // Off the word's step, not the mono line box, which differs per OS (@14px: Cascadia Mono 16, Noto Sans Mono
        // CJK JP 21; `CodeGround`).
        size: btnLabel.fontSize
        // The word's baseline, not the cell's middle: a word moved onto a baseline (`headText`) would leave its ground
        // behind.
        wordBaseline: headText.y + headText.baselineOffset
        buttonWord: true
    }
    // The failure mark, on the right shoulder of the word it is about: past the word's end, its ink half a gap over the
    // word's ground — pulled back wherever that would reach the frame (デザイン規約 §git 用語のコード表記「`!` の席」).
    // In a shared box (`TopBar.widestAction`) only the shorter words get slack: the word that set the box has no room
    // for the `!` and would stand it against the frame.
    NavIcon {
        id: alertMark
        // Folded, the seat wears it instead (`ActionButtonSeat.cornerAlert`); a phrase, at its left end (`phraseAlert`).
        visible: btnLabel.alert && !btnLabel.folded && !btnLabel.phrased
        kind: "bang"
        tint: btnLabel.alertTone
        width: Theme.iconSm
        height: Theme.iconSm
        x: Math.min(btnLabel.alertWordEnd + alertMark.inkRight, btnLabel.alertWall) - alertMark.inkRight
        y: Math.max(btnLabel.alertWordTop - Theme.spaceXs / 2, btnLabel.alertCeiling) - alertMark.inkTop
    }
}
