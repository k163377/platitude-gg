//! Presets about signatures, and the key material a throwaway repository
//! can make for itself.

use std::path::Path;
use std::process::Command;

use super::repo::DemoRepo;

/// Every outcome the details pane can show, and a working tree set up to
/// make one more: a commit signed by a key this repository vouches for
/// (`G`), one signed by a key it has never heard of (`U` — measured, not
/// the `E` one might expect), and one not signed at all. SSH signing is
/// what a throwaway repository can do on its own: a passphrase-less key
/// needs no agent, so no pinentry can appear.
pub(super) fn signed(repo: &mut DemoRepo) -> Result<(), String> {
    repo.commit("a.txt", "v1\n", "feat: land before signing was on")?;

    let trusted = keygen(repo, "trusted", "demo@example.com")?;
    // Made, never vouched for: its public line goes nowhere.
    keygen(repo, "stranger", "stranger@example.com")?;

    // Only the first key is vouched for; the other one is a signature
    // git can read but cannot judge.
    let allowed = repo.root.join("allowed_signers");
    std::fs::write(&allowed, format!("demo@example.com {trusted}"))
        .map_err(|e| format!("writing allowed_signers: {e}"))?;
    for (key, value) in [
        ("gpg.format", "ssh".to_string()),
        ("gpg.ssh.allowedSignersFile", config_path(&allowed)),
        ("commit.gpgsign", "true".to_string()),
    ] {
        repo.git(&["config", key, &value])?;
    }

    let stranger_key = repo.root.join("stranger.pub");
    repo.git(&["config", "user.signingkey", &config_path(&stranger_key)])?;
    repo.commit("b.txt", "v1\n", "feat: signed by a key nobody vouched for")?;

    let trusted_key = repo.root.join("trusted.pub");
    repo.git(&["config", "user.signingkey", &config_path(&trusted_key)])?;
    repo.commit("c.txt", "v1\n", "feat: signed and verified")?;

    // Something to commit, so the commit button — which is where the
    // signing tick rides — is live and reachable.
    repo.write("a.txt", "v1\nabout to be committed\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    Ok(())
}

/// Writes a passphrase-less ed25519 key pair under the demo root and
/// returns the public key's one line.
pub(super) fn keygen(repo: &DemoRepo, name: &str, comment: &str) -> Result<String, String> {
    let path = repo.root.join(name);
    let out = crate::run_captured(
        Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&path),
    )?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::read_to_string(path.with_extension("pub"))
        .map(|k| k.trim().to_string())
        .map_err(|e| format!("reading {name}.pub: {e}"))
}

/// git reads config values with its own parser, which takes forward
/// slashes on every platform.
pub(super) fn config_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// The one verdict `signed` cannot stage: `E`, a signature git cannot
/// check. SSH signing never answers it — no allowedSignersFile is `N`,
/// a missing or short allowedSignersFile is `U`, tampered bytes and a
/// missing verifier are both `B` (all measured) — so the commit here is
/// OpenPGP-signed, and `E` is what gpg says when the public key is in
/// no keyring it can see. The key was made once, in a throwaway home
/// that no longer exists, and the signed object is carried whole below:
/// building needs no gpg and no key material, and no machine can hold
/// the key, so the verdict cannot drift to `G`. Verifying does need a
/// gpg binary — without one git calls the same commit unsigned
/// (measured: `N`) — which Git for Windows bundles and ci/linux
/// installs.
pub(super) fn errsig(repo: &mut DemoRepo) -> Result<(), String> {
    // The object names its tree, so the same tree is built first; the
    // hash checks prove nothing drifted, byte for byte.
    repo.write("a.txt", "one\n")?;
    repo.git(&["add", "--", "a.txt"])?;
    let tree = repo.git(&["write-tree"])?;
    if tree != ERRSIG_TREE {
        return Err(format!("errsig tree drifted: {tree}"));
    }
    let commit = repo.git_stdin(
        &["hash-object", "-w", "-t", "commit", "--stdin"],
        ERRSIG_OBJECT,
    )?;
    if commit != ERRSIG_COMMIT {
        return Err(format!("errsig commit drifted: {commit}"));
    }
    repo.git(&["update-ref", "refs/heads/main", &commit])?;
    repo.git(&["reset", "--hard"])?;
    Ok(())
}

const ERRSIG_TREE: &str = "20e50a07feffafe7699bf38ff4027a606f406eaa";
const ERRSIG_COMMIT: &str = "bdc88d46075d5f43d0f8b23a8f48d280769ff273";
/// `git cat-file commit` of the signed commit, escaped a line at a time
/// so the checkout's line endings cannot reach the bytes. The armour's
/// blank line really is `" "` — a space under the `gpgsig` header's
/// continuation indent.
const ERRSIG_OBJECT: &str = concat!(
    "tree 20e50a07feffafe7699bf38ff4027a606f406eaa\n",
    "author demo <demo@example.com> 1767323045 +0000\n",
    "committer demo <demo@example.com> 1767323045 +0000\n",
    "gpgsig -----BEGIN PGP SIGNATURE-----\n",
    " \n",
    " iIcEABYKAC8WIQQdamFUB//cf9AEH47UOMlB1A5vegUCankafxEcZGVtb0BleGFt\n",
    " cGxlLmNvbQAKCRDUOMlB1A5ven70AP9L7BWNVvo87cSiucHqL52AuGc6uD5BI/ad\n",
    " tIXQBS7ncgD9Gsrff1I162MIgFMh+Hr21cNHvfCKTdsPLb2BTp2r5w0=\n",
    " =OTHe\n",
    " -----END PGP SIGNATURE-----\n",
    "\n",
    "feat: sign with a key that is not shipped\n",
);
