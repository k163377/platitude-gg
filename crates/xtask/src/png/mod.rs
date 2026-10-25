//! PNG by hand, std only (CLAUDE.md §技術スタック): the fixtures this
//! crate writes, and the screenshots it reads back to cut a crop out of.

mod apng;
mod checksum;
mod inflate;
mod read;
mod write;

pub(crate) use apng::{Frame, animate, image_data};
pub(crate) use read::{Image, decode};
pub(crate) use write::rgba;
