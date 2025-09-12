use egui::RichText;

use crate::app::{
    Request, Syncher,
    helpers::{
        StatusMessage, StatusMessageSeverity,
        downloader::{self, Downloader, DownloaderAction},
    },
};

const DATA_TYPE: crate::app::DataType = crate::app::DataType::Toa;

#[derive(Debug)]
pub struct TOAData {
    pub id: i32,
    pub process: i32,
    pub pulsar: String,
    pub observer: i32,
    pub template: i32,
    pub frequency: f32,
    pub time: f64,
    pub error: f32,
}

impl downloader::Item for TOAData {
    const NAME: &str = "TOA";
    const COLUMNS: &[(&str, &str)] = &[
        ("Pc.", "The ID of the process."),
        ("Pulsar", "The ID of the pulsar."),
        ("Time", "The arrival time."),
        ("Error", "The error in the time."),
        ("Ob.", "The ID of the observer."),
        ("Tm.", "The ID of the template used."),
        ("Frequency", "The observing frequency."),
    ];

    fn id(&self) -> i32 {
        self.id
    }

    fn cmp_by(&self, other: &Self, index: usize) -> std::cmp::Ordering {
        match index {
            0 => self.process.cmp(&other.process),
            1 => self.pulsar.cmp(&other.pulsar),
            2 => self.time.total_cmp(&other.time),
            3 => self.error.total_cmp(&other.error),
            4 => self.template.cmp(&other.template),
            5 => self.observer.cmp(&other.observer),
            6 => self.frequency.total_cmp(&other.frequency),
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn format(&self, row: &mut egui_extras::TableRow) {
        row.col(|ui| {
            ui.label(self.process.to_string());
        });
        row.col(|ui| {
            ui.label(&self.pulsar);
        });
        row.col(|ui| {
            ui.label(self.error.to_string());
        });
        row.col(|ui| {
            ui.label(self.time.to_string());
        });
        row.col(|ui| {
            ui.label(self.template.to_string());
        });
        row.col(|ui| {
            ui.label(self.observer.to_string());
        });
        row.col(|ui| {
            ui.label(self.frequency.to_string());
        });
    }
}

pub struct TOAsApp {
    pub downloader: Downloader<TOAData>,
    messages: Vec<StatusMessage>,
}
impl TOAsApp {
    pub const fn new() -> Self {
        Self {
            downloader: Downloader::new(),
            messages: Vec::new(),
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
                archivist.request(Request::Download(DATA_TYPE, ft));
            }
        }

        let response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.scope_builder(
                    egui::UiBuilder::new().sense(egui::Sense::click()),
                    |ui| egui::Frame::default().show(ui, |ui| self.body(ui)),
                )
                .response
            })
            .inner;

        if response.clicked() {
            self.downloader.deselect();
        }
    }

    pub fn deselect(&mut self) {
        self.downloader.deselect();
    }

    fn body(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("TOAs").strong());
        ui.add_space(12.0);

        ui.separator();
        self.downloader.table(ui);
    }
}
