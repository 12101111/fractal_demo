use eframe::egui::{self, *};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{mem::size_of, sync::Arc};

#[derive(Debug)]
pub struct JuliaSetShader {
    gl: OnceCell<Arc<Mutex<Context>>>,
    center: (f32, f32),
    ratio: f32,
    step: f32,
    c: (f32, f32),
    m: i32,
}

impl Default for JuliaSetShader {
    fn default() -> Self {
        Self {
            gl: Default::default(),
            center: (0.0, 0.0),
            ratio: 1.0,
            step: 0.1,
            c: (0.3, 0.5),
            m: 2,
        }
    }
}

impl super::View for JuliaSetShader {
    fn name(&self) -> &'static str {
        "Julia Set (Shader)"
    }

    fn is_dynamic(&self) -> bool {
        false
    }

    fn ui(&mut self, ui: &mut Ui) {
        let painter = Painter::new(
            ui.ctx().clone(),
            ui.layer_id(),
            ui.available_rect_before_wrap(),
        );
        let rect = painter.clip_rect();
        ui.expand_to_include_rect(rect);

        let gl = self.gl.clone();
        let ppp = ui.ctx().pixels_per_point();
        let (width, height) = (rect.width() * ppp, rect.height() * ppp);
        let margin = rect.left() * ppp + 0.5;
        let center = self.center;
        let ratio = self.ratio;
        let c = self.c;
        let m = self.m;

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(move |_info, render_ctx| {
                if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                    let mut gl = gl.get().unwrap().lock();
                    gl.paint(painter.gl(), (width, height), center, ratio, margin, c, m);
                } else {
                    eprintln!("Can't do custom painting because we are not using a glow context");
                }
            }),
        };
        painter.add(callback);
        Frame::popup(ui.style())
            .stroke(Stroke::none())
            .show(ui, |ui| {
                ui.set_max_width(250.0);
                CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
            });
    }
}

impl JuliaSetShader {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default = Self::default();
        default
            .gl
            .get_or_init(|| Arc::new(Mutex::new(Context::new(&cc.gl))));
        default
    }
    fn options_ui(&mut self, ui: &mut Ui) {
        if ui.input().key_pressed(Key::ArrowLeft) {
            self.center.0 -= 0.1 / self.ratio;
        }
        if ui.input().key_pressed(Key::ArrowRight) {
            self.center.0 += 0.1 / self.ratio;
        }
        if ui.input().key_pressed(Key::ArrowDown) {
            self.center.1 -= 0.1 / self.ratio;
        }
        if ui.input().key_pressed(Key::ArrowUp) {
            self.center.1 += 0.1 / self.ratio;
        }

        if ui.input().key_pressed(Key::A) {
            self.c.0 -= 0.01 * self.step;
        }
        if ui.input().key_pressed(Key::D) {
            self.c.0 += 0.01 * self.step;
        }
        if ui.input().key_pressed(Key::S) {
            self.c.1 -= 0.01 * self.step;
        }
        if ui.input().key_pressed(Key::W) {
            self.c.1 += 0.01 * self.step;
        }

        if ui.input().key_pressed(Key::Enter) || ui.input().key_pressed(Key::PageDown) {
            self.ratio *= 1.2;
        }
        if ui.input().key_pressed(Key::Backspace) || ui.input().key_pressed(Key::PageUp) {
            self.ratio /= 1.2;
        }
        ui.horizontal(|ui| {
            ui.label("center :");
            ui.label("x:");
            ui.add(DragValue::new(&mut self.center.0).speed(0.01));
            ui.label("y:");
            ui.add(DragValue::new(&mut self.center.1).speed(0.01));
        });
        ui.horizontal(|ui| {
            ui.label("C :");
            ui.add(DragValue::new(&mut self.c.0).speed(0.01));
            ui.label("+");
            ui.add(DragValue::new(&mut self.c.1).speed(0.01).suffix("i"));
        });
        ui.horizontal(|ui| {
            ui.label("m :");
            ui.add(DragValue::new(&mut self.m).speed(1.0).clamp_range(2..=9));
            if ui.button("+").clicked() && self.m < 10 {
                self.m += 1;
            }
            if ui.button("-").clicked() && self.m > 2 {
                self.m -= 1;
            }
        });
        ui.horizontal(|ui| {
            ui.label("ratio :");
            ui.add(
                DragValue::new(&mut self.ratio)
                    .speed(0.5)
                    .clamp_range(1.0..=f32::MAX),
            );
        });
        ui.horizontal(|ui| {
            ui.label("step :");
            ui.add(
                DragValue::new(&mut self.step)
                    .speed(0.05)
                    .clamp_range(0.05..=1.0),
            );
        });
        if ui.button("reset").clicked() || ui.input().key_pressed(Key::Escape) {
            self.center = (0.0, 0.0);
            self.ratio = 1.0;
        }
    }
}

#[derive(Debug)]
struct Context {
    program: glow::Program,
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
}

const VERTICES: &[f32] = &[-1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];
const INDICES: &[i32] = &[0, 1, 2, 1, 2, 3];

const VERTEX_SHADER: &str = r#"
layout (location = 0) in vec2 in_pos;
void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
}
"#;

// hsv2rgb: https://stackoverflow.com/questions/15095909/from-rgb-to-hsv-in-opengl-glsl
const FRAGMENT_SHADER: &str = r#"
precision mediump float;
uniform vec2 viewport;
uniform vec2 min;
uniform vec2 max;
uniform float margin;
uniform vec2 c;
uniform int m;
out vec4 out_color;
const float MAX = 128.0;
const float LIMIT = 4.0;

vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

vec3 run() {
    float count;
    vec2 z = mix(min, max, (gl_FragCoord.xy - vec2(margin, margin)) / viewport);
    for (count = 0.0; count < MAX; count+=1.0) {
        for (int n = 1; n < m; n++) {
            float r = z.x * z.x - z.y * z.y;
            float i = 2.0 * z.x * z.y;
            z = vec2(r, i);
        }
        z.x += c.x;
        z.y += c.y;
        if (z.x * z.x + z.y * z.y > LIMIT) break;
    }
    return vec3(z, count);
}

void main() {
    vec3 r = run();
    if (r.z == MAX) {
        out_color = vec4(0.0, 0.0, 0.0, 0.0);
    } else if (r.z == 0.0) {
        out_color = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        float c = r.z / MAX;
        float sum = r.x * r.x + r.y * r.y;
        vec3 color = hsv2rgb(vec3(c , 0.9, sum / 4.0));
        out_color = vec4(color, 1.0);
    }
}
"#;

impl Context {
    fn new(gl: &glow::Context) -> Self {
        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            // in/out
            "#version 300 es"
        } else {
            // location
            "#version 330"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let shader_sources = [
                (glow::VERTEX_SHADER, VERTEX_SHADER),
                (glow::FRAGMENT_SHADER, FRAGMENT_SHADER),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        panic!("{}", gl.get_shader_info_log(shader));
                    }
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));

            let verts_slice = std::slice::from_raw_parts(
                VERTICES.as_ptr() as *const u8,
                VERTICES.len() * size_of::<f32>(),
            );

            let indices_slice = std::slice::from_raw_parts(
                INDICES.as_ptr() as *const u8,
                INDICES.len() * size_of::<i32>(),
            );

            let vbo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, verts_slice, glow::DYNAMIC_DRAW);

            let ebo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                indices_slice,
                glow::DYNAMIC_DRAW,
            );

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * size_of::<f32>() as i32, 0);

            Self {
                program,
                vao,
                _vbo: vbo,
                _ebo: ebo,
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        gl: &glow::Context,
        view: (f32, f32),
        center: (f32, f32),
        ratio: f32,
        margin: f32,
        c: (f32, f32),
        m: i32,
    ) {
        use glow::HasContext as _;
        let wh = view.0 / view.1;
        let min = (center.0 - 1.5 / ratio * wh, center.1 - 1.5 / ratio);
        let max = (center.0 + 1.5 / ratio * wh, center.1 + 1.5 / ratio);
        unsafe {
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "viewport").as_ref(),
                view.0,
                view.1,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "min").as_ref(),
                min.0,
                min.1,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "max").as_ref(),
                max.0,
                max.1,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "margin").as_ref(),
                margin,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "c").as_ref(),
                c.0,
                c.1,
            );
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "m").as_ref(), m);
            gl.draw_elements(glow::TRIANGLES, INDICES.len() as i32, glow::UNSIGNED_INT, 0);
        }
    }
}
