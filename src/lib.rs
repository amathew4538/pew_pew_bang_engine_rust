//! # Main Script
//!
//! Creates the main window for the engine

mod resources;
mod model;
mod light;
mod vertex;
mod texture;
mod camera;
mod instance;
mod state;
pub mod app;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

/// # Run
///
/// See [`crate::app::run()`]
pub fn run() -> anyhow::Result<()> {
    app::run()
}

/// # Run Web
///
/// See [`crate::app::run_web()`]
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    app::run_web()
}
