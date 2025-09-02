use std::path::PathBuf;

use arpa::data_types::ParMeta;
use egui::RichText;
use egui_extras::{Column, TableBuilder};
use rayon::prelude::ParallelSliceMut;

use crate::{
    app::{Request, Syncher},
    helpers::{
        downloader::{Downloader, DownloaderAction, FetchType}, table_header, IconicButton, StatusMessage, StatusMessageSeverity
    },
};

const PAR_META_TABLE: [(&str, &str); 3] = [
    ("ID", "The automatically generated ID."),
    ("Pulsar", "The ID of the pulsar referred to."),
    ("Path", "The path to the file."),
];

pub struct EphemerideApp {
    messages: Vec<StatusMessage>,
    downloader: Downloader<ParMeta>,

    sort_by: usize,
    new_par: Option<PathBuf>,
    new_par_pid: String,
}

impl EphemerideApp {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            downloader: Downloader::new(),

            sort_by: 0,
            new_par: None,
            new_par_pid: String::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, archivist: &Syncher) {
        self.downloader.show(ctx);

        match self.downloader.action() {
            DownloaderAction::None => {}
            DownloaderAction::Delete(index) => match index {
                Some(id) => archivist.request(Request::DeletePulsar(id)),
                None => self.messages.push(StatusMessage {
                    severity: StatusMessageSeverity::Error,
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

        // ui.horizontal(|ui| {
            // ui.add_space(16.0);
            self.par_data_entry(ui);
            // ui.add_space(8.0);
        //     ui.separator();
        //     ui.add_space(8.0);
            // self.par_data_controls(ui, archivist);
        //     ui.add_space(8.0);
        // });

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
            .column(Column::remainder())
            .column(Column::auto())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .sense(egui::Sense::click());

        table
        .header(24.0, |mut header| (0..7).for_each(|i| {
            header.col(|ui| {
                let sort = table_header(
                    ui,
                    PAR_META_TABLE[i].0,
                    PAR_META_TABLE[i].1
                );

                if sort {
                    self.sort_by = i;
                    self.sort_table();
                }
            });
        }))
        .body(|mut body| {
            let mut clicked = None;
            for (index, item) in self.downloader.data().iter().enumerate() {
                body.row(18.0, |mut row| {
                    row.set_selected(self.downloader.selected() == Some(index));

                    format_par_meta(item, &mut row);

                    if row.response().clicked() {
                        clicked = Some(index);
                    }
                });
            }

            // if let Some(i) = clicked
            // .map(|i| self.downloader.select(i))
            // .flatten() {
            //     self.new_pulsar = self.downloader.data()[i].clone();
            // }
        });
    }

    pub fn set_pars(&mut self, pars: Vec<ParMeta>) {
        *self.downloader.data_mut() = pars;
        self.downloader.stop_fetching();
    }

    pub fn add_par(&mut self, par: ParMeta) {
        self.downloader.data_mut().push(par);
    }

    fn sort_table(&mut self) {
        use ParMeta as PM;

        let compare = match self.sort_by {
            0 => |a: &PM, b: &PM| a.id.cmp(&b.id),
            1 => |a: &PM, b: &PM| a.pulsar_id.cmp(&b.pulsar_id),
            2 => |a: &PM, b: &PM| a.file_path.cmp(&b.file_path),
            _ => return,
        };
        self.downloader.data_mut().par_sort_by(compare);
    }
    
    fn par_data_entry(&mut self, ui: &mut egui::Ui) {
        let load = ui.add(
            IconicButton::new("🗁")
            .on_hover_text("Load file")
        );

        if load.clicked() {
            self.new_par = rfd::FileDialog::new().pick_file();
        }

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
            });
    }
}

fn format_par_meta(
    item: &ParMeta, 
    row: &mut egui_extras::TableRow<'_, '_>
) {
    // id
    row.col(|ui| {
        ui.label(item.id.to_string());
    });

    // pid
    row.col(|ui| {
        ui.label(item.pulsar_id.to_string());
    });

    // path
    row.col(|ui| {
        ui.label(item.file_path.to_string());
    });
}
