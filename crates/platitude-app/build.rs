//! Hands the linker the Windows resource carrying what the exe says about
//! itself to the shell: its own icon, and the name shown in its place —
//! without the latter the shell falls back to `platitude-gg.exe`.
//!
//! `assets/platitude.res` is checked in rather than compiled here. The
//! resource compiler ships with the Windows SDK, and making every build
//! find it — including on the two platforms with no use for a `.res` —
//! buys nothing over a 21KB file in the tree. `assets/platitude.rc` says
//! how to rebuild it.
//!
//! The icon here is separate from the one the running window wears, which
//! `src/winframe.rs` sets at startup: one is read off the file by the
//! shell, the other off the window by the taskbar.

fn main() {
    println!("cargo:rerun-if-changed=assets/platitude.res");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Ok(root) = std::env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let res = std::path::Path::new(&root)
        .join("assets")
        .join("platitude.res");
    if res.is_file() {
        // Bins only: the test harnesses link without a resource.
        println!("cargo:rustc-link-arg-bins={}", res.display());
    } else {
        println!(
            "cargo:warning=assets/platitude.res is missing, so the exe keeps the shell's generic icon and is named after its own file"
        );
    }
}
