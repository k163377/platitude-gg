//! The `demo-repo` section: one line per shape a repository can be handed to
//! the app in. Written where a preset is added (`demo.rs`), and kept apart
//! from the runner's own paragraphs so that neither list's growth is the
//! other's problem.

// No `"\` continuation on the opening line: it swallows the next line's
// leading whitespace, and every line of this page is indented.
pub(super) const PRESETS: &str = "  demo-repo <preset> [--at <dir>]
      Build a throwaway repository (isolated from your git config) and
      print its path. Presets:
        basic     branches + remote (ahead) + tags + stash + dirty WIP
        dirty     every WIP bucket: staged, unstaged, untracked, renamed
        eol       all four line-ending cases, in a directory of LF files
        clashing  two branches that will not merge cleanly, unmerged yet
                  (also the pick that will not copy cleanly)
        revert-clashes  a commit whose undoing collides with the branch
                  that carried on past it, not yet reverted
        conflict  a merge stopped on conflicts (MERGE_HEAD present)
        conflict-typed   the same, with the file typed over: no markers
                  left in the tree, still unmerged in the index
        conflict-staged  and staged: nothing waiting on a decision, the
                  merge still standing, the commit button live
        conflict-ours    the same merge kept as ours and staged: status
                  empty, merge still standing, a merge still to write —
                  the one clean tree with an uncommitted row over it
        rebase-clashes   the same two branches with the rebase not run
        rebase-conflict  a rebase stopped on conflicts, 1 of 2 steps
        rebase-staged    the same rebase, conflict resolved and staged
        rebase-empty     a rebase stopped on a commit that came out empty
        cherry-pick-conflict  a cherry-pick stopped on a conflict
        cherry-pick-quit  the same, let go of with --quit: no operation
                  standing, the same files still unmerged
        conflict-kinds   four kinds of conflict in one stopped merge
        drop-collides    a drop that replays clean and collides restoring
        drop-stops       a drop that stops part-way, over a dirty tree
        stashes   three stashes, one with untracked files
        detached  HEAD detached at a tag
        behind    remote has commits fetch would bring in
        diverged  the same, fetched, with a commit of our own on top
        outrun    the same, NOT fetched, with a commit of our own on top:
                  a plain push git will not send until the remote has been
                  read again (push-outdated)
        unpublished  a branch never sent anywhere, two remotes, one taken name
        forkmark  a fork checkout: fetches from origin, and the branch's
                  own mark sends every push of it to the fork instead
        protected an origin that keeps what it holds: every push to it is
                  turned away the way a forge turns one away from a
                  protected branch, and it holds a tag as well
                  (remote-refused, tag-refused)
        hooked    a pre-commit hook that says no, over something staged
                  for it to say it about (commit-refused, notice-over-diff)
        noremote  commits and no remote at all: the first push writes one down
        plan      a straight run of five commits, origin holding all but the
                  newest and a branch on the base: what the interactive-
                  rebase plan opens over (rebase-plan, rebase-plan-run,
                  rebase-edit-stop)
        signed    ssh-signed commits: verified, unjudgeable, unsigned
        errsig    an embedded OpenPGP signature no key can verify: the E
                  verdict (signature-tip passes only on this preset)
        co-authors  one, three and no co-authors, signed and not
        authorship  every way an author and a committer can be two
        tags      a tag in every state a remote can put it in (fetch first)
        manytags  basic, with more tags than any pane can show at once
        deep      more commits than the graph loads at once: the window
                  cut, which nothing shorter can show (graph-tail)
        deep-detached
                  deep, standing 800 commits down it: the only shape in
                  which the graph's HEAD stand-in rides the bottom edge,
                  and the only one where it is detached (graph-head-below)
        edges     every string at both ends of what git allows, in Japanese
        long      a message, a commit and a work tree that all run past
                  the pane they are shown in (80 files, 60-odd unstaged)
        longpaths one committed-then-changed file under a path wider than
                  any pane: both paths views elide it, and both tree
                  views elide its folder chain (path-tip [..-tree])
        manyhunks one unstaged file with 20 hunks in it: more diff than
                  window, which is the only shape that has a reading
                  place to keep through a rebuild (keep-place)
        widechars one unstaged file whose changed lines carry full-width
                  glyphs: the only shape in which a display column is not
                  one advance of the mono font, so the only one that says
                  whether the row's wash sits on its own characters
                  (diff-file columns.txt)
        widelines one unstaged file whose lines run far past the pane:
                  the only shape with anywhere sideways to go, so the
                  only one the diff's own bar comes out on (code-send)
        worktrees six linked working copies, one in each state git can
                  report: ordinary, detached, locked with a reason,
                  locked without one, one whose folder is gone, and one
                  whose folder and branch both run past the pane
        empty     `git init` and nothing else

";
