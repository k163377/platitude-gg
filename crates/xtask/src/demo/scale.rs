//! Presets at the size and shape the panes have to survive: the measured
//! walls a ref and a path stop at, and a repository sprawling enough that
//! nothing shows whole.

use super::repo::{DemoRepo, file_url};

/// Bytes a ref's last component may have. Measured in throwaway
/// repositories on both systems: a loose ref is the file `<name>.lock`
/// against a 255-byte filename, so Linux stops at 250 — while NTFS counts
/// UTF-16 units and took 100 kanji (301 bytes) without complaint. The
/// smaller wall is the shared one, and a repository that holds a ref only
/// Windows can spell is a repository Linux cannot check out.
const REF_WALL: usize = 250;
/// Bytes a path component may have: 255 on both, and the worktree path
/// has to fit MAX_PATH under a temp directory, so files stay well inside.
const PATH_ROOM: usize = 120;

/// `head`, then kanji until the whole is exactly `bytes` long. The point
/// is the last character: it ends *on* the wall, so anything that slices
/// by byte index cuts it in half there — which is how this project lost
/// its graph once (core.md, the co-author key).
fn to_the_byte(head: &str, bytes: usize) -> String {
    const FILL: [char; 5] = ['長', 'い', '名', '前', 'の'];
    let mut s = String::from(head);
    let mut i = 0;
    while s.len() + 3 <= bytes {
        s.push(FILL[i % FILL.len()]);
        i += 1;
    }
    while s.len() < bytes {
        s.push('x');
    }
    s
}

/// A paragraph somebody pasted, in both scripts, *at least* `bytes` long.
/// git puts no wall in front of a subject, a body, an author name or a URL
/// — it took a megabyte of each in the same measurement — so what stands in
/// for "the limit" here is the worst thing a person plausibly does.
pub(crate) fn pasted(bytes: usize) -> String {
    let unit = "この行は長い日本語の文章で、折り返しと省略の両方を試すために置いてある。 \
                And an English clause rides along so the run of Latin text is measured too. ";
    let mut s = String::new();
    while s.len() < bytes {
        s.push_str(unit);
    }
    s.push_str("終端");
    s
}

/// The message a release actually gets written with: paragraphs, a list,
/// and a note in Japanese. Long enough that no pane shows it whole —
/// which is what the description box's grip is pulled for — but written
/// rather than repeated, because a wall of the same sentence tells you
/// nothing about how a real message wraps
/// (デザイン規約 §コミットメッセージの 2 つの枠).
///
/// Some paragraphs are wrapped at 72 columns and some are one long line,
/// because both turn up in real repositories — one from an editor, the
/// other pasted in — and they are the two things a box that wraps has to
/// be looked at doing.
const LONG_MESSAGE: &str = "\
refactor: move the whole store behind one interface

Every reader of the store used to reach into it its own way: some took a
lock and walked the map, some asked the index and then went back for the
row, and two of them cached what they found. That was fine while there
was one writer and nothing to invalidate, and it stopped being fine the
moment the background walk started landing rows while a pane was reading
them.

So the store has an interface now, and nothing outside it knows how the rows are held. What that costs is a hop through a trait object on every read; what it buys is that invalidation happens in one place, and that the next change to how rows are stored touches one file rather than eleven — this paragraph is one long line on purpose, the way a pasted one arrives, so the box is seen wrapping text that nobody wrapped for it.

The parts worth knowing about:

- Readers take a snapshot, not a lock. A snapshot is cheap (it clones a
  handle, not the rows) and it never blocks the writer, so a pane that
  is halfway through drawing cannot stall the walk that feeds it.
- Writes go through one method, which is also where the notification is
  raised. There is no way left to change a row without saying so.
- The two caches are gone. Both existed to skip a lookup that is now a
  vector index, and both had a way to go stale that nobody had noticed
  because the tests seeded them in order.
- The index is built once per batch instead of once per row. On the
  reference repository that is the difference between 40ms and 3ms, and
  it is the only reason this is worth doing at all rather than leaving
  the interface for later.

読み手が増えるたびに同じ罠を踏み直していたので、入口を 1 つにまとめた。
ここから先の変更は、行の持ち方を変えても呼ぶ側に出ない。逆に言えば、
この 1 ファイルの外に置き場所を作った時点で同じ話が戻ってくる。

What is deliberately not in here: the on-disk format is untouched, the
walk still produces rows in the same order, and nothing about how the
panes ask for a range has changed. Those are three separate arguments
and this commit is already the wrong size for having any of them in it.
";

/// Where the bulk of one commit lands, and how many files each place
/// takes. Spread over real-looking directories rather than one flat
/// heap: the file list is a tree first, and a tree of one folder is not
/// a tree.
const SPRAWL: [(&str, usize, &str); 6] = [
    ("src/core", 18, "rs"),
    ("src/ui", 14, "rs"),
    ("src/net", 9, "rs"),
    ("tests", 16, "rs"),
    ("docs", 12, "md"),
    ("assets/icons", 11, "svg"),
];

/// A commit nobody can read at a glance, over a tree nobody can scroll at
/// a glance, in a work tree of the same. Everything here is long on
/// purpose: the message runs past any pane, the commit touches 80 files,
/// and the working tree carries 60-odd changes of its own — which is the
/// state the description box's grip, the CHANGES list and the WIP list
/// are all hard to look at without.
pub(super) fn long(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA repository with more in every commit than fits on screen.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/core/store.rs", "// the store\n", "feat: add the store")?;
    repo.commit(
        "src/ui/pane.rs",
        "// a pane that reads the store\n",
        "feat: add a pane that reads it",
    )?;

    // A remote, and the ordinary history sent to it before the wall goes
    // on top: a repository with nothing to fetch from reads as a broken
    // window rather than as a preset about something else, and leaving
    // the wall unpushed is what makes sending 80 files something that
    // can be tried here.
    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;

    for (dir, count, ext) in SPRAWL {
        for i in 0..count {
            repo.write(
                &format!("{dir}/part_{i:02}.{ext}"),
                &format!("// {dir} part {i:02}, before the rewrite\n"),
            )?;
        }
    }
    repo.git(&["add", "--all"])?;
    repo.git(&["commit", "-m", LONG_MESSAGE])?;

    for (dir, count, ext) in SPRAWL {
        for i in (0..count).step_by(4).flat_map(|s| [s, s + 1, s + 2]) {
            if i >= count {
                continue;
            }
            repo.write(
                &format!("{dir}/part_{i:02}.{ext}"),
                &format!("// {dir} part {i:02}, after the rewrite\n\n// and a second thought\n"),
            )?;
        }
    }
    for i in 0..8 {
        repo.write(
            &format!("src/core/sketch_{i:02}.rs"),
            &format!("// sketch {i:02}, not added yet\n"),
        )?;
    }
    std::fs::remove_file(repo.work.join("src/net/part_08.rs"))
        .map_err(|e| format!("removing src/net/part_08.rs: {e}"))?;
    repo.git(&["mv", "docs/part_00.md", "docs/renamed.md"])?;
    Ok(())
}

/// Lines the file carries, and how far apart the changes in it sit. The
/// spacing is what makes them twenty hunks rather than fewer: git carries
/// three lines of context either side, so two changes fewer than eight
/// lines apart come out as one hunk with a gap in it.
const HUNK_LINES: usize = 320;
const HUNK_STEP: usize = 16;

/// The file, before and after: 320 lines with 20 of them reworded.
fn notes(changed: bool) -> String {
    let mut s = String::new();
    for i in 0..HUNK_LINES {
        if changed && i % HUNK_STEP == HUNK_STEP / 2 {
            s.push_str(&format!("{i:03}: reworded while reading down the file\n"));
        } else {
            s.push_str(&format!("{i:03}: a line of notes nobody has touched\n"));
        }
    }
    s
}

/// One unstaged file with twenty hunks in it, which is the only shape in
/// which a reader can have a place in a diff at all: a place exists where
/// there is more diff than window, and it can only be seen kept if a
/// partial write leaves enough behind to come back to (`keep-place`).
///
/// Plain text on purpose. The colours land after the rows and swap the
/// whole list a second time (`colour-place`), and that swap under this
/// one would take the view back to the top after the restore had put it
/// right — a different story, told by its own verb over its own fixture.
pub(super) fn manyhunks(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nA file too long to read at one sitting.\n",
        "docs: start the readme",
    )?;
    repo.commit("notes.txt", &notes(false), "docs: write the notes down")?;
    repo.write("notes.txt", &notes(true))?;
    Ok(())
}

/// How wide the lines of the `widelines` fixture run. Past any pane on any
/// screen this app is built for: the point of the preset is that the end of
/// the line cannot be brought on screen by making the window bigger.
const WIDE_COLUMNS: usize = 400;

/// One unstaged file whose lines run far past the pane they are shown in —
/// the only shape in which the diff has anywhere sideways to go, and so
/// the only one where the bar along its bottom edge and the hand that
/// moves it can be seen at all (`code-send`).
///
/// Both sides are wide: a change with a short old side would let the
/// reader send the new one out of the frame and leave the row half empty,
/// which says nothing about how far the diff reaches.
pub(super) fn widelines(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit(
        "README.md",
        "# demo\n\nLines nobody meant to be read at one sitting.\n",
        "docs: start the readme",
    )?;
    repo.commit("wide.txt", &wide("kept"), "docs: write the wide lines down")?;
    repo.write("wide.txt", &wide("torn"))?;
    Ok(())
}

/// `HUNK_LINES` of `WIDE_COLUMNS`, one in every `HUNK_STEP` of them
/// changed. The word that changes sits at the front, so the diff says what
/// it is about without being sent anywhere — what is out past the edge is
/// the rest of the line. ASCII throughout, so a column is a character.
///
/// As long as it is wide: the hand that sends the code sideways carries
/// the rows up and down at the same time, and a file that fits its pane
/// has no up or down to be carried through.
fn wide(word: &str) -> String {
    let mut out = String::new();
    for i in 0..HUNK_LINES {
        let said = if i % HUNK_STEP == HUNK_STEP / 2 {
            word
        } else {
            "kept"
        };
        let mut line = format!("{i:03}: {said} ");
        while line.len() < WIDE_COLUMNS {
            line.push_str("the line goes on ");
        }
        line.truncate(WIDE_COLUMNS);
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// One file under a path wider than any pane, committed and then changed
/// again: the flat paths views (the commit's file list and the working
/// tree's) each hold a row that middle-elides at any sane width, and the
/// tree views compact the same directories into one folder-chain row
/// that elides too — both the states the row's hover has to answer
/// (PG_AUTO_ACT=path-tip, with and without `-tree`). The dirty tree
/// keeps the graph shape fixed: row 0 is the WIP row, row 1 the commit
/// that holds the file.
pub(super) fn longpaths(repo: &mut DemoRepo) -> Result<(), String> {
    // Demo-repo content only: the path is made up and exists in no
    // checkout of this repository.
    const FAR: &str =
        "crates/platitude-core/src/session/integration/support/fixtures/refs_join_snapshot.rs";
    repo.commit(
        FAR,
        "// kept far down the tree\n",
        "feat: keep a fixture far down the tree",
    )?;
    repo.write(FAR, "// kept far down the tree\n// and changed\n")?;
    Ok(())
}

/// Every string the UI shows, at both ends of what git allows, with
/// Japanese in all of them. Two repositories in one: the short end is a
/// single character everywhere (including a commit with *no* message,
/// which git accepts and reads back empty), the long end sits on the
/// measured wall.
pub(super) fn edges(repo: &mut DemoRepo) -> Result<(), String> {
    let wall_branch = to_the_byte("b", REF_WALL);
    let wall_tag = to_the_byte("t", REF_WALL);
    let wall_file = to_the_byte("f", PATH_ROOM);
    let long_author = format!("{} <{}@example.com>", pasted(200), to_the_byte("m", 60));

    repo.commit(
        "README.md",
        "# 端\n\nちょうど端の値だけを集めたリポジトリ。\n",
        "docs: 端の値を集める",
    )?;

    repo.commit("q", "x\n", "日")?;
    // git takes a commit with no message at all and reads it back empty.
    repo.git(&["commit", "--allow-empty", "--allow-empty-message", "-m", ""])?;
    repo.git(&["commit", "--allow-empty", "--author=日 <あ>", "-m", "一"])?;

    // The body is longer than any pane is tall on purpose: the details
    // pane's description box can be pulled open by its corner, and a body
    // that runs out before the room does never reaches the bound that
    // pull stops at (デザイン規約 §コミットメッセージの 2 つの枠).
    let long_subject = pasted(2000);
    let long_body = format!(
        "{}\n\n{}\n\nCo-authored-by: {} <{}@example.com>\n",
        pasted(2000),
        pasted(3000),
        pasted(120),
        to_the_byte("c", 40)
    );
    repo.write(&wall_file, "端の名前のファイル\n")?;
    repo.git(&["add", "--", &wall_file])?;
    repo.git(&["commit", "-m", &long_subject, "-m", &long_body])?;

    repo.commit(
        "第一階層/第二階層/第三階層/第四階層/深い場所のファイル.txt",
        "deep\n",
        "feat: 深い階層にファイルを置く",
    )?;
    repo.commit("名前に 空白 が入る.txt", "space\n", "feat: 空白入りの名前")?;

    repo.git(&[
        "commit",
        "--allow-empty",
        &format!("--author={long_author}"),
        "-m",
        "chore: 著者名が段落のコミット",
    ])?;

    // The one-character branch is non-ASCII on purpose.
    repo.git(&["branch", "あ"])?;
    repo.git(&["branch", &wall_branch])?;
    repo.git(&["tag", "x"])?;
    repo.git(&["tag", "-a", &wall_tag, "-m", &pasted(300)])?;

    repo.add_origin()?;
    repo.git(&["push", "--set-upstream", "origin", "main"])?;
    repo.git(&["push", "origin", "あ", "x"])?;

    // The wall-length tag drifts away from the long-named remote's copy:
    // push it, then move the local one.
    //
    // Drift rather than remote-only, because a tag a fetch can reach is
    // one auto-follow brings down: only a tag on a commit nobody here
    // has stays away, and such a tag is on no row to hang a chip from
    // (core.md, タグのリモート状態のデータ).
    //
    // And no unreachable remote lives here: `fetch --prune --all` walks
    // every one of them, so a single absurd URL would turn every fetch
    // in this repository into a failure and every fetch verb into a
    // FAIL. The remote's name sits at the wall because a remote's name
    // is a ref component too.
    let long_remote = to_the_byte("r", REF_WALL);
    let bare = repo.root.join("long.git");
    std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
    repo.git_at(&bare.clone(), &["init", "--bare", "-b", "main"])?;
    let long_remote_url = file_url(&bare);
    repo.git(&["remote", "add", &long_remote, &long_remote_url])?;
    repo.git(&["push", &long_remote, "main", &wall_tag])?;
    repo.git(&[
        "commit",
        "--allow-empty",
        "-m",
        "chore: タグを動かす前の一手",
    ])?;
    repo.git(&["tag", "--force", &wall_tag])?;

    repo.write("q", "x\nstashed\n")?;
    repo.git(&["stash", "push", "-m", "日"])?;
    repo.write("q", "x\nstashed again\n")?;
    repo.git(&["stash", "push", "-m", &pasted(400)])?;

    repo.write(&wall_file, "端の名前のファイル\n編集した行\n")?;
    repo.git(&["add", "--", &wall_file])?;
    repo.write("名前に 空白 が入る.txt", "space\n編集\n")?;
    repo.write(&to_the_byte("u", PATH_ROOM), "untracked\n")?;
    repo.write("z", "")?;
    Ok(())
}
