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

const DEFAULT_DEPTH: u32 = 2;
const MAX_DEPTH: u32 = 10;

#[derive(Debug)]
pub struct SierpinskiTriangle {
    gl: OnceCell<Arc<Mutex<Context>>>,
    depth: u32,
}

impl Default for SierpinskiTriangle {
    fn default() -> Self {
        Self {
            gl: Default::default(),
            depth: DEFAULT_DEPTH,
        }
    }
}

impl super::View for SierpinskiTriangle {
    fn name(&self) -> &'static str {
        "Sierpinski Triangle"
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

impl SierpinskiTriangle {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default = Self::default();
        default
            .gl
            .get_or_init(|| Arc::new(Mutex::new(Context::new(&cc.gl))));
        default
    }
    fn options_ui(&mut self, ui: &mut Ui) {
        ui.label(format!("Painted triangle count: {}", 3i32.pow(self.depth)));
        ui.horizontal(|ui| {
            ui.label("Depth :");
            ui.add(
                DragValue::new(&mut self.depth)
                    .speed(1.0)
                    .clamp_range(0..=MAX_DEPTH),
            );
            if ui.button("+").clicked() && self.depth < MAX_DEPTH {
                self.depth += 1;
            }
            if ui.button("-").clicked() && self.depth > 0 {
                self.depth -= 1;
            }
        });
        if ui.button("reset").clicked() {
            self.depth = DEFAULT_DEPTH;
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct TriangleIndex {
    l: u32,
    r: u32,
    u: u32,
}

fn index(l: u32, r: u32, u: u32) -> TriangleIndex {
    TriangleIndex { l, r, u }
}

#[derive(Debug)]
struct Context {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    vertices: Vec<Pos2>,
    indices: Vec<Vec<TriangleIndex>>,
    depth: u32,
}

const VERTEX_SHADER: &str = r#"
layout (location = 0) in vec2 in_pos;
uniform float uni_ratio;
out vec3 v_color;

void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    gl_Position.x *= uni_ratio;
    float r = (0.8 + in_pos.y) / 3.0;
    float g = (0.8 - in_pos.x - in_pos.y) / 1.6;
    float b = (in_pos.x + 0.8 - in_pos.y) / 1.6;
    v_color = vec3(r, g, b);
}
"#;

const FRAGMENT_SHADER: &str = r#"
precision mediump float;
in vec3 v_color;
out vec4 out_color;
void main() {
    out_color = vec4(v_color, 1.0);
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

            Self {
                program,
                vao: gl.create_vertex_array().unwrap(),
                vbo: gl.create_buffer().unwrap(),
                ebo: gl.create_buffer().unwrap(),
                vertices: vec![
                    pos2(-0.8, -0.8 / 3.0_f32.sqrt()),
                    pos2(0.8, -0.8 / 3.0_f32.sqrt()),
                    pos2(0.0, 1.6 / 3.0_f32.sqrt()),
                ],
                indices: vec![vec![TriangleIndex { l: 0, r: 1, u: 2 }]],
                depth: 0,
            }
        }
    }

    fn calc(&mut self, depth: u32) {
        if self.indices.len() > depth as usize {
            return;
        }
        for d in self.indices.len() - 1..depth as usize {
            let len = self.indices[d].len();
            let mut new = Vec::with_capacity(len * 3);
            for s in &self.indices[d] {
                let i = self.vertices.len() as u32;
                let l = self.vertices[s.l as usize].to_vec2();
                let r = self.vertices[s.r as usize].to_vec2();
                let u = self.vertices[s.u as usize].to_vec2();
                let nl = ((l + u) / 2.0).to_pos2(); // i
                let nr = ((r + u) / 2.0).to_pos2(); // i + 1
                let nd = ((l + r) / 2.0).to_pos2(); // i + 2
                let li = index(s.l, i + 2, i);
                let ri = index(i + 2, s.r, i + 1);
                let ui = index(i, i + 1, s.u);
                self.vertices.extend([nl, nr, nd]);
                new.extend([li, ri, ui]);
            }
            tracing::debug!(depth = d, indices = new.len(), verts = self.vertices.len());
            self.indices.push(new);
        }
    }

    unsafe fn update_vertices(&mut self, gl: &glow::Context) {
        use glow::HasContext as _;

        let mut vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        let len = (3usize.pow(self.depth) + 1) * 3 / 2;
        let verts_slice = &self.vertices[..len];
        let verts_slice = std::slice::from_raw_parts(
            verts_slice.as_ptr() as *const u8,
            verts_slice.len() * size_of::<Pos2>(),
        );

        let indices_slice = self.indices[self.depth as usize].as_slice();
        let indices_slice = std::slice::from_raw_parts(
            indices_slice.as_ptr() as *const u8,
            indices_slice.len() * size_of::<TriangleIndex>(),
        );

        let mut vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, verts_slice, glow::DYNAMIC_DRAW);

        let mut ebo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            indices_slice,
            glow::DYNAMIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * size_of::<f32>() as i32, 0);
        swap(&mut self.vao, &mut vao);
        swap(&mut self.vbo, &mut vbo);
        swap(&mut self.ebo, &mut ebo);
        gl.delete_vertex_array(vao);
        gl.delete_buffer(vbo);
        gl.delete_buffer(ebo);
    }

    fn paint(&mut self, gl: &glow::Context, mut depth: u32, ratio: f32) {
        use glow::HasContext as _;
        depth = depth.min(MAX_DEPTH);
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
            gl.draw_elements(
                glow::TRIANGLES,
                3i32.pow(self.depth + 1),
                glow::UNSIGNED_INT,
                0,
            );
        }
    }
}
