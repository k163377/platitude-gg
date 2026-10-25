//! `cargo xtask film` — behaviour as an animation: a scene played into
//! the headless window with real input events and grabbed every tick,
//! each side of a comparison shot from its own tree, and the sides baked
//! into APNGs on one clock for the board (film skill).
//!
//! The verb that plays the scene is put into the tree for the film and
//! taken out after it (`inject`); everything else is here. The frames are
//! drawn by the app — the hand and the caption band included — so this
//! side only assembles PNGs it never decodes (`png::animate`).

mod bake;
mod changes;
mod cli;
mod inject;
mod scene;
mod shoot;

pub(crate) use cli::run;
#[cfg(test)]
pub(crate) use inject::PAYLOAD;
pub(crate) use inject::injected;

use std::path::{Path, PathBuf};

use crate::command::{self, Permission, Where};

pub(crate) static IN: command::Command = command::Command {
    id: "film.in",
    call: "film in",
    purpose: "put the film verb into this tree for a film, never to be committed",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static OUT: command::Command = command::Command {
    id: "film.out",
    call: "film out",
    purpose: "take the film verb out of this tree again",
    run_in: Where::Seat,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static SHOOT: command::Command = command::Command {
    id: "film.shoot",
    call: "film shoot <scene>",
    purpose: "one side of a scene, headless, into the scene's directory",
    run_in: Where::Seat,
    needs: &["the film verb in this tree (`film in`)"],
    permission: Permission::Plain,
};

pub(crate) static BAKE: command::Command = command::Command {
    id: "film.bake",
    call: "film bake <scene>",
    purpose: "a scene's sides as APNGs on one clock, for the board",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static CHANGES: command::Command = command::Command {
    id: "film.changes",
    call: "film changes <scene>",
    purpose: "when a side's picture changed and where",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&IN, &OUT, &SHOOT, &BAKE, &CHANGES];

/// One side of a comparison: the tree it is shot from, and the tag its
/// frames carry.
struct Side {
    name: &'static str,
    /// The band's word for the side, in the board's language.
    tag: &'static str,
    /// The tag's colour, as the name of a Theme token.
    colour: &'static str,
}

/// Every side, in the order `bake` puts them: what is compared against
/// first.
const SIDES: &[Side] = &[
    Side {
        name: "before",
        tag: "直す前",
        colour: "danger",
    },
    Side {
        name: "main",
        tag: "main",
        colour: "danger",
    },
    Side {
        name: "after",
        tag: "直した後",
        colour: "success",
    },
    Side {
        name: "now",
        tag: "今",
        colour: "success",
    },
    Side {
        name: "opt2",
        tag: "案 2",
        colour: "textLink",
    },
    Side {
        name: "opt3",
        tag: "案 3",
        colour: "textLink",
    },
];

fn side(name: &str) -> Result<&'static Side, String> {
    SIDES.iter().find(|side| side.name == name).ok_or_else(|| {
        let names: Vec<&str> = SIDES.iter().map(|side| side.name).collect();
        format!("`{name}` is no side ({})", names.join(" "))
    })
}

/// Where a scene's repositories, settings and frames go: under this tree's
/// target/, by the scene file's name.
fn scene_dir(tree: &Path, scene: &Path) -> Result<PathBuf, String> {
    let stem = scene
        .file_stem()
        .ok_or_else(|| format!("{} names no file", scene.display()))?;
    Ok(tree.join("target").join("film").join(stem))
}
