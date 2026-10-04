//! What the bundle is made of, as text: its name and identity, its
//! Info.plist, the name its archive goes by, and what `otool` and dyld say
//! about it. Nothing here runs a tool, so all of it is checked on any desk.

/// The identifier the release will carry (P5-確認事項 §8): reverse DNS
/// under the repository's owner, there being no domain of the project's.
const RELEASE_ID: &str = "io.github.k163377.platitude-gg";

/// The bundle's directory: the name the Finder shows (`CFBundleName` in
/// the template) and `.app`.
pub(super) const APP: &str = "Platitude GG.app";

/// A snapshot's identifier, beside the release's rather than the same: the
/// signature, the quarantine record and the privacy grants macOS keeps are
/// keyed by it, and a snapshot's are not the release's to inherit.
pub(super) fn snapshot_id() -> String {
    format!("{RELEASE_ID}.snapshot")
}

/// `packaging/macos/Info.plist` with its two placeholders filled. Only the
/// values are replaced (`<string>@…@</string>`), not the comment that
/// describes them, and a placeholder left over is refused: the template
/// is not a valid plist as it stands.
pub(super) fn info_plist(template: &str, version: &str, id: &str) -> Result<String, String> {
    let mut text = template.to_string();
    for (field, value) in [("@VERSION@", version), ("@BUNDLE_ID@", id)] {
        let slot = format!("<string>{field}</string>");
        if !text.contains(&slot) {
            return Err(format!("the Info.plist template has no {slot} to fill"));
        }
        text = text.replace(&slot, &format!("<string>{value}</string>"));
    }
    if let Some(at) = text.find("<string>@") {
        let rest = &text[at..];
        let slot = &rest[..rest.find("</string>").map_or(rest.len(), |end| end + 9)];
        return Err(format!(
            "the Info.plist template holds {slot}, which nothing here fills"
        ));
    }
    Ok(text)
}

/// The name a snapshot's archive goes by, short of its extension: what it
/// is, the day its commit was made, the commit, and where it runs — Apple
/// silicon only (CLAUDE.md).
pub(super) fn archive_stem(day: &str, commit: &str) -> String {
    format!("platitude-gg-snapshot-{day}-{commit}-macos-arm64")
}

/// The run paths a Mach-O carries, off `otool -l`: the `path` of each
/// `LC_RPATH` load command, in order.
pub(super) fn rpaths(load_commands: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_rpath = false;
    for line in load_commands.lines().map(str::trim) {
        if let Some(cmd) = line.strip_prefix("cmd ") {
            in_rpath = cmd.trim() == "LC_RPATH";
        } else if in_rpath && let Some(rest) = line.strip_prefix("path ") {
            let path = rest.rsplit_once(" (offset ").map_or(rest, |(path, _)| path);
            found.push(path.to_string());
            in_rpath = false;
        }
    }
    found
}

/// Where the images a run loaded came from, off the lines
/// `DYLD_PRINT_LIBRARIES` adds to its stderr.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Loaded {
    /// From inside the bundle.
    pub(super) inside: usize,
    /// Neither the bundle's nor the system's: what the bundle was supposed
    /// to carry and was found elsewhere.
    pub(super) outside: Vec<String>,
}

/// Where the operating system's own images sit. Everything else a bundle
/// loads has to come from inside it.
const SYSTEM: [&str; 3] = ["/System/", "/usr/lib/", "/Library/Apple/"];

/// Sorts the images dyld named for the process `pid` by where they came
/// from. `bundle` is the bundle's path as the run was started from it.
///
/// By the process, because the variable reaches what the app starts too:
/// git, whose own images are not the bundle's to carry.
pub(super) fn loaded(lines: &[String], bundle: &str, pid: u32) -> Loaded {
    let mut inside = 0;
    let mut outside = Vec::new();
    let images = lines
        .iter()
        .filter_map(|line| image_loaded(line))
        .filter(|(loader, _)| *loader == pid);
    for (_, image) in images {
        if image.starts_with(bundle) {
            inside += 1;
        } else if !SYSTEM.iter().any(|system| image.starts_with(system)) {
            outside.push(image.to_string());
        }
    }
    Loaded { inside, outside }
}

/// The process and the image of a line dyld writes as it loads one:
/// `dyld[<pid>]: <uuid> <path>` (dyld's `Loader::logLoad`, macOS 12 on —
/// below the floor Qt sets; an image without one says `<no uuid>`). Any
/// other line, dyld's own errors among them, names no image.
pub(super) fn image_loaded(line: &str) -> Option<(u32, &str)> {
    let (pid, rest) = line.strip_prefix("dyld[")?.split_once("]: ")?;
    let (_uuid, path) = rest.strip_prefix('<')?.split_once("> ")?;
    let pid = pid.parse().ok()?;
    path.starts_with('/').then_some((pid, path))
}

#[cfg(test)]
mod tests {
    use super::{APP, Loaded, archive_stem, image_loaded, info_plist, loaded, rpaths, snapshot_id};

    const TEMPLATE: &str = "<!--\n    @VERSION@    -- the version\n    @BUNDLE_ID@  -- the id\n-->\n\
        <dict>\n\t<key>CFBundleIdentifier</key>\n\t<string>@BUNDLE_ID@</string>\n\
        \t<key>CFBundleShortVersionString</key>\n\t<string>@VERSION@</string>\n\
        \t<key>CFBundleVersion</key>\n\t<string>@VERSION@</string>\n</dict>\n";

    #[test]
    fn the_plist_takes_the_version_and_the_id_and_keeps_its_comment() {
        let plist = info_plist(TEMPLATE, "0.1.0", &snapshot_id()).expect("filled");
        assert!(plist.contains("<string>io.github.k163377.platitude-gg.snapshot</string>"));
        assert_eq!(plist.matches("<string>0.1.0</string>").count(), 2);
        assert!(plist.contains("    @VERSION@    -- the version"), "{plist}");
        assert!(!plist.contains("<string>@"), "{plist}");
    }

    #[test]
    fn a_template_missing_a_slot_or_holding_another_is_refused() {
        let without = TEMPLATE.replace("<string>@BUNDLE_ID@</string>", "<string>x</string>");
        let refused = info_plist(&without, "0.1.0", "id").expect_err("no id slot");
        assert!(
            refused.contains("<string>@BUNDLE_ID@</string>"),
            "{refused}"
        );
        let more = TEMPLATE.replace("</dict>", "\t<string>@COPYRIGHT@</string>\n</dict>");
        let refused = info_plist(&more, "0.1.0", "id").expect_err("an unfilled slot");
        assert!(
            refused.contains("<string>@COPYRIGHT@</string>"),
            "{refused}"
        );
    }

    /// The real template is read by the run, on a Mac; here only its slots.
    #[test]
    fn the_checked_in_template_fills() {
        let root = crate::tree::workspace_root();
        let template =
            std::fs::read_to_string(root.join("packaging/macos/Info.plist")).expect("the template");
        let plist = info_plist(&template, "0.0.0", &snapshot_id()).expect("filled");
        assert!(plist.contains("<string>Platitude GG</string>"), "{plist}");
        assert_eq!(APP, "Platitude GG.app");
    }

    #[test]
    fn a_snapshot_is_named_so_and_is_not_the_release() {
        assert_eq!(snapshot_id(), "io.github.k163377.platitude-gg.snapshot");
        assert_eq!(
            archive_stem("20261005", "6690d93b"),
            "platitude-gg-snapshot-20261005-6690d93b-macos-arm64"
        );
    }

    #[test]
    fn the_run_paths_are_the_rpath_commands_paths() {
        let listed = "Load command 12\n          cmd LC_LOAD_DYLIB\n      cmdsize 88\n         \
            name @rpath/QtCore.framework/Versions/A/QtCore (offset 24)\n\
            Load command 26\n          cmd LC_RPATH\n      cmdsize 64\n         \
            path /Users/runner/work/_temp/qt/6.12.0/macos/lib (offset 12)\n\
            Load command 27\n          cmd LC_RPATH\n      cmdsize 48\n         \
            path @executable_path/../Frameworks (offset 12)\n\
            Load command 28\n          cmd LC_FUNCTION_STARTS\n      cmdsize 16\n";
        assert_eq!(
            rpaths(listed),
            [
                "/Users/runner/work/_temp/qt/6.12.0/macos/lib",
                "@executable_path/../Frameworks"
            ]
        );
        assert!(rpaths("Load command 1\n cmd LC_UUID\n").is_empty());
    }

    /// Lines as dyld writes them (`Loader::logLoad`: `"<%s> %s\n"` after
    /// the `dyld[%d]: ` every line of its log starts with).
    #[test]
    fn what_was_loaded_from_outside_is_named_and_the_system_is_not() {
        let bundle = "/Users/runner/work/_temp/package/Platitude GG.app";
        let lines: Vec<String> = [
            "dyld[812]: <9F848759-9AB8-3BD2-96A1-C069DC1FFD43> /Users/runner/work/_temp/package/Platitude GG.app/Contents/MacOS/platitude-gg",
            "dyld[812]: <0B5A1C2E-6F1D-3B7A-9C44-2D0E8F6A1B33> /Users/runner/work/_temp/package/Platitude GG.app/Contents/MacOS/../Frameworks/QtCore.framework/Versions/A/QtCore",
            "dyld[812]: <no uuid> /Users/runner/work/_temp/package/Platitude GG.app/Contents/PlugIns/platforms/libqoffscreen.dylib",
            "dyld[812]: <4C4C4436-5555-3144-A1C3-4E8A1C6B2F10> /usr/lib/libSystem.B.dylib",
            "dyld[812]: <7D1E2F30-4A5B-3C6D-8E9F-A0B1C2D3E4F5> /System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
            "dyld[812]: <1A2B3C4D-5E6F-3A1B-8C2D-3E4F5A6B7C8D> /Library/Apple/usr/lib/libexample.dylib",
            "dyld[812]: <2B3C4D5E-6F7A-3B2C-9D3E-4F5A6B7C8D9E> /Users/runner/work/_temp/qt/6.12.0/macos/lib/QtGui.framework/Versions/A/QtGui",
            "INFO build tree tree=- tag=v0.0.0",
            "dyld[812]: Library not loaded: @rpath/QtSvg.framework/Versions/A/QtSvg",
            // git, started by the app, with the variable inherited.
            "dyld[907]: <3C4D5E6F-7A8B-3C3D-8E4F-5A6B7C8D9E0F> /opt/homebrew/Cellar/git/2.55.0/bin/git",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            loaded(&lines, bundle, 812),
            Loaded {
                inside: 3,
                outside: vec![
                    "/Users/runner/work/_temp/qt/6.12.0/macos/lib/QtGui.framework/Versions/A/QtGui"
                        .to_string()
                ],
            }
        );
        assert_eq!(
            loaded(&lines, bundle, 1).inside,
            0,
            "another process's lines"
        );
        // dyld's own error is not a load, so the stand shows it.
        assert_eq!(image_loaded(&lines[8]), None);
        assert_eq!(image_loaded(&lines[7]), None);
        assert_eq!(
            image_loaded(&lines[3]),
            Some((812, "/usr/lib/libSystem.B.dylib"))
        );
    }
}
