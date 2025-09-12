//! A GUI for `argos-arpa`.
//!
//! The intent is to cover most of the expected usage of the library, which
//! currently means:
//!  - pulsar metadata management.

#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::must_use_candidate)]

extern crate argos_arpa as arpa;

pub mod app;

use app::Application;
use log::{debug, error};

fn main() {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    let application = match Application::new() {
        Ok(a) => a,
        Err(err) => {
            error!("{err}");
            return;
        }
    };

    debug!("Running app");

    let result = eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Ok(Box::new(application.init(cc)))),
    );

    match result {
        Ok(()) => println!("Application closed gracefully."),
        Err(err) => println!("Runtime error: {err}"),
    }
}
