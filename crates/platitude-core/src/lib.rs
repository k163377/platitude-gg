//! Core domain logic for platitude-gg: git subprocess execution, output
//! parsing, commit-graph computation and the domain model.
//!
//! This crate must stay free of Qt (and any bridge) dependencies so that it
//! can be unit-tested on any machine and reused unchanged if the QML bridge
//! implementation is swapped (Qt Bridges -> CXX-Qt).
