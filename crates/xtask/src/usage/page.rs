//! The usage page's lead, and the order its sections print in.

use super::{
    app_runs, container, demo_repo, landing, measurement, pictures, pre_merge, sessions, verify_ui,
};

pub(crate) fn print() {
    print!(
        "{HEAD}{}{}{}{}{}{}{}{}{}",
        pre_merge::ENTRIES,
        demo_repo::PRESETS,
        verify_ui::VERBS,
        app_runs::ENTRIES,
        measurement::ENTRIES,
        container::ENTRIES,
        landing::ENTRIES,
        pictures::ENTRIES,
        sessions::ENTRIES,
    );
}

const HEAD: &str = "\
cargo xtask <command>

commands:
";
