use eframe::egui::{containers::*, widgets::*, *};
use std::f32::consts::TAU;

#[derive(PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FractalClock {
    paused: bool,
    time: f64,
    zoom: f32,
    start_line_width: f32,
    depth: usize,
    length_factor: f32,
    luminance_factor: f32,
    width_factor: f32,
    line_count: usize,
    timezone_offset: f64,
    offset_setting: (u8, u8, u8),
}

impl Default for FractalClock {
    fn default() -> Self {
        let (h, m, s) = Self::timezone_offset();
        let timezone_offset = ((h as u64 * 60 + m as u64) * 60 + s as u64) as f64;
        Self {
            paused: false,
            time: 0.0,
            zoom: 0.25,
            start_line_width: 2.5,
            depth: 9,
            length_factor: 0.8,
            luminance_factor: 0.8,
            width_factor: 0.9,
            line_count: 0,
            timezone_offset,
            offset_setting: (h, m, s),
        }
    }
}

impl super::View for FractalClock {
    fn name(&self) -> &'static str {
        "fractal clock"
    }

    fn is_dynamic(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut Ui) {
        if !self.paused {
            self.time = self.time();
            ui.ctx().request_repaint();
        }

        let painter = Painter::new(
            ui.ctx().clone(),
            ui.layer_id(),
            ui.available_rect_before_wrap(),
        );
        // Make sure we allocate what we used (everything)
        ui.expand_to_include_rect(painter.clip_rect());

        Frame::popup(ui.style())
            .stroke(Stroke::none())
            .show(ui, |ui| {
                ui.set_max_width(250.0);
                CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
            });

        self.paint(&painter);
    }
}

impl FractalClock {
    fn options_ui(&mut self, ui: &mut Ui) {
        ui.label(format!(
            "time: {:02}:{:02}:{:02}.{:03}",
            (self.time % (24.0 * 60.0 * 60.0) / 3600.0).floor(),
            (self.time % (60.0 * 60.0) / 60.0).floor(),
            (self.time % 60.0).floor(),
            (self.time % 1.0 * 100.0).floor()
        ));
        ui.label(format!("Painted line count: {}", self.line_count));

        ui.checkbox(&mut self.paused, "Paused");
        ui.horizontal(|ui| {
            ui.label("TimeZone :");
            ui.add(
                DragValue::new(&mut self.offset_setting.0)
                    .speed(1.0)
                    .prefix("h:")
                    .clamp_range(0u8..=23),
            );
            ui.add(
                DragValue::new(&mut self.offset_setting.1)
                    .speed(1.0)
                    .prefix("m:")
                    .clamp_range(0u8..=59),
            );
            ui.add(
                DragValue::new(&mut self.offset_setting.2)
                    .speed(1.0)
                    .prefix("s:")
                    .clamp_range(0u8..=59),
            );
            if ui.button("Set").clicked() {
                let (h, m, s) = self.offset_setting;
                self.timezone_offset = ((h as u64 * 60 + m as u64) * 60 + s as u64) as f64;
            }
        });
        ui.add(Slider::new(&mut self.zoom, 0.0..=1.0).text("zoom"));
        ui.add(Slider::new(&mut self.start_line_width, 0.0..=5.0).text("Start line width"));
        ui.add(Slider::new(&mut self.depth, 0..=14).text("depth"));
        ui.add(Slider::new(&mut self.length_factor, 0.0..=1.0).text("length factor"));
        ui.add(Slider::new(&mut self.luminance_factor, 0.0..=1.0).text("luminance factor"));
        ui.add(Slider::new(&mut self.width_factor, 0.0..=1.0).text("width factor"));

        eframe::egui::reset_button(ui, self);
    }

    fn paint(&mut self, painter: &Painter) {
        struct Hand {
            length: f32,
            angle: f32,
            vec: Vec2,
        }

        impl Hand {
            fn from_length_angle(length: f32, angle: f32) -> Self {
                Self {
                    length,
                    angle,
                    vec: length * Vec2::angled(angle),
                }
            }
        }

        let angle_from_period =
            |period| TAU * (self.time.rem_euclid(period) / period) as f32 - TAU / 4.0;

        let hands = [
            // Second hand:
            Hand::from_length_angle(self.length_factor, angle_from_period(60.0)),
            // Minute hand:
            Hand::from_length_angle(self.length_factor, angle_from_period(60.0 * 60.0)),
            // Hour hand:
            Hand::from_length_angle(0.5, angle_from_period(12.0 * 60.0 * 60.0)),
        ];

        let mut shapes: Vec<Shape> = Vec::new();

        let rect = painter.clip_rect();
        let to_screen = emath::RectTransform::from_to(
            Rect::from_center_size(Pos2::ZERO, rect.square_proportions() / self.zoom),
            rect,
        );

        let mut paint_line = |points: [Pos2; 2], color: Color32, width: f32| {
            let line = [to_screen * points[0], to_screen * points[1]];

            // culling
            if rect.intersects(Rect::from_two_pos(line[0], line[1])) {
                shapes.push(Shape::line_segment(line, (width, color)));
            }
        };

        let hand_rotations = [
            hands[0].angle - hands[2].angle + TAU / 2.0,
            hands[1].angle - hands[2].angle + TAU / 2.0,
        ];

        let hand_rotors = [
            hands[0].length * emath::Rot2::from_angle(hand_rotations[0]),
            hands[1].length * emath::Rot2::from_angle(hand_rotations[1]),
        ];

        #[derive(Clone, Copy)]
        struct Node {
            pos: Pos2,
            dir: Vec2,
        }

        let mut nodes = Vec::new();

        let mut width = self.start_line_width;

        for (i, hand) in hands.iter().enumerate() {
            let center = pos2(0.0, 0.0);
            let end = center + hand.vec;
            paint_line([center, end], Color32::from_additive_luminance(255), width);
            if i < 2 {
                nodes.push(Node {
                    pos: end,
                    dir: hand.vec,
                });
            }
        }

        let mut luminance = 0.7; // Start dimmer than main hands

        let mut new_nodes = Vec::new();
        for _ in 0..self.depth {
            new_nodes.clear();
            new_nodes.reserve(nodes.len() * 2);

            luminance *= self.luminance_factor;
            width *= self.width_factor;

            let luminance_u8 = (255.0 * luminance).round() as u8;
            if luminance_u8 == 0 {
                break;
            }

            for &rotor in &hand_rotors {
                for a in &nodes {
                    let new_dir = rotor * a.dir;
                    let b = Node {
                        pos: a.pos + new_dir,
                        dir: new_dir,
                    };
                    paint_line(
                        [a.pos, b.pos],
                        Color32::from_additive_luminance(luminance_u8),
                        width,
                    );
                    new_nodes.push(b);
                }
            }

            std::mem::swap(&mut nodes, &mut new_nodes);
        }
        self.line_count = shapes.len();
        painter.extend(shapes);
    }

    // This is ugly, but it works.
    fn timezone_offset() -> (u8, u8, u8) {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                    let date = js_sys::Date::new_0();
                    let m = date.get_timezone_offset() as u32;
                    ((m / 60) as u8, (m % 60) as u8, 0)
            } else if #[cfg(target_os = "windows")] {
                let offset = time::UtcOffset::current_local_offset().unwrap();
                offset.as_hms()
            } else if #[cfg(target_vendor = "apple")] {
                // https://developer.apple.com/documentation/foundation/nstimezone?changes=latest_minor&language=objc
                use objc::{class, msg_send, sel, sel_impl, runtime::Object};
                unsafe {
                    let tz: *const Object = msg_send![class!(NSTimeZone), localTimeZone];
                    let s: i64 = msg_send![tz, secondsFromGMT];
                    ((s / 3600) as u8, ((s / 60) % 60) as u8, (s % 60) as u8)
                }
            } else {
                // CVE-2020-26235
                // https://github.com/advisories/GHSA-wcg3-cvx6-7396
                // TLDR: Unix envirentment variable is not MT safe, thus we can't read TZ
                (0, 0, 0)
            }
        }
    }

    fn time(&self) -> f64 {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let date = js_sys::Date::new_0();
                let h = date.get_utc_hours() as u64;
                let m = date.get_utc_minutes() as u64;
                let s = date.get_utc_seconds() as u64;
                let ms = date.get_utc_milliseconds();
                ((h as u64 * 60 + m as u64) * 60 + s as u64) as f64 + (ms as f64) * 1e-3+ self.timezone_offset
            } else {
                let (h, m, s, ns) = time::OffsetDateTime::now_utc().to_hms_nano();
                ((h as u64 * 60 + m as u64) * 60 + s as u64) as f64 + (ns as f64) * 1e-9 + self.timezone_offset
            }
        }
    }
}
