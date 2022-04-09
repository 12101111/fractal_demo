mod fractal_clock;
mod koch_snowflake;
mod sierpinski_triangle;
mod mandelbrot_shader;
mod juliaset_shader;

use eframe::egui::Ui;
pub use fractal_clock::FractalClock;
pub use koch_snowflake::KochSnowFlake;
pub use sierpinski_triangle::SierpinskiTriangle;
pub use mandelbrot_shader::MandelbrotShader;
pub use juliaset_shader::JuliaSetShader;

pub trait View {
    fn name(&self) -> &'static str;
    fn is_dynamic(&self) -> bool;
    fn ui(&mut self, ui: &mut Ui);
}
