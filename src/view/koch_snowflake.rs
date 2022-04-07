use eframe::{
    egui::{self, containers::*, *},
    emath::{pos2, Pos2},
};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{
    mem::{size_of, swap},
    sync::Arc,
};

const DEFAULT_DEPTH: u32 = 6;
const MAX_DEPTH: u32 = 10;

#[derive(Debug)]
pub struct KochSnowFlake<const ANTI: bool> {
    gl: OnceCell<Arc<Mutex<Context<ANTI>>>>,
    depth: u32,
}

impl<const ANTI: bool> Default for KochSnowFlake<ANTI> {
    fn default() -> Self {
        Self {
            gl: Default::default(),
            depth: DEFAULT_DEPTH,
        }
    }
}

impl<const ANTI: bool> super::View for KochSnowFlake<ANTI> {
    fn name(&self) -> &'static str {
        if ANTI {
            "Koch Antisnowflake"
        } else {
            "Koch Snowflake"
        }
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

        Frame::popup(ui.style())
            .stroke(Stroke::none())
            .show(ui, |ui| {
                ui.set_max_width(250.0);
                CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
            });

        let gl = self.gl.clone();
        let depth = self.depth;
        let ratio = rect.height() / rect.width();

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(move |_info, render_ctx| {
                if let Some(painter) = render_ctx.downcast_ref::<egui_glow::Painter>() {
                    let mut gl = gl.get().unwrap().lock();
                    gl.paint(painter.gl(), depth, ratio);
                } else {
                    eprintln!("Can't do custom painting because we are not using a glow context");
                }
            }),
        };
        painter.add(callback);
    }
}

impl<const ANTI: bool> KochSnowFlake<ANTI> {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default = Self::default();
        default
            .gl
            .get_or_init(|| Arc::new(Mutex::new(Context::new(&cc.gl))));
        default
    }
    fn options_ui(&mut self, ui: &mut Ui) {
        ui.label(format!(
            "Painted line count: {}",
            3 * 4usize.pow(self.depth - 1)
        ));
        ui.horizontal(|ui| {
            ui.label("Depth :");
            ui.add(
                DragValue::new(&mut self.depth)
                    .speed(1.0)
                    .clamp_range(1..=MAX_DEPTH),
            );
            if ui.button("+").clicked() && self.depth < MAX_DEPTH {
                self.depth += 1;
            }
            if ui.button("-").clicked() && self.depth > 1 {
                self.depth -= 1;
            }
        });
        if ui.button("reset").clicked() {
            self.depth = DEFAULT_DEPTH;
        }
    }
}

#[derive(Debug)]
struct Context<const ANTI: bool> {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertices: Vec<Vec<Pos2>>,
    depth: u32,
}

const VERTEX_SHADER: &str = r#"
layout (location = 0) in vec2 in_pos;
uniform float uni_ratio;
void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    gl_Position.x *= uni_ratio;
}
"#;

const FRAGMENT_SHADER: &str = r#"
precision mediump float;
out vec4 out_color;
void main() {
    out_color = vec4(0.7, 0.7, 0.7, 1.0);
}
"#;

impl<const ANTI: bool> Context<ANTI> {
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

            Self {
                program,
                vao: gl.create_vertex_array().unwrap(),
                vbo: gl.create_buffer().unwrap(),
                vertices: vec![vec![
                    pos2(-0.8, -0.8 / 3.0_f32.sqrt()),
                    pos2(0.8, -0.8 / 3.0_f32.sqrt()),
                    pos2(0.0, 1.6 / 3.0_f32.sqrt()),
                ]],
                depth: 1,
            }
        }
    }

    fn calc(&mut self, depth: u32) {
        if self.vertices.len() > depth as usize - 1 {
            return;
        }
        for d in self.vertices.len()..depth as usize {
            let len = self.vertices[d - 1].len();
            let mut new = Vec::with_capacity(len * 4);
            let iter = (1..len)
                .map(|i| (i - 1, i))
                .chain([(len - 1, 0)])
                .map(|(s, e)| (self.vertices[d - 1][s], self.vertices[d - 1][e]));
            for (s, e) in iter {
                // s---l\   /r---e
                //       \ /
                //        m
                let l = pos2((e.x + 2.0 * s.x) / 3.0, (e.y + 2.0 * s.y) / 3.0);
                let r = pos2((s.x + 2.0 * e.x) / 3.0, (s.y + 2.0 * e.y) / 3.0);
                #[allow(clippy::collapsible_else_if)]
                let m = if ANTI {
                    if s.y == e.y {
                        pos2((s.x + e.x) / 2.0, s.y - (s.x - e.x) / (2.0 * 3.0f32.sqrt()))
                    } else {
                        pos2(
                            (s.x + e.x) / 2.0 - (e.y - s.y) / (2.0 * 3.0f32.sqrt()),
                            (s.y + e.y) / 2.0 - (s.x - e.x) / (2.0 * 3.0f32.sqrt()),
                        )
                    }
                } else {
                    if s.y == e.y {
                        // s-e
                        pos2((s.x + e.x) / 2.0, s.y + (s.x - e.x) / (2.0 * 3.0f32.sqrt()))
                    } else {
                        // e
                        //  \ r
                        //   ----m
                        //      /
                        //   l /
                        //     \ s
                        // (e.x-s.x)x + (e.y-s.y)y = 0
                        // ==>
                        // x = e.y - s.y
                        // y = s.x - e.x
                        pos2(
                            (s.x + e.x) / 2.0 + (e.y - s.y) / (2.0 * 3.0f32.sqrt()),
                            (s.y + e.y) / 2.0 + (s.x - e.x) / (2.0 * 3.0f32.sqrt()),
                        )
                    }
                };
                new.extend_from_slice(&[s, l, m, r]);
            }
            tracing::debug!(depth = d + 1, len = new.len(), verts = self.vertices.len());
            self.vertices.push(new);
        }
    }

    unsafe fn update_vertices(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        let mut vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let verts_slice = self.vertices[self.depth as usize - 1].as_slice();
        let verts_slice = std::slice::from_raw_parts(
            verts_slice.as_ptr() as *const u8,
            verts_slice.len() * size_of::<Pos2>(),
        );

        let mut vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, verts_slice, glow::DYNAMIC_DRAW);

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * size_of::<f32>() as i32, 0);
        swap(&mut self.vao, &mut vao);
        swap(&mut self.vbo, &mut vbo);
        gl.delete_vertex_array(vao);
        gl.delete_buffer(vbo);
    }

    fn paint(&mut self, gl: &glow::Context, mut depth: u32, ratio: f32) {
        use glow::HasContext as _;
        depth = depth.min(MAX_DEPTH);
        depth = depth.max(1);
        if self.depth != depth {
            self.calc(depth);
            self.depth = depth;
            unsafe { self.update_vertices(gl) };
        }
        unsafe {
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "uni_ratio").as_ref(),
                ratio,
            );
            gl.draw_arrays(glow::LINE_LOOP, 0, 3 * 4i32.pow(depth - 1));
        }
    }
}
