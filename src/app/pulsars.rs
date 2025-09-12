use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};

use arpa::{ARPAError, data_types::PulsarMeta};
use egui::RichText;

use crate::app::{
    Request, Syncher,
    helpers::{
        ICON_CLEAR, ICON_INSERT, ICON_WRITE, IconicButton, StatusMessage,
        StatusMessageSeverity, confirm_button,
        downloader::{self, Downloader, DownloaderAction},
        enter_data_option, format_data_option, format_unique_data_option,
        opt_cmp,
    },
};
const DATA_TYPE: crate::app::DataType = crate::app::DataType::Pulsar;

impl downloader::Item for PulsarMeta {
    const NAME: &str = "pulsar";
    const COLUMNS: &[(&str, &str)] = &[
        ("ID", "The automatically generated ID."),
        (
            "Alias",
            "An alias for the pulsar, often the same as the J name",
        ),
        ("J name", "Optional."),
        ("B name", "Optional."),
        ("RA", "J2000 right ascension, optional."),
        ("DEC", "J2000 declination, optional."),
        (".par id", "Master ephemeride file id"),
    ];

    fn id(&self) -> i32 {
        self.id
    }

    fn format(&self, row: &mut egui_extras::TableRow) {
        // id
        row.col(|ui| {
            ui.label(self.id.to_string());
        });

        // names
        row.col(|ui| {
            ui.label(RichText::new(&self.alias).strong());
        });
        row.col(|ui| {
            ui.label(format_unique_data_option(
                self.j_name.as_ref(),
                &self.alias,
            ));
        });
        row.col(|ui| {
            ui.label(format_unique_data_option(
                self.b_name.as_ref(),
                &self.alias,
            ));
        });
        // coords
        row.col(|ui| {
            ui.label(format_data_option(self.j2000_ra.as_ref()));
        });
        row.col(|ui| {
            ui.label(format_data_option(self.j2000_dec.as_ref()));
        });
        // par
        row.col(|ui| {
            ui.label(format_data_option(self.master_parfile_id.as_ref()));
        });
    }

    fn cmp_by(&self, other: &Self, index: usize) -> std::cmp::Ordering {
        match index {
            0 => self.id.cmp(&other.id),
            1 => self.alias.cmp(&other.alias),
            2 => opt_cmp(self.j_name.as_ref(), other.j_name.as_ref()),
            3 => opt_cmp(self.b_name.as_ref(), other.b_name.as_ref()),
            4 => opt_cmp(self.j2000_ra.as_ref(), other.j2000_ra.as_ref()),
            5 => opt_cmp(self.j2000_dec.as_ref(), other.j2000_dec.as_ref()),
            6 => opt_cmp(
                self.master_parfile_id.as_ref(),
                other.master_parfile_id.as_ref(),
            ),
            _ => std::cmp::Ordering::Equal,
        }
    }
}

pub struct PulsarsApp {
    messages: Vec<StatusMessage>,
    pub downloader: Downloader<PulsarMeta>,

    new_pulsar: PulsarMeta,
    pulsar_file: Option<PathBuf>,
}

impl PulsarsApp {
    pub const fn new() -> Self {
        Self {
            messages: Vec::new(),
            downloader: Downloader::new(),

            new_pulsar: PulsarMeta::null(),
            pulsar_file: None,
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

                None => {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Warning,
                        message: "Something went wrong...".into(),
                    });
                }
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
                self.pulsar_file.clone_from(&df.path);
            }
        });

        // Handle input file
        if let Some(path) = self.pulsar_file.take() {
            let results = match Self::read_pulsars_from_file(path) {
                Ok(rs) => rs,
                Err(err) => {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: err.to_string(),
                    });
                    return;
                }
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
        // self.pulsar_table(ui);
        let selected = self.downloader.table(ui);
        if let Some(i) = selected {
            self.new_pulsar = self.downloader.data()[i].clone();
        }
    }

    fn pulsar_data_entry(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("new_pulsar_grid")
            .num_columns(2)
            .spacing([32.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Alias");
                ui.text_edit_singleline(&mut self.new_pulsar.alias);
                ui.end_row();

                ui.label("J name");
                enter_data_option(ui, &mut self.new_pulsar.j_name);
                ui.end_row();

                ui.label("B name");
                enter_data_option(ui, &mut self.new_pulsar.b_name);
                ui.end_row();

                ui.label("RA");
                enter_data_option(ui, &mut self.new_pulsar.j2000_ra);
                ui.end_row();

                ui.label("DEC");
                enter_data_option(ui, &mut self.new_pulsar.j2000_dec);
                ui.end_row();
            });
    }

    fn pulsar_data_controls(&mut self, ui: &mut egui::Ui, archivist: &Syncher) {
        ui.vertical(|ui| {
            ui.add_space(6.0);
            let clear = ui.add(
                IconicButton::new(ICON_CLEAR).on_hover_text("Clear fields"),
            );

            ui.add_space(2.0);
            let new = ui.add(
                IconicButton::new(ICON_INSERT).on_hover_text("Insert new"),
            );
            ui.add_space(2.0);
            let overwrite = ui.add(
                IconicButton::new(ICON_WRITE)
                    .enabled(self.downloader.selected().is_some())
                    .on_hover_text("Overwrite"),
            );

            if clear.clicked() {
                self.new_pulsar = PulsarMeta::null();
            }

            if new.clicked() {
                if let Err(err) = self.new_pulsar.verify() {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: format!("Cannot add pulsar! {err}"),
                    });
                    return;
                }

                let meta = self.new_pulsar.clone();
                archivist.request(Request::AddPulsar(meta));
            }

            if confirm_button(&overwrite, "Overwrite selected?") {
                if let Err(err) = self.new_pulsar.verify() {
                    self.messages.push(StatusMessage {
                        severity: StatusMessageSeverity::Error,
                        message: format!("Cannot overwrite pulsar! {err}"),
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
            [ui.available_width() - 16.0, ui.available_height()],
            egui::Button::new("Load file"),
        );

        if load.clicked() {
            self.pulsar_file = rfd::FileDialog::new().pick_file();
        }
    }

    fn read_pulsars_from_file(
        path: PathBuf,
    ) -> Result<Vec<Result<PulsarMeta, ARPAError>>, ARPAError> {
        let reader = BufReader::new(std::fs::File::open(path)?);
        let results = reader
            .lines()
            .map_while(std::result::Result::ok)
            .map(|l| {
                l.split_whitespace().map(str::to_string).collect::<Vec<_>>()
            })
            .filter(|l| l.first().is_some_and(|w| !w.starts_with('#')))
            .map(|ws| {
                PulsarMeta::from_strs(
                    &ws.iter().map(String::as_str).collect::<Vec<_>>(),
                )
            })
            .collect();

        Ok(results)
    }

    pub(crate) fn select_with_id(&mut self, id: i32) {
        let data = self.downloader.data();
        for (index, item) in data.iter().enumerate() {
            if item.id == id {
                self.downloader.select(index);
                return;
            }
        }
    }
}
