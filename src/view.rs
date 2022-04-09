mod fractal_clock;
mod juliaset_shader;
mod koch_snowflake;
mod mandelbrot_shader;
mod sierpinski_triangle;

use eframe::egui::Ui;
pub use fractal_clock::FractalClock;
pub use juliaset_shader::JuliaSetShader;
pub use koch_snowflake::KochSnowFlake;
pub use mandelbrot_shader::MandelbrotShader;
pub use sierpinski_triangle::SierpinskiTriangle;

pub trait View {
    fn name(&self) -> &'static str;
    fn is_dynamic(&self) -> bool;
    fn ui(&mut self, ui: &mut Ui);
}
