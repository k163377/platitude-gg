//! `cargo xtask film <in|out|shoot|bake|changes>`.

use crate::tree::workspace_root;

const USAGE: &str = "\
cargo xtask film <command>   (the film skill has the scene syntax and the order of a film)

  in        Put the film verb into this tree: a QML component, its line in
            the qmldir, its embed, and the harness's slots — never
            committed (a commit is refused while it stands).
  out       Take it out again, by the same substitutions backwards.
  shoot <scene>... [--side <side>] [--no-build] [--fresh-base]
            One side of each scene (`now` unless --side says before, main,
            after, opt2 or opt3), headless, into target/film/<scene>/. The
            first scene builds the release; --no-build skips that too.
            --fresh-base builds the scene's repository again rather than
            copying the one standing.
  bake <scene> [--sides <side>,<side>]
            The sides shot (or those named) as target/film/<scene>/<side>.png,
            APNGs on one clock, and the last frame under each caption.
  changes <scene> [--side <side>]
            Each frame whose picture changed from the one before, with the
            box that changed.
";

pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let tree = workspace_root();
    let rest = args.get(1..).unwrap_or_default();
    match args.first().map(String::as_str) {
        Some("in") => super::inject::put_in(&tree),
        Some("out") => super::inject::take_out(&tree),
        Some("shoot") => super::shoot::run(&tree, &super::shoot::parse(rest)?),
        Some("bake") => super::bake::run(&tree, rest),
        Some("changes") => super::changes::run(&tree, rest),
        _ => {
            print!("{USAGE}");
            Err("film needs a command".into())
        }
    }
}
