use egui::RichText;
use std::path::PathBuf;

use crate::app::{
    Request, Syncher,
    helpers::{
        ICON_INSERT, ICON_OPEN, ICON_WRITE, IconicButton, StatusMessage,
        StatusMessageSeverity, confirm_button,
        downloader::{Downloader, DownloaderAction},
    },
};

const DATA_TYPE: crate::app::DataType = crate::app::DataType::Ephemeride;

#[derive(Debug)]
pub struct ParData {
    pub id: i32,
    pub pulsar_id: i32,
    pub pulsar_name: String,
    pub path: String,
}
impl super::helpers::downloader::Item for ParData {
    const NAME: &str = "ephemerides";
    const COLUMNS: &[(&str, &str)] = &[
        ("ID", "The automatically generated ID."),
        ("Pulsar", "The name of the pulsar referred to."),
        ("Pulsar ID", "The ID of the pulsar referred to."),
        ("Path", "The path to the file."),
    ];

    fn id(&self) -> i32 {
        self.id
    }

    fn cmp_by(&self, other: &Self, index: usize) -> std::cmp::Ordering {
        match index {
            0 => self.id.cmp(&other.id),
            1 => self.pulsar_name.cmp(&other.pulsar_name),
            2 => self.pulsar_id.cmp(&other.pulsar_id),
            3 => self.path.cmp(&other.path),
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn format(&self, row: &mut egui_extras::TableRow) {
        // id
        row.col(|ui| {
            ui.label(self.id.to_string());
        });

        // pulsar
        row.col(|ui| {
            ui.label(&self.pulsar_name);
        });
        row.col(|ui| {
            ui.label(self.pulsar_id.to_string());
        });

        // path
        row.col(|ui| {
            ui.label(self.path.to_string());
        });
    }
}

pub struct EphemerideApp {
    messages: Vec<StatusMessage>,
    pub downloader: Downloader<ParData>,

    new_par: Option<PathBuf>,
    new_par_pid: String,
    new_par_mastery: bool,

    move_to_pulsar_id: Option<i32>,
}

impl EphemerideApp {
    pub const fn new() -> Self {
        Self {
            messages: Vec::new(),
            downloader: Downloader::new(),

            new_par: None,
            new_par_pid: String::new(),
            new_par_mastery: false,

            move_to_pulsar_id: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, archivist: &Syncher) {
        self.downloader.action_bar(ctx);

        match self.downloader.action() {
            DownloaderAction::None => {}
            DownloaderAction::Delete(index) => match index {
                Some(id) => {
                    archivist.request(Request::DeleteItem(DATA_TYPE, id));
                }
                None => self.messages.push(StatusMessage {
                    severity: StatusMessageSeverity::Warning,
                    message: "Something went wrong...".into(),
                }),
            },

            DownloaderAction::Download(ft) => {
                let request = Request::Download(DATA_TYPE, ft);
                archivist.request(request);
            }
        }

        let response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.scope_builder(
                    egui::UiBuilder::new().sense(egui::Sense::click()),
                    |ui| {
                        egui::Frame::default()
                            .show(ui, |ui| self.body(ui, archivist))
                    },
                )
                .response
            })
            .inner;

        if response.clicked() {
            self.downloader.deselect();
        }

        ctx.input(|i| {
            if let Some(df) = i.raw.dropped_files.first() {
                self.new_par.clone_from(&df.path);
            }
        });
    }

    fn body(&mut self, ui: &mut egui::Ui, archivist: &Syncher) {
        ui.heading(RichText::new("Ephemerides").strong());
        ui.add_space(12.0);

        ui.add_space(16.0);
        self.par_data_entry(ui);
        self.par_data_controls(ui, archivist);

        ui.separator();
        self.downloader.table(ui);
    }

    fn par_data_entry(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("new_par_grid")
            .num_columns(2)
            .spacing([32.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("File");
                if let Some(p) = &self.new_par {
                    ui.label(p.display().to_string());
                } else {
                    ui.label(RichText::new("Choose or drop a file").italics());
                }
                ui.end_row();

                ui.label("Pulsar ID or alias");
                ui.text_edit_singleline(&mut self.new_par_pid);

                ui.end_row();

                ui.label("Master").on_hover_text(
                    "Whether or not this should be set as the \
                        pulsar's master ephemeride.",
                );
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.new_par_mastery, "");
                });
            });
    }

    fn par_data_controls(&mut self, ui: &mut egui::Ui, archivist: &Syncher) {
        ui.horizontal(|ui| {
            let load =
                ui.add(IconicButton::new(ICON_OPEN).on_hover_text("Load file"));
            if load.clicked() {
                self.new_par = rfd::FileDialog::new().pick_file();
            }

            let insert = ui.add(
                IconicButton::new(ICON_INSERT)
                    .enabled(
                        self.new_par.is_some() && !self.new_par_pid.is_empty(),
                    )
                    .on_hover_text("Insert new ephemeride"),
            );
            if insert.clicked() {
                let Some(path) = self.new_par.clone() else {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Warning,
                        message: "Something went wrong.".into(),
                    });
                    return;
                };

                archivist.request(Request::AddPar {
                    path,
                    pulsar: self.new_par_pid.clone(),
                    master: self.new_par_mastery,
                });
            }

            let overwrite = ui.add(
                IconicButton::new(ICON_WRITE)
                    .enabled(
                        self.new_par.is_some() && !self.new_par_pid.is_empty(),
                    )
                    .on_hover_text("Overwrite ephemeride"),
            );
            if confirm_button(&overwrite, "Overwrite selected?") {
                let Some(path) = self.new_par.clone() else {
                    self.messages.push(StatusMessage::wrong());
                    return;
                };

                let Some(id) = self.downloader.selected_id() else {
                    self.messages.push(StatusMessage::wrong());
                    return;
                };

                archivist.request(Request::UpdatePar {
                    id,
                    path,
                    pulsar: self.new_par_pid.clone(),
                    master: self.new_par_mastery,
                });
            }
        });
    }

    pub fn reset_ui(&mut self) {
        self.downloader.stop_fetching();
    }

    pub fn deselect(&mut self) {
        self.downloader.deselect();
    }

    pub(crate) fn select_pulsar(&mut self) -> Option<i32> {
        self.move_to_pulsar_id.take()
    }

    pub(crate) fn selected(&self) -> Option<i32> {
        self.downloader.selected_id()
    }
}
