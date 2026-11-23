mod frame;
use eframe::egui;
use frame::{Frame, Options};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Default)]
struct Viewer {
    frame: Option<Frame>,
    texture: Option<egui::TextureHandle>,
    options: Options,
    files: Vec<PathBuf>,
    index: usize,
    status: String,
}

fn siblings(path: &Path) -> Vec<PathBuf> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let prefix = stem.trim_end_matches(|c: char| c.is_ascii_digit());
    let digits = stem.len() - prefix.len();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut files = Vec::new();
    if digits > 0
        && let Ok(entries) = parent.read_dir()
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("7th"))
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.strip_prefix(prefix))
                    .is_some_and(|s| s.len() == digits && s.bytes().all(|b| b.is_ascii_digit()))
            {
                files.push(p);
            }
        }
    }
    files.sort();
    if files.is_empty() {
        files.push(path.to_owned())
    }
    files
}
impl Viewer {
    fn open(&mut self, ctx: &egui::Context, path: PathBuf) {
        self.files = siblings(&path);
        self.index = self
            .files
            .iter()
            .position(|p| p.file_name() == path.file_name())
            .unwrap_or(0);
        self.load(ctx);
    }
    fn load(&mut self, ctx: &egui::Context) {
        let p = &self.files[self.index];
        self.texture = None;
        self.frame = None;
        match Frame::open(p) {
            Ok(f) => {
                self.status = format!(
                    "{}   ·   {} × {}   ·   YUYV 4:2:2",
                    p.file_name().unwrap_or_default().to_string_lossy(),
                    f.w,
                    f.h
                );
                self.frame = Some(f);
                self.refresh(ctx);
            }
            Err(e) => self.status = format!("{} — {e}", p.display()),
        }
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        if let Some(f) = &self.frame {
            let (w, h, rgb) = f.render(2048, self.options);
            self.texture = Some(ctx.load_texture(
                "frame",
                egui::ColorImage::from_rgb([w, h], &rgb),
                egui::TextureOptions::LINEAR,
            ));
        }
    }
    fn choose(&mut self, ctx: &egui::Context) {
        match Command::new("/usr/bin/osascript")
            .args([
                "-e",
                "POSIX path of (choose file with prompt \"Open a 7thSense frame\")",
            ])
            .output()
        {
            Ok(o) if o.status.success() => self.open(
                ctx,
                PathBuf::from(String::from_utf8_lossy(&o.stdout).trim_end_matches('\n')),
            ),
            Ok(o) if !String::from_utf8_lossy(&o.stderr).contains("(-128)") => {
                self.status = String::from_utf8_lossy(&o.stderr).into_owned()
            }
            Err(e) => self.status = e.to_string(),
            _ => {}
        }
    }
}
impl eframe::App for Viewer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(p) = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone())) {
            self.open(ctx, p);
        }
        let shortcut = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O));
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Open…").clicked() || shortcut {
                    self.choose(ctx);
                }
                let previous = ui
                    .add_enabled(self.index > 0, egui::Button::new("←"))
                    .clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
                let next = ui
                    .add_enabled(self.index + 1 < self.files.len(), egui::Button::new("→"))
                    .clicked()
                    || ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
                if previous && self.index > 0 {
                    self.index -= 1;
                    self.load(ctx);
                }
                if next && self.index + 1 < self.files.len() {
                    self.index += 1;
                    self.load(ctx);
                }
                if !self.files.is_empty() {
                    ui.label(format!("{} / {}", self.index + 1, self.files.len()));
                }
                ui.separator();
                let mut changed = ui
                    .checkbox(&mut self.options.rec601, "Use BT.601")
                    .changed();
                changed |= ui
                    .checkbox(&mut self.options.limited, "Video range")
                    .changed();
                changed |= ui
                    .checkbox(&mut self.options.flip, "Flip vertically")
                    .changed();
                if changed {
                    self.refresh(ctx);
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(t) = &self.texture {
                let ratio = ui.available_size() / t.size_vec2();
                let size = t.size_vec2() * ratio.x.min(ratio.y);
                let rect = egui::Rect::from_center_size(ui.max_rect().center(), size);
                ui.put(rect, egui::Image::new((t.id(), size)));
            } else {
                ui.centered_and_justified(|ui| ui.heading("Drop a .7th frame here"));
            }
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--preview") {
        if args.len() != 3 {
            return Err("Usage: frame-viewer --preview FRAME.7th PREVIEW.png".into());
        }
        let f = Frame::open(Path::new(&args[1]))?;
        let (w, h, rgb) = f.render(2048, Options::default());
        let mut enc = png::Encoder::new(fs::File::create_new(&args[2])?, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(&rgb)?;
        return Ok(());
    }
    eframe::run_native(
        "7th Frame Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1080., 680.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = Viewer::default();
            if let Some(p) = args.first() {
                app.open(&cc.egui_ctx, p.into());
            }
            Ok(Box::new(app))
        }),
    )?;
    Ok(())
}
