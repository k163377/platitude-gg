//! The everyday presets: a repository with ordinary history, and the
//! small states around it.

use super::repo::DemoRepo;

/// Branches, a remote one commit behind, tags in both places, a stash,
/// and a dirty working tree — the state most verbs can act on.
pub(super) fn basic(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository for kicking tires.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit("src/lib.txt", "lib v1\n", "feat: add the library")?;
    repo.commit(
        "src/app.txt",
        "app v1\nwith settings\n",
        "feat: read settings",
    )?;
    repo.git(&["tag", "-a", "v0.1", "-m", "first cut"])?;
    repo.commit("docs/guide.md", "guide v1\n", "docs: add a guide")?;
    // Japanese subject and body on purpose: every screenshot of this
    // preset then exercises CJK rendering, and 直 / 骨 make a wrong
    // (Chinese-variant) glyph visible at a glance.
    repo.commit(
        "docs/guide.md",
        "guide v1\n\n## 使い方\n直感的な操作の案内。骨組みだけ先に日本語で書く。\n",
        "docs: 利用案内の骨子を日本語で直す",
    )?;
    repo.commit("src/lib.txt", "lib v2\n", "fix: harden the library")?;

    repo.git(&["switch", "--create", "feature/topic-a"])?;
    repo.commit("src/topic.txt", "topic draft\n", "feat: draft the topic")?;
    repo.commit("src/topic.txt", "topic ready\n", "feat: finish the topic")?;
    repo.git(&["switch", "main"])?;

    repo.commit(
        "src/app.txt",
        "app v2\nwith settings\n",
        "feat: rework the app",
    )?;
    repo.git(&["tag", "v0.2"])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "feature/topic-a", "v0.1", "v0.2"])?;

    // A branch that exists only on the remote: push it, drop the local.
    repo.git(&["switch", "--create", "feature/remote-only", "main"])?;
    repo.commit("src/remote.txt", "remote work\n", "feat: work kept remote")?;
    repo.git(&["push", "origin", "feature/remote-only"])?;
    repo.git(&["switch", "main"])?;
    repo.git(&["branch", "-D", "feature/remote-only"])?;

    // Ahead of origin/main by one, so push has something to do.
    repo.commit("docs/guide.md", "guide v2\n", "docs: extend the guide")?;
    // A tag only this clone has.
    repo.git(&["tag", "v0.3-local"])?;

    repo.write("src/lib.txt", "lib v2\nstashed experiment\n")?;
    repo.git(&["stash", "push", "-m", "experiment on the library"])?;
    // A dirty working tree: staged, unstaged, untracked. The
    // unstaged edit REPLACES the first line, so the diff's body line 0 is
    // a change — the stage-line / discard-line hooks pick line 0, and a
    // context line there would select nothing.
    repo.write("src/app.txt", "app v2\nwith settings\nstaged line\n")?;
    repo.git(&["add", "--", "src/app.txt"])?;
    repo.write("docs/guide.md", "guide v2, reworded\nunstaged line\n")?;
    repo.write("notes.txt", "untracked scratch\n")?;
    Ok(())
}

/// Every WIP bucket at once: staged, unstaged, staged+unstaged on one
/// file, a staged rename, and an untracked file.
pub(super) fn dirty(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "a v1\n", "feat: a")?;
    repo.commit("b.txt", "b v1\n", "feat: b")?;
    repo.commit("c.txt", "c v1\n", "feat: c")?;
    repo.commit(
        "renamed-from.txt",
        "moves around\n",
        "feat: file that moves",
    )?;

    repo.write("a.txt", "a v1\nstaged\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    repo.write("b.txt", "b v1\nunstaged\n")?;
    repo.write("c.txt", "c v1\nstaged\n")?;
    repo.git(&["add", "--", "c.txt"])?;
    repo.write("c.txt", "c v1\nstaged\nand unstaged on top\n")?;
    repo.git(&["mv", "renamed-from.txt", "renamed-to.txt"])?;
    // Long enough to be read as a file rather than as a line: the diff of
    // something the repository has never seen is all additions under one
    // heading, and one line of it cannot show what that looks like.
    repo.write(
        "untracked.txt",
        "Notes for the next release\n\
         \n\
         - decide what goes in the first tag\n\
         - write the install steps down\n\
         - check the licence headers\n\
         - measure the cold start once more\n\
         \n\
         Nothing here is recorded yet.\n",
    )?;
    Ok(())
}

/// All four line-ending cases at once, in a directory with a house style.
///
/// The neighbours are what makes the two estimated cases sayable: three
/// usable votes are needed, and `flipped.kt` (CRLF in the work tree),
/// `mixed.kt` (mixed) and `bare.kt` (no ending at all) can none of them
/// vote. The four plain ones sort ahead of `flipped.kt`, so the sample is
/// unanimous LF and the notice may say "here".
pub(super) fn eol(repo: &mut DemoRepo) -> Result<(), String> {
    for name in ["alpha", "beta", "delta", "gamma"] {
        repo.commit(
            &format!("src/{name}.kt"),
            &format!("fun {name}() = \"{name}\"\n"),
            &format!("feat: {name}"),
        )?;
    }
    repo.commit(
        "src/flipped.kt",
        "fun one() = 1\nfun two() = 2\nfun three() = 3\n",
        "feat: three of them",
    )?;
    // Long enough that the untouched lines outnumber the changed one, so
    // the notice can name what the file uses.
    repo.commit(
        "src/mixed.kt",
        "fun a() = 1\nfun b() = 2\nfun c() = 3\nfun d() = 4\nfun e() = 5\n",
        "feat: five of them",
    )?;
    repo.commit("src/bare.kt", "fun bare() = 0", "feat: no ending at all")?;
    // An ordinary change with nothing to say, so "a file is marked" and
    // "this commit carries a marked file" can be told apart.
    repo.commit("src/plain.kt", "fun plain() = 1\n", "feat: an ordinary one")?;

    // (a) every line's ending changes, and nothing else does.
    repo.write(
        "src/flipped.kt",
        "fun one() = 1\r\nfun two() = 2\r\nfun three() = 3\r\n",
    )?;
    // (b) one line lands with the other ending.
    repo.write(
        "src/mixed.kt",
        "fun a() = 1\nfun b() = 2\nfun c() = 3\r\nfun d() = 4\nfun e() = 5\n",
    )?;
    // (d) the file that had none gains its first.
    repo.write("src/bare.kt", "fun bare() = 0\r\n")?;
    repo.write("src/plain.kt", "fun plain() = 1\nfun alsoPlain() = 2\n")?;
    // (c) a file the repository has never seen, disagreeing with its
    // neighbours.
    repo.write(
        "src/fresh.kt",
        "fun fresh() = \"new\"\r\nfun alsoFresh() = \"new\"\r\n",
    )?;
    Ok(())
}

/// Commits and nowhere to send them: no remote at all. The first push
/// cannot even pick a destination here — the remote dialog opens by
/// itself, name prefilled `origin`, with the question standing behind it.
pub(super) fn noremote(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    Ok(())
}

/// Three stashes; the newest carries an untracked file.
pub(super) fn stashes(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("work.txt", "v1\n", "feat: work")?;
    repo.commit("other.txt", "v1\n", "feat: other")?;
    repo.write("work.txt", "v1\nfirst try\n")?;
    repo.git(&["stash", "push", "-m", "first try"])?;
    repo.write("other.txt", "v1\nsecond angle\n")?;
    repo.git(&["stash", "push", "-m", "second angle"])?;
    repo.write("work.txt", "v1\nthird pass\n")?;
    repo.write("sketch.txt", "untracked sketch\n")?;
    repo.git(&[
        "stash",
        "push",
        "--include-untracked",
        "-m",
        "third, with a sketch",
    ])?;
    Ok(())
}

/// A straight run for the interactive-rebase plan: five steps on main
/// with origin holding all but the newest, a branch standing on the
/// commit the default plan lands on, and a clean tree. Linear on
/// purpose — the plan refuses a range with a merge in it, and the
/// verbs walk down from HEAD by row, which only a branchless stretch
/// keeps meaning "this branch's own history".
pub(super) fn plan(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("notes.txt", "v1\n", "docs: start the notes")?;
    repo.commit("src/one.txt", "one\n", "feat: the first step")?;
    repo.commit("src/two.txt", "two\n", "feat: the second step")?;
    repo.commit("src/three.txt", "three\n", "feat: the third step")?;
    repo.commit("src/four.txt", "four\n", "feat: the fourth step")?;
    repo.git(&["branch", "base", "HEAD~3"])?;
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    // Ahead by one, so the plan's newest row is this clone's own.
    repo.commit("src/five.txt", "five\n", "feat: the newest step")?;
    Ok(())
}

/// HEAD detached at a tagged commit.
pub(super) fn detached(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: one")?;
    repo.commit("a.txt", "v2\n", "feat: two")?;
    repo.commit("a.txt", "v3\n", "feat: three")?;
    repo.git(&["tag", "v1.0", "HEAD~1"])?;
    repo.commit("a.txt", "v4\n", "feat: four")?;
    repo.git(&["switch", "--detach", "v1.0"])?;
    Ok(())
}

/// A merge inside the stretch a rewrite of the newest commit would
/// replay, so a fold or a drop of the tip is turned down before git is
/// asked (`report::rewrite_across_merge`).
pub(super) fn rewrite_merge(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("README.md", "# demo\n", "docs: start the readme")?;
    repo.git(&["switch", "--create", "side"])?;
    repo.commit("side.txt", "side v1\n", "feat: work on the side")?;
    repo.git(&["switch", "main"])?;
    repo.commit("main.txt", "main v1\n", "feat: work on main")?;
    repo.git(&["merge", "--no-ff", "--no-edit", "side"])?;
    repo.commit("main.txt", "main v2\n", "feat: carry on after the merge")?;
    Ok(())
}

/// One commit on the branch: it is the first as well as the newest, so a
/// fold has nothing to fold into and a drop would take the whole history
/// with it (`report::fold_first_commit` / `report::drop_all_commits`).
///
/// `apart` holds one more that main never took, committed last so it is
/// the newest of the two and the graph draws it first: the third refusal,
/// about a commit the current branch cannot see
/// (`report::rewrite_off_branch`). **It belongs to a history with no
/// merge in it** — a merge anywhere in the branch answers first, since
/// the range a rewrite would replay is read before the commit is looked
/// for in it (実測).
pub(super) fn one_commit(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nJust the one.\n",
        "docs: start the readme",
    )?;
    repo.git(&["switch", "--create", "apart"])?;
    repo.commit("apart.txt", "apart v1\n", "feat: work nobody took")?;
    repo.git(&["switch", "main"])?;
    Ok(())
}
