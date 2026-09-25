//! The packages whose tests the container runs too. Read by the gate's
//! plan and by the sweep, so it reads nothing of either
//! (反映前テストの機械化.md §依存木).

/// The packages whose tests the container runs too: core and xtask, which
/// hold code behind `cfg(not(windows))` the host never compiles, and whose
/// tests run on the image's Qt-free stage. The app holds some as well;
/// before a merge Linux reaches it through `clippy-linux` (every target, so
/// its tests compile), the container's verbs, qmltest and `bare`, and its
/// own tests run there only in CI and `linux offline`. Read by the sweep as
/// well (`crate::sweep::tested_here`).
pub(crate) fn tested_on_linux(package: &str) -> bool {
    matches!(package, "platitude-core" | "xtask")
}
