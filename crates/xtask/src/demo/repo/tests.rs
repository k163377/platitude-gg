//! The stamps a demo repository's commits carry, read back from git.

use super::DemoRepo;

#[test]
fn a_commit_written_earlier_is_authored_exactly_that_long_before_it_is_committed() {
    let root = crate::verify::claim_dir(&std::env::temp_dir().join("pgg-demo"), "earlier")
        .expect("a directory of its own");
    let earlier = 2 * 24 * 3600;
    let mut repo = DemoRepo::init(&root, "repo").expect("a repository");
    repo.commit_written_earlier("a.txt", "x\n", "m", None, earlier)
        .expect("a commit");
    let stamps = repo
        .git(&["log", "-1", "--format=%at %ct"])
        .expect("the commit's stamps");
    std::fs::remove_dir_all(&root).expect("the directory, cleared");
    let [authored, committed] = [0, 1].map(|at| {
        stamps
            .split(' ')
            .nth(at)
            .and_then(|stamp| stamp.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("two stamps, not {stamps:?}"))
    });
    assert_eq!(committed - authored, earlier, "{stamps}");
}
