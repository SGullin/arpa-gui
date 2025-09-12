use std::path::PathBuf;

use egui::RichText;
use egui_extras::{Column, TableBuilder};
use rayon::prelude::ParallelSliceMut;

use crate::{
    app::{DataType, Request, Syncher,
        helpers::{
            confirm_button, table_header, 
            downloader::{Downloader, DownloaderAction, FetchType}, 
            IconicButton, StatusMessage, StatusMessageSeverity, 
            ICON_INSERT, ICON_OPEN, ICON_WRITE
        },
    },
};

const PAR_META_TABLE: [(&str, &str); 4] = [
    ("ID", "The automatically generated ID."),
    ("Pulsar", "The name of the pulsar referred to."),
    ("Pulsar ID", "The ID of the pulsar referred to."),
    ("Path", "The path to the file."),
];

#[derive(Debug)]
pub(crate) struct ParData {
    pub id: i32, 
    pub pulsar_id: i32,
    pub pulsar_name: String,
    pub path: String,
}
impl super::helpers::downloader::Item for ParData {
    fn id(&self) -> i32 {
        self.id
    }
}

pub(crate) struct EphemerideApp {
    messages: Vec<StatusMessage>,
    pub downloader: Downloader<ParData>,

    sort_by: usize,
    new_par: Option<PathBuf>,
    new_par_pid: String,
    new_par_mastery: bool,

    move_to_pulsar_id: Option<i32>,
}

impl EphemerideApp {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            downloader: Downloader::new(),

            sort_by: 0,
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
                Some(id) => archivist.request(
                    Request::DeleteItem(DataType::TOA, id)
                ),
                None => self.messages.push(StatusMessage {
                    severity: StatusMessageSeverity::Warning,
                    message: "Something went wrong...".into(),
                }),
            },

            DownloaderAction::Download(ft) => {
                let request = match ft {
                    FetchType::All => Request::DownloadAllEphemerides,
                    FetchType::Id(id) => Request::DownloadEphemerideById(id),
                };
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
                self.new_par = df.path.clone();
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
        self.par_table(ui);
    }

    fn par_table(&mut self, ui: &mut egui::Ui) {
        if self.downloader.data().is_empty() {
            ui.label("No ephemerides in memory!\n (Sync button below)");
            return;
        }

        let height = ui.available_height();
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .columns(Column::remainder(), 2)
            .column(Column::auto())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .sense(egui::Sense::click());

        table
        .header(24.0, |mut header| 
            PAR_META_TABLE
            .iter()
            .enumerate()
            .for_each(|(i, info)| { header.col(|ui| {
                let sort = table_header(
                    ui,
                    info.0,
                    info.1
                );

                if sort {
                    self.sort_by = i;
                    self.sort_table();
                }
            }); }
        ))
        .body(|mut body| {
            let mut clicked = None;
            let mut secondary_click = false;
            for (index, item) in self.downloader.data().iter().enumerate() {
                body.row(18.0, |mut row| {
                    row.set_selected(self.downloader.selected() == Some(index));

                    format_par_data(item, &mut row);

                    if row.response().clicked() {
                        clicked = Some(index);
                    }
                    row.response().context_menu(|ui|
                        if ui.button("⬉ Select pulsar").clicked() {
                            clicked = Some(index);
                            secondary_click = true;
                        }
                    );
                });
            }

            if let Some(i) = clicked
            .map(|i| self.downloader.select(i))
            .flatten() {
                self.new_par = Some(PathBuf::from(&self.downloader.data()[i].path));
                let pid = self.downloader.data()[i].id;
                self.new_par_pid = pid.to_string();
                if secondary_click {
                    self.move_to_pulsar_id = Some(pid);
                }
            }
        });
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
                }
                else {
                    ui.label(RichText::new("Choose or drop a file").italics());
                }
                ui.end_row();

                ui.label("Pulsar ID or alias");
                ui.text_edit_singleline(&mut self.new_par_pid);
                
                ui.end_row();

                ui.label("Master")
                    .on_hover_text("Whether or not this should be set as the \
                        pulsar's master ephemeride.");
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.new_par_mastery, "");
                });
            });
    }
    
    fn par_data_controls(&mut self, ui: &mut egui::Ui, archivist: &Syncher) {
        ui.horizontal(|ui| {
            let load = ui.add(
                IconicButton::new(ICON_OPEN)
                .on_hover_text("Load file")
            );
            if load.clicked() {
                self.new_par = rfd::FileDialog::new().pick_file();
            }
    
            let insert = ui.add(
                IconicButton::new(ICON_INSERT)
                .enabled(self.new_par.is_some() && !self.new_par_pid.is_empty())
                .on_hover_text("Insert new ephemeride")
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
                .enabled(self.new_par.is_some() && !self.new_par_pid.is_empty())
                .on_hover_text("Overwrite ephemeride")
            );
            if confirm_button(overwrite, "Overwrite selected?") {
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

    fn sort_table(&mut self) {
        use ParData as PD;

        let compare = match self.sort_by {
            0 => |a: &PD, b: &PD| a.id.cmp(&b.id),
            1 => |a: &PD, b: &PD| a.pulsar_name.cmp(&b.pulsar_name),
            2 => |a: &PD, b: &PD| a.pulsar_id.cmp(&b.pulsar_id),
            3 => |a: &PD, b: &PD| a.path.cmp(&b.path),
            _ => return,
        };
        self.downloader.data_mut().par_sort_by(compare);
    }
    
    pub(crate) fn select_pulsar(&mut self) -> Option<i32> {
        self.move_to_pulsar_id.take()
    }
    
    pub(crate) fn selected(&self) -> Option<i32> {
        self.downloader.selected_id()
    }
}

fn format_par_data(
    item: &ParData, 
    row: &mut egui_extras::TableRow<'_, '_>
) {
    // id
    row.col(|ui| {
        ui.label(item.id.to_string());
    });

    // pulsar
    row.col(|ui| {
        ui.label(&item.pulsar_name);
    });
    row.col(|ui| {
        ui.label(item.pulsar_id.to_string());
    });

    // path
    row.col(|ui| {
        ui.label(item.path.to_string());
    });
}
