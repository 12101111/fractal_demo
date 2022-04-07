mod fractal_clock;
mod koch_snowflake;

use eframe::egui::Ui;
pub use fractal_clock::FractalClock;
pub use koch_snowflake::KochSnowFlake;

pub trait View {
    fn name(&self) -> &'static str;
    fn is_dynamic(&self) -> bool;
    fn ui(&mut self, ui: &mut Ui);
}
