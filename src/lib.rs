#![warn(clippy::all, clippy::pedantic, rust_2018_idioms)]
#![allow(
    clippy::must_use_candidate,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]

mod app;
mod core;
mod screen;
pub use app::GbApp;
