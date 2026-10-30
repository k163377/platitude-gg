//! The execution context shared by the container launcher and commands it runs.
//! A child reading this protocol does not depend on how the host starts or builds a container.

/// Set for everything the container runs, and by nothing else: the mark a
/// run reads to know it is not on the machine whose checkout it is writing.
pub(crate) const IN_CONTAINER: &str = "PGG_IN_CONTAINER";
