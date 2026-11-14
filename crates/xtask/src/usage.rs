//! What `cargo xtask` prints when it is given no command it knows: the
//! lead here, then the entries of each stage of the work from a file of
//! its own, in the order printed.

mod app_runs;
mod container;
mod demo_repo;
mod landing;
mod measurement;
mod pictures;
mod pre_merge;
mod sessions;
mod verify_ui;

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
