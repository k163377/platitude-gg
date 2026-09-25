//! Hands the linker the Windows resource with the exe's icon and the name
//! the shell shows for it (without it, `platitude-gg.exe`).
//!
//! `assets/platitude.res` is checked in compiled: the resource compiler
//! comes only with the Windows SDK. `assets/platitude.rc` says how to
//! rebuild it.
//!
//! The running window's icon is separate (`winframe::set_icon`): the shell
//! reads this one off the file, the taskbar that one off the window.

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
