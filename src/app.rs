use eframe::App;
use egui::{
    CentralPanel, Color32, Image, Key, Label, Pos2, RichText, ScrollArea, Sense, SidePanel, Stroke,
    TopBottomPanel, Vec2, Window,
};
use eyre::{eyre, WrapErr};
use std::{
    fs::DirEntry,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RotationMode {
    Free,
    Line,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RtImageRotator {
    image_dir: String,
    image_ext: String,
    #[serde(skip)]
    image_dir_entries: Vec<DirEntry>,
    raw_dir: String,
    raw_ext: String,
    current_image_idx: usize,
    current_rotation: f32,
    rotation_mode: RotationMode,
    #[serde(skip)]
    drag_start: Option<Pos2>,
    conversion_command: String,
    #[serde(skip)]
    last_error: Option<eyre::Report>,
}

impl Default for RtImageRotator {
    fn default() -> Self {
        Self {
            image_dir: "".to_string(),
            image_ext: "jpg".to_string(),
            image_dir_entries: vec![],
            raw_dir: "".to_string(),
            raw_ext: "NEF".to_string(),
            current_image_idx: 0,
            current_rotation: 0.0,
            rotation_mode: RotationMode::Line,
            drag_start: None,
            conversion_command: String::new(),
            last_error: None,
        }
    }
}

impl RtImageRotator {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn select_image(&mut self, image_idx: usize) {
        self.current_image_idx = image_idx;
        self.current_rotation = 0.0;
    }

    fn generate_conversion_command(&mut self) -> eyre::Result<()> {
        let mut command =
            "parallel --delay 2 -j3 rawtherapee-cli -o converted -q -p {}.pp3.rot -j90 -js2 -Y -c {} ::: "
                .to_string();
        let image_names = std::fs::read_dir(&self.raw_dir)?
            .flatten()
            .filter(|e| e.path().extension().map(|e| e.to_str()) == Some(Some("rot")))
            .map(|e| {
                e.path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .expect("Cannot handle non UTF-8 file paths.")
                    .to_string()
            });
        for file_base_name in image_names {
            command.push_str(&file_base_name[..file_base_name.len() - 4]);
            command.push(' ');
        }
        command.push_str("\n\n# Optionally replace the original pp3 files with the modified ones:\n# for f in *.pp3.rot; do mv -- \"$f\" \"${f%.rot}\"; done");
        self.conversion_command = command;
        Ok(())
    }

    fn open_image_directory(&mut self) -> eyre::Result<()> {
        self.image_dir_entries = std::fs::read_dir(&self.image_dir)?
            .flatten()
            .filter(|e| {
                e.file_type().unwrap().is_file()
                    && e.path().extension().map(|e| {
                        e.to_str()
                            .expect("Cannot handle non UTF-8 file paths.")
                            .to_lowercase()
                    }) == Some(self.image_ext.to_lowercase())
            })
            .collect();
        self.image_dir_entries.sort_by_key(|e| {
            e.path()
                .to_str()
                .expect("Cannot handle non UTF-8 file paths.")
                .to_string()
        });
        self.select_image(self.current_image_idx);
        Ok(())
    }
}

impl App for RtImageRotator {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        if self.last_error.is_none() {
            ctx.input(|i| {
                if i.key_released(Key::ArrowUp)
                    && self.current_image_idx > 0
                    && self
                        .image_dir_entries
                        .get(self.current_image_idx - 1)
                        .is_some()
                {
                    self.select_image(self.current_image_idx - 1);
                }
                if i.key_released(Key::ArrowDown)
                    && self
                        .image_dir_entries
                        .get(self.current_image_idx + 1)
                        .is_some()
                {
                    self.select_image(self.current_image_idx + 1);
                }
            });
        }

        if self.last_error.is_some() {
            Window::new("Error!").collapsible(false).show(ctx, |ui| {
                ui.add(Label::new(format!("{:#}", self.last_error.as_ref().unwrap())).wrap(true));
                if ui.button("Ok").clicked() {
                    self.last_error = None;
                }
            });
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.set_enabled(self.last_error.is_none());
            ui.collapsing("Parameters", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Image directory");
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::singleline(&mut self.image_dir),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Image extension");
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::singleline(&mut self.image_ext),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Raw directory");
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::singleline(&mut self.raw_dir),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Raw extension");
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::singleline(&mut self.raw_ext),
                    );
                });
                if ui.button("Open image directory").clicked() {
                    if let Err(e) = self.open_image_directory() {
                        self.last_error = Some(e);
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Rotation mode:");
                ui.radio_value(&mut self.rotation_mode, RotationMode::Line, "Line");
                ui.radio_value(&mut self.rotation_mode, RotationMode::Free, "Free");
            });
        });
        SidePanel::left("image_files").show(ctx, |ui| {
            ui.set_enabled(self.last_error.is_none());
            ScrollArea::vertical().show(ui, |ui| {
                let mut new_image_idx = None;
                for (i, file) in self.image_dir_entries.iter().enumerate() {
                    let (i, label_ctx) = if i == self.current_image_idx {
                        (
                            i,
                            ui.add(
                                Label::new(
                                    RichText::new(
                                        file.path()
                                            .file_name()
                                            .unwrap()
                                            .to_str()
                                            .expect("Cannot handle non UTF-8 file paths."),
                                    )
                                    .strong(),
                                )
                                .sense(Sense::click()),
                            ),
                        )
                    } else {
                        (
                            i,
                            ui.add(
                                Label::new(RichText::new(
                                    file.path()
                                        .file_name()
                                        .unwrap()
                                        .to_str()
                                        .expect("Cannot handle non UTF-8 file paths."),
                                ))
                                .sense(Sense::click()),
                            ),
                        )
                    };
                    if label_ctx.clicked() {
                        new_image_idx = Some(i);
                    }
                }
                if let Some(i) = new_image_idx {
                    self.select_image(i);
                }
            })
        });
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.set_enabled(self.last_error.is_none());
            ui.label(format!("rotation: {}", self.current_rotation));
            if ui.button("Reset rotation").clicked() {
                self.current_rotation = 0.;
            }
            ui.separator();
            if ui.button("Apply additional rotation").clicked() {
                let mut pp3_path = PathBuf::new();
                pp3_path.push(&self.raw_dir);
                pp3_path.push(
                    self.image_dir_entries[self.current_image_idx]
                        .path()
                        .file_stem()
                        .unwrap(),
                );
                pp3_path.set_extension(format!("{}.pp3", self.raw_ext));

                if let Err(e) = apply_additional_rotation_to_pp3(&pp3_path, self.current_rotation) {
                    self.last_error = Some(e);
                }
            }
            ui.separator();
            ui.collapsing("Conversion", |ui| {
                if ui.button("Generate conversion command").clicked() {
                    if let Err(e) = self.generate_conversion_command() {
                        self.last_error = Some(e);
                    }
                }
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_size().x, 60.],
                        egui::TextEdit::multiline(&mut self.conversion_command),
                    );
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(self.last_error.is_none());
            if let Some(dir_entry) = self.image_dir_entries.get(self.current_image_idx) {
                let img_resp = ui.add(
                    Image::new(format!(
                        "file://{}",
                        dir_entry
                            .path()
                            .to_str()
                            .expect("Cannot handle non UTF-8 file paths.")
                    ))
                    .rotate(f32::to_radians(self.current_rotation), Vec2::splat(0.5))
                    .sense(Sense::drag()),
                );
                match self.rotation_mode {
                    RotationMode::Free => {
                        if img_resp.dragged() {
                            let delta = img_resp.drag_delta();
                            self.current_rotation += delta.y * 0.1;
                            if self.current_rotation < -45.0 {
                                self.current_rotation = -45.0;
                            } else if self.current_rotation > 45.0 {
                                self.current_rotation = 45.0;
                            }
                        }
                    }
                    RotationMode::Line => {
                        if img_resp.drag_started() {
                            self.drag_start = img_resp.interact_pointer_pos();
                        } else if img_resp.drag_released() {
                            if let Some(drag_start) = self.drag_start.take() {
                                let mut drag_rotation = (drag_start.to_vec2()
                                    - img_resp
                                        .interact_pointer_pos()
                                        .expect(
                                            "There should always be a valid position at this point.",
                                        )
                                        .to_vec2())
                                .angle()
                                .to_degrees();

                                if drag_rotation < 0. {
                                    drag_rotation += 180.;
                                }

                                if drag_rotation < 40. {
                                    self.current_rotation -= drag_rotation;
                                } else if drag_rotation > 140. {
                                    self.current_rotation += 180. - drag_rotation;
                                } else if drag_rotation > 50. && drag_rotation < 130. {
                                    self.current_rotation += 90. - drag_rotation;
                                }
                            }
                        }
                    }
                }
                if let (Some(drag_start), Some(pointer_pos)) =
                    (self.drag_start, img_resp.interact_pointer_pos())
                {
                    ui.painter().line_segment(
                        [drag_start, pointer_pos],
                        Stroke {
                            width: 1.,
                            color: Color32::BLACK,
                        },
                    );
                }
            }
        });
    }
}

fn apply_additional_rotation_to_pp3(path: &Path, additional_rotation: f32) -> eyre::Result<()> {
    let mut pp3: Vec<String> = std::fs::read_to_string(path)?
        .lines()
        .map(|s| s.to_string())
        .collect();

    let mut in_section = false;
    for line in pp3.iter_mut() {
        if !in_section && line.contains("[Rotation]") {
            in_section = true;
        } else if in_section && line.starts_with("Degree=") {
            let pp3_rotation = line
                .split_once('=')
                .ok_or(eyre!("Line invalid"))
                .map(|(_, deg)| deg.parse::<f32>().wrap_err("Rotation invalid"))??;
            *line = format!("Degree={}", pp3_rotation - additional_rotation);
            break;
        }
    }
    let mut output_pp3 = path.to_owned();
    output_pp3.set_extension("pp3.rot");

    std::fs::write(&output_pp3, pp3.join("\n"))?;
    Ok(())
}
