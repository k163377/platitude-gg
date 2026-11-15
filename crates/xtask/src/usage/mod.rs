//! What `cargo xtask` prints when it is given no command it knows: the
//! lead and the order the sections print in (`page`), and the entries of
//! each stage of the work from a file of its own.

mod app_runs;
mod container;
mod demo_repo;
mod landing;
mod measurement;
mod page;
mod pictures;
mod pre_merge;
mod sessions;
mod verify_ui;

pub(crate) use page::print;
