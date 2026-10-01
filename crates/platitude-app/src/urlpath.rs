//! `file://` URL ↔ local path. The conversions are Qt's (`QUrl`) — the
//! same the platform dialogs answer with and QML's `Image` reads. What is
//! this app's own is the rest: a path with no root has no URL, the picker
//! opens beside the repository, and the screen spells a path with `/`.

use std::path::{Path, PathBuf};

use cxx_qt_lib::{QString, QUrl};

/// The local path a `file:` URL names: a dialog's answer, or a URL this
/// side made. Anything that names no local file is empty — none of the
/// slots that take one is handed a bare path.
pub fn file_url_to_path(url: &str) -> PathBuf {
    QUrl::from(url)
        .to_local_file()
        .map_or_else(PathBuf::new, |path| PathBuf::from(path.to_string()))
}

/// The folder the picker opens at for a repository at `path`: the one it
/// sits in, or the repository itself when it is a root.
pub fn picker_folder_url(path: &Path) -> String {
    path_to_file_url(path.parent().unwrap_or(path))
}

/// A `file:` URL for a file QML is to load.
pub fn file_url(path: &Path) -> String {
    path_to_file_url(path)
}

/// The last segment of a path, whichever separator wrote it.
pub fn path_leaf(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A path spelled with `/`, the way the screen spells one
/// (デザイン規約 §パスの区切り).
///
/// Folds only on Windows: elsewhere a backslash is a letter of the name.
/// The `\\?\` / `\\.\` prefixes are left alone, as `repo_key` leaves them.
pub fn shown_path(path: &str) -> String {
    #[cfg(windows)]
    if !path.starts_with(r"\\?\") && !path.starts_with(r"\\.\") {
        return path.replace('\\', "/");
    }
    path.to_string()
}

/// Encoded, so a query the caller appends (`?read=`) stays one. Empty for
/// a path with no root: a dialog cannot be opened at one.
fn path_to_file_url(path: &Path) -> String {
    if !path.has_root() {
        return String::new();
    }
    let url = QUrl::from_local_file(&QString::from(path.to_string_lossy().as_ref()));
    String::from_utf8_lossy(url.to_encoded().as_slice()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A URL as a dialog answers with it: QML's `selectedFile.toString()`.
    fn dialog_url(path: &Path) -> String {
        QUrl::from_local_file(&QString::from(path.to_string_lossy().as_ref()))
            .to_qstring()
            .to_string()
    }

    #[test]
    fn a_path_is_shown_with_the_separator_git_answers_with() {
        assert_eq!(shown_path("C:/Users/dev/repo"), "C:/Users/dev/repo");
        assert_eq!(shown_path("/home/dev/repo"), "/home/dev/repo");
    }

    #[cfg(windows)]
    #[test]
    fn a_windows_path_is_respelled_for_the_screen() {
        assert_eq!(shown_path(r"C:\Users\dev\repo"), "C:/Users/dev/repo");
    }

    /// Respelling the prefix addresses somewhere else.
    #[cfg(windows)]
    #[test]
    fn a_verbatim_prefix_keeps_the_backslashes_it_is_made_of() {
        assert_eq!(shown_path(r"\\?\C:\repo"), r"\\?\C:\repo");
        assert_eq!(shown_path(r"\\.\C:\repo"), r"\\.\C:\repo");
    }

    #[cfg(not(windows))]
    #[test]
    fn a_backslash_is_left_where_it_is_part_of_a_name() {
        assert_eq!(shown_path(r"/home/dev/we\ird"), r"/home/dev/we\ird");
    }

    #[test]
    fn the_leaf_is_the_last_segment_whichever_separator_wrote_it() {
        assert_eq!(path_leaf("C:\\Users\\dev\\repo"), "repo");
        assert_eq!(path_leaf("/home/dev/repo"), "repo");
        assert_eq!(path_leaf("repo"), "repo");
    }

    /// What a dialog answers reads back to the path it was opened on —
    /// whatever the name holds.
    #[test]
    fn a_dialog_s_answer_reads_back_to_its_path() {
        let mut paths = vec![
            "/with space/repo",
            "/日本語/repo",
            "/50%off/repo",
            "/pct%41/repo",
            "/a#b/repo",
            "/plus+amp&at@semi;/repo",
        ];
        if cfg!(windows) {
            paths.extend([r"C:\Users\dev\with space", r"\\server\share\repo"]);
        } else {
            paths.extend(["/q?mark/repo", r"/we\ird/repo"]);
        }
        for path in paths {
            let want = if cfg!(windows) {
                path.replace('\\', "/")
            } else {
                path.to_string()
            };
            let url = dialog_url(Path::new(path));
            assert_eq!(file_url_to_path(&url), PathBuf::from(want), "{url}");
        }
    }

    #[test]
    fn what_names_no_local_file_reads_as_no_path() {
        assert_eq!(file_url_to_path(""), PathBuf::new());
        assert_eq!(file_url_to_path("http://example.com/x"), PathBuf::new());
    }

    /// Encoded, so the preview's `?read=` cannot be read as part of a name
    /// that holds a `?` or a `#`.
    #[test]
    fn a_file_url_is_encoded() {
        assert_eq!(
            file_url(Path::new("/home/dev/日本語 #1/a?b.png")),
            "file:///home/dev/%E6%97%A5%E6%9C%AC%E8%AA%9E%20%231/a%3Fb.png"
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_windows_path_becomes_a_file_url() {
        assert_eq!(
            file_url(Path::new(r"C:\Users\dev\repo")),
            "file:///C:/Users/dev/repo"
        );
        assert_eq!(
            file_url(Path::new(r"\\server\share\repo")),
            "file://server/share/repo"
        );
    }

    /// Folded, it would name `/home/dev/we/ird.png`.
    #[test]
    #[cfg(not(windows))]
    fn a_backslash_in_a_name_is_encoded_not_folded() {
        let path = Path::new(r"/home/dev/we\ird.png");
        let url = file_url(path);
        assert_eq!(url, "file:///home/dev/we%5Cird.png");
        assert_eq!(file_url_to_path(&url), path);
    }

    #[test]
    fn a_repository_opens_the_folder_it_sits_in() {
        assert_eq!(
            picker_folder_url(Path::new("/home/dev/repo")),
            "file:///home/dev"
        );
    }

    /// An empty answer would open wherever the dialog was last left.
    #[test]
    fn a_repository_at_a_root_opens_the_root() {
        assert_eq!(picker_folder_url(Path::new("/")), "file:///");
        assert_eq!(picker_folder_url(Path::new("/repo")), "file:///");
    }

    // What counts as a root is the platform's reading of the path, so the
    // drive and UNC roots are tested on Windows only.
    #[test]
    #[cfg(windows)]
    fn a_repository_at_a_drive_root_opens_the_drive() {
        assert_eq!(
            picker_folder_url(Path::new("C:/Users/dev/repo")),
            "file:///C:/Users/dev"
        );
        assert_eq!(picker_folder_url(Path::new("C:/")), "file:///C:/");
        assert_eq!(picker_folder_url(Path::new(r"C:\")), "file:///C:/");
        assert_eq!(picker_folder_url(Path::new("C:/repo")), "file:///C:/");
        // A share is a root (`\\server` is not a folder). From `parent()`
        // it carries a trailing separator, as `C:/` does; given as the
        // repository itself it has none.
        assert_eq!(
            picker_folder_url(Path::new(r"\\server\share")),
            "file://server/share"
        );
        assert_eq!(
            picker_folder_url(Path::new(r"\\server\share\repo")),
            "file://server/share/"
        );
    }

    #[test]
    fn a_path_with_no_root_has_no_url() {
        assert_eq!(picker_folder_url(Path::new("")), "");
        assert_eq!(picker_folder_url(Path::new("repo")), "");
        assert_eq!(file_url(Path::new("repo/sub")), "");
    }

    /// One pixel, to load.
    const PNG: [u8; 70] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64,
        0x60, 0xf8, 0x5f, 0x0f, 0x00, 0x02, 0x87, 0x01, 0x80, 0xeb, 0x47, 0xba, 0x92, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// The child: a Qt application loading the QML its parent wrote, which
    /// says how each `Image` came out.
    #[test]
    #[ignore = "started by real_files_load_from_their_urls_and_read_back, offscreen, as a process of its own"]
    fn images_side() {
        let Some(qml) = std::env::var_os("URLPATH_IMAGES") else {
            return;
        };
        let mut app = qtbridge::qtbridge_type_lib::QGuiApplication::new();
        let mut engine = qtbridge::qtbridge_type_lib::QQmlApplicationEngine::new();
        crate::qml_engine::arm(engine.pin_mut());
        crate::qml_engine::load(engine.pin_mut(), &file_url(Path::new(&qml)))
            .expect("the images load");
        app.pin_mut().exec();
        drop(engine);
        qtbridge::collect_garbage();
        drop(app);
    }

    /// Real files under names that need encoding: a dialog's answer reads
    /// back to the same file, and `file_url`, bare and with the preview's
    /// query, loads in an `Image`.
    #[test]
    fn real_files_load_from_their_urls_and_read_back() {
        let base = std::env::temp_dir().join(format!("pgg url 実在 #{}", std::process::id()));
        let mut names = vec![
            "plain.png",
            "日本語 名前.png",
            "hash#1.png",
            "pct%41.png",
            "a#b/inner.png",
        ];
        if !cfg!(windows) {
            names.extend(["q?mark.png", r"we\ird.png"]);
        }
        let mut sources = Vec::new();
        for name in &names {
            let path = base.join(name);
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("the folder");
            std::fs::write(&path, PNG).expect("the file");
            let back = file_url_to_path(&dialog_url(&path));
            assert_eq!(
                std::fs::read(&back).ok().as_deref(),
                Some(&PNG[..]),
                "{name}"
            );
            sources.push(format!("{:?}", file_url(&path)));
            sources.push(format!("{:?}", format!("{}?read=1", file_url(&path))));
        }
        let qml = base.join("Images.qml");
        std::fs::write(
            &qml,
            format!(
                "import QtQuick\nItem {{\n    id: root\n    Repeater {{\n        id: images\n        model: [{}]\n        \
                 delegate: Image {{ required property string modelData; source: modelData }}\n    }}\n    \
                 function report() {{\n        for (let i = 0; i < images.count; i++)\n            \
                 console.log('IMAGE\\t' + images.itemAt(i).status + '\\t' + images.itemAt(i).source)\n        \
                 Qt.quit()\n    }}\n    Component.onCompleted: Qt.callLater(root.report)\n}}\n",
                sources.join(", ")
            ),
        )
        .expect("the QML");
        let out = std::process::Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "urlpath::tests::images_side",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env("URLPATH_IMAGES", &qml)
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_FORCE_STDERR_LOGGING", "1")
            .output()
            .expect("the child starts");
        let errors = String::from_utf8_lossy(&out.stderr);
        let images: Vec<&str> = errors
            .lines()
            .filter_map(|line| line.split_once("IMAGE\t").map(|(_, rest)| rest))
            .collect();
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(images.len(), sources.len(), "every image said:\n{errors}");
        // `Image.Ready` is 1: a local file loads in the call that sets it.
        let failed: Vec<&&str> = images
            .iter()
            .filter(|line| !line.starts_with("1\t"))
            .collect();
        assert!(failed.is_empty(), "{failed:?}");
    }
}
