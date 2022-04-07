//! This module define main app logic

use crate::view::*;
use eframe::{egui, epi};

pub struct FractalApp {
    selected: usize,
    views: Vec<Box<dyn View>>,
}

impl FractalApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            selected: Default::default(),
            views: vec![
                Box::new(KochSnowFlake::<false>::new(cc)),
                Box::new(KochSnowFlake::<true>::new(cc)),
                Box::new(FractalClock::default()),
            ],
        }
    }
}

impl epi::App for FractalApp {
    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, _storage: &mut dyn epi::Storage) {
        //TODO: save the state
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);

                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
                for i in 0..self.views.len() {
                    if ui
                        .selectable_label(i == self.selected, self.views[i].name())
                        .clicked()
                    {
                        self.selected = i
                    }
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| self.views[self.selected].ui(ui));
        });
        if self.views[self.selected].is_dynamic() {
            ctx.request_repaint();
        }
    }
}
