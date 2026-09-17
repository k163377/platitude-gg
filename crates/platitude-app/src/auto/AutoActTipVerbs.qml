pragma ComponentBehavior: Bound

import QtQuick
// For the attached types alone (rules-refs/app-ui.md carries what an unimported one answers).
import QtQuick.Controls.Fusion
import platitude
import platitude.ui

/// The cards and tooltips something puts out under a pointer — the signature, a stash, a path, an author, the
/// people a message credits. Each of these names its target — hover cannot be injected.
///
/// Built by `AutoActDriver`, which `RepoPage` builds only when a verb was given. What these verbs act on
/// hangs off that driver; the names it owns are
/// read back once below so the verbs can name them bare.
// An `Item` only because `QtObject` has no default property to hold the timers below; it is a
// sizeless holder.
Item {
    id: acts

    /// The driver these verbs belong to. `var` because naming its type here would be a circle: it is
    /// the file that builds this one.
    required property var driver

    // The driver's own names, read once so the verbs can name them bare.
    readonly property var page: driver.page
    readonly property var workTree: driver.workTree
    readonly property var graphModel: driver.graphModel
    readonly property var detailsModel: driver.detailsModel
    readonly property var worktreeModel: driver.worktreeModel
    readonly property var graphPane: driver.graphPane
    readonly property var detailsPane: driver.detailsPane
    readonly property var wipPane: driver.wipPane
    readonly property var carriedPane: driver.carriedPane

    /// Runs `act` if it is one of this family's, and says whether it was. The families are asked in turn
    /// and the first to know a verb runs it — each verb has one family (`AutoActDriver`).
    function run(act, arg) {
        if (act === "signature") {
            // The mark appears when the verify comes back, so the report waits for it.
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTimer.start()
        } else if (act === "signature-tip") {
            // Once the verify is back the mark is asked to say why.
            page.activateRow(graphModel.oidAt(Number(arg)))
            signatureTipTimer.start()
        } else if (act === "stash-tip") {
            // The box is asked why it refuses the caret — the row decides which refusal answers. `older` names the
            // row under HEAD's, so a commit other than HEAD's own is found off the graph itself.
            page.activateRow(graphModel.oidAt(
                arg === "older" ? graphModel.rowOf(workTree.headOid) + 1 : Number(arg)))
            stashTipTimer.start()
        } else if (act === "path-tip") {
            // Row 0 is the elided leaf in the flattened view, the folder chain in the tree (`-tree`). The argument
            // picks the pane the way `corner` does, with one more: `carried:<copy>` is another working copy's, which
            // is named — its rows all answer to the same all-zero id (`driver.rowOfCopy`).
            // **The tree's suffix is read off the whole argument**, so a copy whose own folder ends in `-tree` is
            // read as the tree form of a copy without it; the presets hold no such name.
            const wantsTree = ("" + arg).endsWith("-tree")
            const pane = wantsTree ? ("" + arg).slice(0, -5) : arg
            pathTipTimer.carried = pane.startsWith("carried:") ? pane.slice(8) : ""
            pathTipTimer.wipSide = pane === "" || pane === "wip"
            if (pathTipTimer.carried !== "") {
                // The copy has to be stood on before its list exists at all, which is this timer's own first tick.
                page.setWipTreeView(wantsTree)
            } else if (pathTipTimer.wipSide) {
                page.showWip()
                worktreeModel.setTreeView(wantsTree)
            } else {
                page.activateRow(graphModel.oidAt(Number(pane)))
                detailsModel.setTreeView(wantsTree)
            }
            pathTipTimer.start()
        } else if (act === "tip-copy" || act === "tip-sweep") {
            // The same row `path-tip` points at in its plain form, for the same reason: the flattened view spells a
            // whole path, which is the longest thing this window puts in a tooltip and so the one worth taking away.
            // The tree's row 0 is a folder and would put one word in the picture.
            page.showWip()
            worktreeModel.setTreeView(false)
            if (act === "tip-sweep")
                tipSweepTimer.start()
            else
                tipCopyTimer.start()
        } else if (act === "author-card" || act === "author-card-open") {
            // Hover cannot be injected, so `-open` writes the same property the handler writes; read the two as a pair
            // — "stayed shut" only means something next to a run where it opened.
            page.activateRow(graphModel.oidAt(Number(arg)))
            authorCardTimer.start()
        } else if (act === "co-authors" || act === "co-authors-open") {
            // Same pairing as author-card.
            page.activateRow(graphModel.oidAt(Number(arg)))
            coAuthorTimer.start()
        } else {
            return false
        }
        return true
    }
    // gpg / ssh-keygen have to finish before the mark they decide can be on screen, so the shot and the report both
    // wait for them.
    SampleTimer {
        id: signatureTimer
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTimer.stop()
            Harness.report("signature kind=" + page.selectedSignatureKind
                              + " code=" + page.selectedSignatureCode
                              + " signer=" + page.selectedSignatureSigner)
            driver.complete()
        }
    }
    // The tooltip halves of signature-tip / stash-tip: the state has to land (gpg's verdict, the stash's details)
    // before the target is pointed at, and the report then waits out Metrics.tipDelayMs so what it reads is the tip on
    // screen.
    SampleTimer {
        id: signatureTipTimer
        onTriggered: {
            if (!page.signatureIsForSelection)
                return
            signatureTipTimer.stop()
            detailsPane.signaturePointedAt = true
            signatureTipReport.start()
        }
    }
    SampleTimer {
        id: signatureTipReport
        onTriggered: {
            if (!detailsPane.signatureTipShown)
                return
            signatureTipReport.stop()
            Harness.report("signature_tip code=" + page.selectedSignatureCode
                              + " tip=" + detailsPane.signatureTipShown)
            driver.complete()
        }
    }
    SampleTimer {
        id: stashTipTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            stashTipTimer.stop()
            detailsPane.summaryPointedAt = true
            stashTipReport.start()
        }
    }
    SampleTimer {
        id: stashTipReport
        onTriggered: {
            if (!detailsPane.summaryTipShown)
                return
            stashTipReport.stop()
            Harness.report(
            "stash_tip blocked=" + (detailsPane.editBlocked !== "")
            + " tip=" + detailsPane.summaryTipShown + " why=" + detailsPane.editBlocked)
            driver.complete()
        }
    }
    // The tooltip half of path-tip: the list has to land before a row can be pointed at, and the report then waits out
    // tipDelayMs so what it reads is the tip on screen. It reads the shared instance itself — the one thing that can
    // also say the words on it.
    SampleTimer {
        id: pathTipTimer
        property bool wipSide: true
        /// The copy this run is pointing into, empty for the two panes of this window's own tree.
        property string carried: ""
        property bool stood: false
        onTriggered: {
            if (pathTipTimer.carried !== "") {
                if (!pathTipTimer.standOnCopy())
                    return
                // The copy's own files have to have arrived: until they do the pane holds this window's, and a row
                // pointed at there would be the wrong tree's.
                const files = page.wipUnstaged
                if (page.carriedPath === "" || files.carriedAt !== page.carriedPath || files.total === 0)
                    return
                if (carriedPane.view.count === 0)
                    return
                pathTipTimer.stop()
                carriedPane.pointedTipRow = 0
                pathTipReport.start()
                return
            }
            if (pathTipTimer.wipSide && worktreeModel.total === 0)
                return
            if (!pathTipTimer.wipSide && !driver.cardSettled)
                return
            pathTipTimer.stop()
            if (pathTipTimer.wipSide)
                wipPane.pointedTipRow = 0
            else
                detailsPane.pointedTipRow = 0
            pathTipReport.start()
        }
        /// Picks the copy's row, once, through the row's own press — the row's decision is the one taken
        /// (verify-ui §壊れない動詞). Answers whether the pane is standing on it.
        function standOnCopy() {
            if (pathTipTimer.stood)
                return true
            if (graphModel.finishCount === 0 || !workTree.loaded)
                return false
            const row = driver.rowOfCopy(pathTipTimer.carried)
            if (row < 0)
                return false
            const item = graphPane.view.itemAtIndex(row)
            if (item === null)
                return false
            item.leftClick(0, Qt.NoModifier)
            pathTipTimer.stood = true
            return false
        }
    }
    SampleTimer {
        id: pathTipReport
        onTriggered: {
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            pathTipReport.stop()
            Harness.report("path_tip pane="
                + (pathTipTimer.carried !== "" ? "carried"
                   : pathTipTimer.wipSide ? "wip" : "details")
                + " tree=" + (pathTipTimer.carried !== "" ? page.wipUnstaged.treeView
                              : pathTipTimer.wipSide ? worktreeModel.treeView : detailsModel.treeView)
                + " tip=" + tip.visible
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // A tooltip's words are a field (`CardText`), and this is the run that says so: it takes the whole of
    // the tip into a selection the way a reader's drag would, and reports what came back against what the tip is
    // showing. **Selection is as far as a run can go** — the clipboard belongs to the platform, and a run that wrote to
    // it would be testing Qt; the walk from the row into the tip cannot be injected at all
    // (verify-ui スキル), so that half is measured on a throwaway qmltestrunner scene.
    //
    // Asked of the contentItem, whatever it is: a tip wearing the style's own `Text` again has no
    // `selectAll` and the run dies on the spot, which is the answer.
    SampleTimer {
        id: tipCopyTimer
        onTriggered: {
            if (worktreeModel.total === 0)
                return
            wipPane.pointedTipRow = 0
            const tip = page.ToolTip.toolTip
            if (!tip.visible)
                return
            tipCopyTimer.stop()
            tip.contentItem.selectAll()
            Harness.report("tip_copy tip=" + tip.visible
                + " copied=" + (tip.contentItem.selected === tip.text)
                + " text=" + tip.text)
            driver.complete()
        }
    }
    // ...and the same words taken from the band around them — the padding a tip
    // keeps between its ground and its sentence, which is the whole of a tooltip's air (規約 §hover のツールチップ).
    //
    // **The starts are the air itself, sampled** (`SweepPad.airPoints`): a grid over the tip with the points standing
    // on the sentence dropped, which leaves exactly what a real press could reach the pad at. A run that pressed the
    // middle of a tip would be pressing on the words, which take their own press, and would go green with the pad
    // taken back out. `air=` is beside `reach=` because a tip is a small box: a run that found two places to press
    // has said less than one that found twenty, and the number says which it was.
    //
    // Reached the way `tip-copy` reaches the words — off the tip's own parts, whatever they are. A ground
    // that lost its pad dies here, which is the answer.
    SampleTimer {
        id: tipSweepTimer
        onTriggered: {
            if (worktreeModel.total === 0)
                return
            wipPane.pointedTipRow = 0
            const tip = page.ToolTip.toolTip
            if (!tip.visible || tip.width <= 0)
                return
            tipSweepTimer.stop()
            // **`all=` is the claim**: how many places a tip's air has depends on how long the path is
            // and on which machine drew it, so the number is a diagnosis and "every one of them, and there was at
            // least one" is the judgement. `caret=` is the half a selection does not say — `Ctrl+C` goes to the field
            // holding the keyboard, so a value picked out without one is not a value the reader can take away. The
            // sentence itself is the pad's (`SweepPad.sweepAir`), which is where the eight surfaces that carry this
            // hand say it once.
            Harness.report("tip_sweep " + tip.background.pad.sweepAir(7))
            driver.complete()
        }
    }
    // The details have to arrive before the name can name anybody. `cut=` leads: judgement is a run (verbs.md).
    SampleTimer {
        id: authorCardTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (Harness.autoAct === "author-card-open")
                detailsPane.messageBlock.showAuthor(true)
            if (Harness.autoAct === "author-card-open" && !detailsPane.authorCards.authorCardOpen)
                return
            authorCardTimer.stop()
            Harness.report(
                "author_card cut=" + detailsPane.messageBlock.nameClipped
                + " open=" + detailsPane.authorCards.authorCardOpen
                + " author=" + detailsPane.details.authorEmail
                + " committer=" + detailsPane.details.committerEmail
                + " other=" + detailsPane.details.committerDiffers
                + " later=" + detailsPane.details.commitTimeDiffers)
            driver.complete()
        }
    }
    // The details have to arrive before the credit line they carry can be opened or counted.
    SampleTimer {
        id: coAuthorTimer
        onTriggered: {
            if (!driver.cardSettled)
                return
            if (Harness.autoAct === "co-authors-open")
                detailsPane.messageBlock.showCoAuthors(true)
            if (Harness.autoAct === "co-authors-open" && !detailsPane.authorCards.matesCardOpen)
                return
            coAuthorTimer.stop()
            // `open` is the card's own visibility: reporting the input that asked for it would go green
            // with the binding cut. `cut=` is the credit line's own eliding, read as it is (verbs.md).
            Harness.report(
                "co_authors count=" + detailsPane.messageBlock.coAuthorRecords.length
                + " cut=" + detailsPane.messageBlock.matesClipped
                + " first=" + detailsPane.messageBlock.coAuthorName(0)
                + " open=" + detailsPane.authorCards.matesCardOpen)
            driver.complete()
        }
    }
}
