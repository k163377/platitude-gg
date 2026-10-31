//! Agent controls owe their tests without owing runs of the app.

use super::{Ask, Composition, Lines, Reading, Tiers, graph, select};

#[test]
fn agent_controls_owe_tests_but_no_app_runs() {
    let dir = crate::tree::workspace_root();
    let g = graph::build(&dir).expect("the current source graph");
    let lines = Lines::load(&dir).expect("registered lines");
    let tiers = Tiers::load(&dir).expect("line tiers");
    let shown = lines.shown(&Composition::read(&dir).expect("composition"));
    let read = Reading {
        dir: &dir,
        lines: &lines,
        added: &[],
        shown: &shown,
        tiers: &tiers,
        everything: false,
        whole_tier: false,
    };
    let ask = Ask {
        main_ref: "main",
        host_only: false,
        whole: false,
        fresh: false,
        extra_verbs: &[],
    };
    // Split paths keep these test subjects out of the graph's literal reads.
    let src = format!("crates/{}/src", "xtask");
    for file in [
        format!("{src}/awake/codex.rs"),
        format!("{src}/hook/awake.rs"),
        format!("{src}/hook/language.rs"),
        format!("{}/{}", ".codex", "hooks.json"),
        format!("{}/{}", ".claude", "settings.json"),
    ] {
        for os in ["windows", "linux"] {
            let changed = vec![file.clone()];
            let reach = g.reach_on(&changed, os);
            let (steps, _) = select(&g, &read, &reach, &changed, &ask);
            let app: Vec<_> = steps
                .iter()
                .filter(|s| s.builds_app)
                .map(|s| &s.id)
                .collect();
            assert!(app.is_empty(), "{file} on {os}: {app:?}");
            if file.ends_with(".rs") {
                assert!(
                    steps.iter().any(|s| s.id.starts_with("test xtask")),
                    "{file} on {os}: {steps:?}"
                );
            }
        }
    }
    // The same selector still owes app runs when its driver changes.
    let changed = vec![format!("{src}/verify/options.rs")];
    let (steps, _) = select(&g, &read, &g.reach(&changed), &changed, &ask);
    assert!(steps.iter().any(|s| s.builds_app));
}
