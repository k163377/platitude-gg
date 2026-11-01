//! The mark a container's processes carry. Nothing else of `linux` belongs here: a reader of the mark
//! (`qmltest`, `sweep`, a verify run) is keyed on this file alone, not on how a container is started or built.

/// Set for everything the container runs, and by nothing else: the mark a
/// run reads to know it is not on the machine whose checkout it is writing.
pub(crate) const IN_CONTAINER: &str = "PGG_IN_CONTAINER";
