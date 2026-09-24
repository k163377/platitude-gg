//! Presets about the two people a commit names: who wrote it, and who put
//! it here.

use super::repo::DemoRepo;
use super::signing::{config_path, keygen};

/// Every combination of "how many people the message credits" and
/// "is it signed", newest first: one co-author signed, three signed,
/// three unsigned, one unsigned, and a commit crediting nobody.
///
/// The spellings differ on purpose — `Co-Authored-By` is what the tooling
/// writes, `Co-authored-by` is what the convention documents — because
/// git's `key=` matches either, and so does the reader.
pub(super) fn co_authors(repo: &mut DemoRepo) -> Result<(), String> {
    const CROWD: &str = "feat: write this one with a crowd\n\n\
         The body sits above the trailers, the way it always does.\n\n\
         Co-authored-by: Claude Opus 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Fable 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Opus 4.8 <noreply@anthropic.com>";
    // The last address is deliberately long: an address has no length
    // worth trusting, and a card that sizes itself to one has to
    // elide.
    const CROWD_SIGNED: &str = "feat: write this one with a crowd, signed\n\n\
         Co-authored-by: Claude Opus 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Fable 5 <noreply@anthropic.com>\n\
         Co-authored-by: Claude Opus 4.8 \
         <an.address.long.enough.to.need.eliding@subdomain.example.co.jp>";
    const PAIR: &str = "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>";

    repo.commit(
        "README.md",
        "# demo\n\nA repository with credited company.\n",
        "docs: start the readme",
    )?;
    repo.commit(
        "src/pair.txt",
        "written by two\n",
        &format!("feat: write this one with company\n\n{PAIR}"),
    )?;
    repo.commit("src/crowd.txt", "written by four\n", CROWD)?;

    // A key this repository vouches for, so the commits signed with it
    // verify.
    let trusted = keygen(repo, "trusted", "demo@example.com")?;
    let allowed = repo.root.join("allowed_signers");
    std::fs::write(&allowed, format!("demo@example.com {trusted}"))
        .map_err(|e| format!("writing allowed_signers: {e}"))?;
    let trusted_key = repo.root.join("trusted.pub");
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("gpg.ssh.allowedSignersFile", config_path(&allowed)),
        ("user.signingkey", config_path(&trusted_key)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }

    repo.commit(
        "src/crowd.txt",
        "written by four, and signed\n",
        CROWD_SIGNED,
    )?;

    // A name long enough to crowd the mark it sits next to: the name is
    // the side that gives way, so the tick stays on screen.
    repo.write("src/long.txt", "written under a long name\n")?;
    repo.git(&["add", "--", "src/long.txt"])?;
    repo.git(&[
        "commit",
        "--author=Alexander Kirillov-Petrov <alexander@example.com>",
        "-m",
        &format!("feat: write this one under a long name\n\n{PAIR}"),
    ])?;

    // Signed with a key nobody vouched for: git reads the signature and
    // cannot judge it. Every SSH signature falls into this state when no
    // allowedSigners file is configured at all, so it is ordinary.
    keygen(repo, "stranger", "stranger@example.com")?;
    let stranger_key = repo.root.join("stranger.pub");
    repo.git(&["config", "user.signingkey", &config_path(&stranger_key)])?;
    repo.commit(
        "src/unjudged.txt",
        "signed by a stranger\n",
        &format!("feat: write this one signed by an unvouched key\n\n{PAIR}"),
    )?;
    repo.git(&["config", "user.signingkey", &config_path(&trusted_key)])?;

    // A signature that no longer matches what it signed: sign properly,
    // then swap the tree underneath.
    repo.commit(
        "src/tampered.txt",
        "before\n",
        &format!("feat: write this one and then tamper with it\n\n{PAIR}"),
    )?;
    repo.write("src/tampered.txt", "after the signature was made\n")?;
    repo.git(&["add", "--", "src/tampered.txt"])?;
    let swapped_tree = repo.git(&["write-tree"])?;
    let tampered = retree_head(repo, &swapped_tree)?;
    repo.git(&["update-ref", "refs/heads/main", &tampered])?;
    repo.git(&["reset", "--hard", "HEAD"])?;

    repo.commit(
        "src/pair.txt",
        "written by two, and signed\n",
        &format!("feat: write this one with company, signed\n\n{PAIR}"),
    )?;
    Ok(())
}

/// Every way the person who wrote a commit and the person who put it
/// here can be two, newest first: a patch applied by somebody else days
/// after it was written, one applied by somebody else the moment it
/// arrived (a squash merge on a forge, so a bot wrote it and carries a
/// forge noreply for an address), one the same hand committed
/// later than it wrote it (an amend, a rebase), and two ordinary
/// commits, where the two are one person at one moment.
pub(super) fn authorship(repo: &mut DemoRepo) -> Result<(), String> {
    const MAILED: &str = "Yuki Tanaka <yuki.tanaka@example.com>";
    // What a forge puts here is usually a bot's work, and a bot's address
    // is a forge noreply — the long kind. The length is deliberate, the
    // way the co-authors' last address is: the card that reads an address
    // in full is where a long one has to hold, and the pane it opens over
    // is narrower than this.
    const BOT: &str = "deps-bot[bot] <49206153+deps-bot[bot]@users.noreply.forge.example.co.jp>";
    const DAY: u64 = 24 * 60 * 60;

    repo.commit(
        "README.md",
        "# demo\n\nA repository where the credit is split.\n",
        "docs: start the readme",
    )?;
    repo.commit("src/app.txt", "app v1\n", "feat: add the app")?;
    repo.commit_written_earlier(
        "src/app.txt",
        "app v1, and it waits for slow disks\n",
        "fix: hold the door open for slow disks",
        None,
        2 * DAY,
    )?;
    repo.write("src/merged.txt", "came in through the web\n")?;
    repo.git(&["add", "--", "src/merged.txt"])?;
    repo.git(&[
        "commit",
        &format!("--author={BOT}"),
        "-m",
        "feat: take the config out into a file",
    ])?;
    repo.commit_written_earlier(
        "src/mailed.txt",
        "arrived as a patch\n",
        "perf: stop reading the index twice",
        Some(MAILED),
        3 * DAY,
    )?;
    Ok(())
}

/// Rewrites HEAD's commit object with a different tree, keeping
/// every other header — including the signature, which is what
/// makes the result read as broken (measured: `%G?` goes from `G`
/// to `B`).
fn retree_head(repo: &mut DemoRepo, tree: &str) -> Result<String, String> {
    let dir = repo.work.clone();
    let out = super::repo::output_of(&mut repo.command(&dir, &["cat-file", "commit", "HEAD"]))?;
    if !out.status.success() {
        return Err("git cat-file commit HEAD failed".to_string());
    }
    let patched = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| {
            if line.starts_with("tree ") {
                format!("tree {tree}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let path = repo.root.join("tampered-commit");
    std::fs::write(&path, patched).map_err(|e| format!("writing tampered commit: {e}"))?;
    repo.git(&["hash-object", "-w", "-t", "commit", &config_path(&path)])
}
