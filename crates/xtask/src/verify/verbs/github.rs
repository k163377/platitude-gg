//! The links to GitHub's pages (デザイン規約 §GitHub のページへのリンク),
//! on `--preset github`. A link offered for a remote that lacks the
//! thing, or missing for one that has it, draws like the right card: the
//! line names which remote leads each link and which wait behind.

use super::{Arg, Verb};

pub(super) const TABLE: &[Verb] = &[
    // Row 0 is `main`, on no remote: no commit link, and the branch on
    // both under the name its upstream has. Row 3 is the fix only
    // upstream has, its chip `upstream/main`: both links lead with
    // upstream — the commit because origin lacks it, the branch because
    // the chip names that remote. Row 5 is the tag both remotes carry.
    Verb {
        name: "github-card",
        when: &[
            (
                Arg::Is("0"),
                "github_card opened=true leads=[branch@origin] others=[branch@upstream]",
            ),
            (
                Arg::Starts("3"),
                "github_card opened=true leads=[commit@upstream,branch@upstream] others=[branch@origin]",
            ),
            (
                Arg::Starts("5"),
                "github_card opened=true leads=[commit@origin,tag@origin] others=[commit@upstream,tag@upstream]",
            ),
        ],
        plain: "github_card opened=true",
    },
    // A right-click inside the choice of two leaves it standing and opens
    // the pair's menu — the card opens only off that menu. Rows 1 and 4:
    // upstream lacks the newer, so only origin compares them.
    Verb {
        name: "github-choice",
        when: &[(
            Arg::Starts("1:4"),
            "github_choice opened=true leads=[compare@origin] others=[]",
        )],
        plain: "github_choice opened=true",
    },
    // The selected lines of a commit's file, on the remote that has the
    // commit.
    Verb {
        name: "diff-github",
        when: &[],
        plain: "diff_github opened=true leads=[lines@origin] others=[]",
    },
    // The left menu's rows offer what a graph row's chip does, on the
    // commit the ref points at: `feature/login` is origin's alone,
    // `upstream/main` leads with its own remote (as row 3), the tag is on
    // both (as row 5).
    Verb {
        name: "github-ref",
        when: &[
            (
                Arg::Starts("branch:feature/login"),
                "github_ref opened=true leads=[commit@origin,branch@origin] others=[]",
            ),
            (
                Arg::Starts("remote:upstream/main"),
                "github_ref opened=true leads=[commit@upstream,branch@upstream] others=[branch@origin]",
            ),
            (
                Arg::Starts("tag:v1.0"),
                "github_ref opened=true leads=[commit@origin,tag@origin] others=[commit@upstream,tag@upstream]",
            ),
        ],
        plain: "github_ref opened=true",
    },
    // A REMOTES row offers that remote's repository, and no other's.
    Verb {
        name: "github-remote",
        when: &[(
            Arg::Starts("upstream"),
            "github_remote opened=true leads=[repo@upstream] others=[]",
        )],
        plain: "github_remote opened=true leads=[repo@origin] others=[]",
    },
    // A commit's changed file, on the remotes that have the commit: row 4
    // is on both, row 1 on origin alone. The right-click reads the file
    // first — the menu acts on what is lit.
    Verb {
        name: "github-file",
        when: &[
            (
                Arg::Starts("4:src/app.txt"),
                "github_file opened=true leads=[file@origin] others=[file@upstream]",
            ),
            (
                Arg::Starts("1:src/login.txt"),
                "github_file opened=true leads=[file@origin] others=[]",
            ),
        ],
        plain: "github_file opened=true",
    },
];
