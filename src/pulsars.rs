use std::{io::{BufRead, BufReader}, path::PathBuf};

use arpa::{data_types::pulsar_meta::PulsarMeta, ARPAError};
use egui::RichText;
use egui_extras::{Column, TableBuilder};
use rayon::slice::ParallelSliceMut;

use crate::{
    app::{Request, Syncher}, 
    helpers::{
        downloader::{Downloader, DownloaderAction, FetchType}, 
        StatusMessage, StatusMessageSeverity,
        IconicButton, 
        confirm_button, table_header, 
        enter_data_option, format_data_option, format_unique_data_option, 
    }
};

const PULSAR_META_TABLE: [(&str, &str); 7] = [
    ("ID",      "The automatically generated ID."), 
    ("Alias",   "An alias for the pulsar, often the same as the J name"), 
    ("J name",  "Optional."), 
    ("B name",  "Optional."), 
    ("RA",      "J2000 right ascension, optional."), 
    ("DEC",     "J2000 declination, optional."), 
    (".par id", "Master ephemeride file id"), 
];

pub struct PulsarsApp {
    messages: Vec<StatusMessage>,

    downloader: Downloader<PulsarMeta>,

    // pulsars: Vec<PulsarMeta>,
    sort_by: usize,

    new_pulsar: PulsarMeta,
    pulsar_file: Option<PathBuf>,
}

impl PulsarsApp {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            downloader: Downloader::new(),
            
            // pulsars: Vec::new(),
            sort_by: 0,

            new_pulsar: PulsarMeta::null(),
            pulsar_file: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, archivist: &Syncher) {
        self.downloader.show(ctx);

        match self.downloader.action() {
            DownloaderAction::None => {},
            DownloaderAction::Delete(index) => match index {
                Some(id) => archivist.request(Request::DeletePulsar(id)),
                None => {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: "Something went wrong...".into(),
                    });
                },
            },

            DownloaderAction::Download(ft) => { 
                let request = match ft { 
                    FetchType::All => Request::DownloadAllPulsars,
                    FetchType::Id(id) => Request::DownloadPulsarById(id),
                };
                archivist.request(request);
            }
        }

        let response = egui::CentralPanel::default().show(ctx, |ui| 
            ui.scope_builder(
                egui::UiBuilder::new()
                .sense(egui::Sense::click()),
                |ui| egui::Frame::default()
                    .show(ui, |ui| self.body(ui, archivist))
            ).response
        ).inner;

        if response.clicked() {
            self.downloader.deselect();
        }
        
        ctx.input(|i| if let Some(df) = i.raw.dropped_files.first() {
            self.pulsar_file = df.path.clone();
        });

        // Handle input file
        if let Some(path) = self.pulsar_file.take() {
            let results = match self.read_pulsars_from_file(path) {
                Ok(rs) => rs,
                Err(err) => {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: err.to_string(),
                    });
                    return;
                },
            };

            for result in results {
                match result {
                    Ok(meta) => archivist.request(Request::AddPulsar(meta)),

                    Err(err) => self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: err.to_string(),
                    }),
                }
            }
        }
    }
    
    pub fn set_pulsars(&mut self, pulsars: Vec<PulsarMeta>) {
        *self.downloader.data_mut() = pulsars;
        self.reset_ui();
    }
    
    pub fn add_pulsar(&mut self, pulsar: PulsarMeta) {
        self.downloader.add(pulsar);
        self.reset_ui();
    }

    pub fn reset_ui(&mut self) {
        self.downloader.stop_fetching();
    }
    
    pub fn messages(&mut self) -> &mut Vec<StatusMessage> {
        &mut self.messages
    }
    
    pub fn deselect(&mut self) {
        self.downloader.deselect();
    }

    fn body(&mut self, ui: &mut egui::Ui, archivist: &Syncher) {
        ui.heading(RichText::new("Pulsars").strong());
        ui.add_space(12.0);
        
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            self.pulsar_data_entry(ui);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.pulsar_data_controls(ui, archivist);
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            self.pulsar_file_button(ui);
        });
            
        ui.separator();
        self.pulsar_table(ui);
    }

    fn pulsar_table(&mut self, ui: &mut egui::Ui) {
        if self.downloader.data().is_empty() {
            ui.label("No pulsars in memory!\n (Sync button below)");
            return;
        }

        let height = ui.available_height();
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .columns(Column::remainder(), 5)
            .column(Column::auto())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .sense(egui::Sense::click());

        table
        .header(24.0, |mut header| (0..7).for_each(|i| { 
            header.col(|ui| {
                let sort = table_header(
                    ui, 
                    PULSAR_META_TABLE[i].0, 
                    PULSAR_META_TABLE[i].1
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
                    
                    format_pulsar_meta(item, &mut row);
                    
                    if row.response().clicked() {
                        clicked = Some(index);
                    }
                });
            }

            if let Some(i) = clicked
            .map(|i| self.downloader.select(i))
            .flatten() {
                self.new_pulsar = self.downloader.data()[i].clone();
            }
        });
    }

    fn pulsar_data_entry(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("new_pulsar_grid")
        .num_columns(2)
        .spacing([32.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label( "Alias");
            ui.text_edit_singleline(&mut self.new_pulsar.alias);
            ui.end_row();
            
            ui.label("J name");
            enter_data_option(ui, &mut self.new_pulsar.j_name);
            ui.end_row();

            ui.label("B name");
            enter_data_option(ui, &mut self.new_pulsar.b_name);
            ui.end_row();
            
            ui.label( "RA");
            enter_data_option(ui, &mut self.new_pulsar.j2000_ra);
            ui.end_row();
            
            ui.label( "DEC");
            enter_data_option(ui, &mut self.new_pulsar.j2000_dec);
            ui.end_row();
        });
    }

    fn pulsar_data_controls(
        &mut self, 
        ui: &mut egui::Ui, 
        archivist: &Syncher
    ) {
        ui.vertical(|ui| {
            ui.add_space(6.0);
            let clear = ui.add(
                IconicButton::new("🗋")
                .on_hover_text("Clear fields")
            );

            ui.add_space(2.0);
            let new = ui.add(
                IconicButton::new("➕")
                .on_hover_text("Insert new")
            );
            ui.add_space(2.0);
            let overwrite = ui.add(
                IconicButton::new("📝")
                .enabled(self.downloader.selected().is_some())
                .on_hover_text("Overwrite")
            );
            
            if clear.clicked() { self.new_pulsar = PulsarMeta::null(); }

            if new.clicked() {
                if let Err(err) = self.new_pulsar.verify() {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: format!("Cannot add pulsar! {}", err),
                    });
                    return;
                }

                let meta = self.new_pulsar.clone();
                archivist.request(Request::AddPulsar(meta));
            }

            if confirm_button(overwrite, "Overwrite selected?") {
                if let Err(err) = self.new_pulsar.verify() {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: format!("Cannot overwrite pulsar! {}", err),
                    });
                    return;
                }

                let id = match self.downloader.selected() {
                    Some(i) => self.downloader.data()[i].id,
                    None => return,
                };
                let meta = self.new_pulsar.clone();
                archivist.request(Request::UpdatePulsar(id, meta));
            }
        });
    }

    fn pulsar_file_button(&mut self, ui: &mut egui::Ui) {
        let load = ui.add_sized(
            [
                ui.available_width() - 16.0,
                ui.available_height(),
            ], 
            egui::Button::new("Load file")
        );

        if load.clicked() {
            self.pulsar_file = rfd::FileDialog::new().pick_file();
        }        
    }

    fn read_pulsars_from_file(
        &mut self, 
        path: PathBuf
    ) -> Result<Vec<Result<PulsarMeta, ARPAError>>, ARPAError> {
        let reader = BufReader::new(std::fs::File::open(path)?);
        let results = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|l| 
            l.split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
        )
        .filter(|l| l
            .first()
            .map(|w| !w.starts_with("#"))
            .unwrap_or(false)
        )
        .map(|ws| PulsarMeta::from_strs(
            &ws.iter()
            .map(|w| w.as_str())
            .collect::<Vec<_>>()
        ))
        .collect();

        Ok(results)
    }

    fn sort_table(&mut self) {
        use arpa::data_types::pulsar_meta::PulsarMeta as PM;

        let compare = match self.sort_by {
            0 => |a: &PM, b: &PM| a.id.cmp(&b.id),
            1 => |a: &PM, b: &PM| a.alias.cmp(&b.alias),
            2 => |a: &PM, b: &PM| opt_cmp(&a.j_name, &b.j_name),
            3 => |a: &PM, b: &PM| opt_cmp(&a.b_name, &b.b_name),
            4 => |a: &PM, b: &PM| opt_cmp(&a.j2000_ra, &b.j2000_ra),
            5 => |a: &PM, b: &PM| opt_cmp(&a.j2000_dec, &b.j2000_dec),
            6 => |a: &PM, b: &PM| opt_cmp(
                &a.master_parfile_id, &b.master_parfile_id
            ),
            _ => return,
        };
        self.downloader.data_mut().par_sort_by(compare);
    }
}

fn format_pulsar_meta(
    psr: &PulsarMeta, 
    row: &mut egui_extras::TableRow<'_, '_>,
) {
    // id
    row.col(|ui| { ui.label(psr.id.to_string()); });

    // names
    row.col(|ui| { ui.label(RichText::new(&psr.alias).strong());});
    row.col(|ui| {
        ui.label(format_unique_data_option(&psr.j_name, &psr.alias));
    });
    row.col(|ui| { 
        ui.label(format_unique_data_option(&psr.b_name, &psr.alias)); 
    });
    // coords
    row.col(|ui| { 
        ui.label(format_data_option(&psr.j2000_ra));
    });
    row.col(|ui| { 
        ui.label(format_data_option(&psr.j2000_dec));
    });
    // par
    row.col(|ui| { ui.label(format_data_option(&psr.master_parfile_id)); });
}

fn opt_cmp<T>(
    a: &Option<T>, 
    b: &Option<T>,
) -> std::cmp::Ordering where T: Ord {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(av), Some(bv)) => av.cmp(bv),
    }
}
    