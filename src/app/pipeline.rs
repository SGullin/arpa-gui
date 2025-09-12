use std::mem::replace;

use arpa::{
    conveniences::display_elapsed_time,
    data_types::{ParMeta, RawMeta, TemplateMeta},
    pipeline::Status,
};
use egui::{Button, Context, RichText};

use crate::app::{
    Request, Syncher,
    ephemerides::EphemerideApp,
    helpers::{
        ICON_ARROW, ICON_CHECK, ICON_CLEAR, ICON_CROSS, ICON_RUN, ICON_WRITE,
        IconicButton, MISSING_DATA,
    },
};

#[derive(Debug, Default)]
struct RunInfo {
    status: Status,
    errored: bool,
    generated_toas: Option<usize>,
    archived_toas: Option<usize>,
    diagnosed: (usize, Vec<(String, bool)>),
    archived_plots: Option<usize>,
    done: Option<std::time::Duration>,
}

const MESSAGES: &[&str] = &[
    "Preparing",
    "Copying file",
    "Installing ephemeride",
    "Manipulating",
    "Verifying template",
    "Generating TOAs",
    "Logging process",
    "Parsing TOA info",
    "Running diagnostics",
    "Finished!",
];

#[derive(Debug)]
enum PipeStage {
    Invalid,
    Relaxed {
        raw: String,
        ephemeride: i32,
        template: i32,
    },
    SettingUp {
        raw: String,
        ephemeride: i32,
        template: i32,
    },
    SetUp {
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
    },

    Running(RunInfo),
}

impl Default for PipeStage {
    fn default() -> Self {
        Self::Relaxed {
            raw: String::new(),
            ephemeride: 0,
            template: 0,
        }
    }
}

pub struct PipelineApp {
    state: PipeStage,
}

impl PipelineApp {
    pub(crate) fn new() -> Self {
        Self {
            state: PipeStage::default(),
        }
    }

    pub(crate) fn show(
        &mut self,
        ctx: &Context,
        archivist: &Syncher,
        ephemeride_app: &EphemerideApp,
    ) {
        let state = replace(&mut self.state, PipeStage::Invalid);

        egui::CentralPanel::default().show(ctx, |ui| match state {
            PipeStage::Invalid => self.state = PipeStage::default(),

            PipeStage::Relaxed {
                mut raw,
                mut ephemeride,
                mut template,
            } => {
                Self::argument_entry(
                    ui,
                    ephemeride_app,
                    &mut raw,
                    &mut ephemeride,
                    &mut template,
                    true,
                );
                self.relaxed_buttons(archivist, ui, raw, ephemeride, template);
            }

            PipeStage::SettingUp {
                mut raw,
                mut ephemeride,
                mut template,
            } => {
                Self::argument_entry(
                    ui,
                    ephemeride_app,
                    &mut raw,
                    &mut ephemeride,
                    &mut template,
                    false,
                );
                Self::setting_up_buttons(ui);
                self.state = PipeStage::SettingUp {
                    raw,
                    ephemeride,
                    template,
                };
            }

            PipeStage::SetUp {
                raw,
                ephemeride,
                template,
            } => {
                Self::set_up_field(ui, &raw, ephemeride.as_ref(), &template);
                self.running_buttons(archivist, ui, raw, ephemeride, template);
            }

            PipeStage::Running(info) => {
                self.running(ui, &info);
                self.state = PipeStage::Running(info);
            }
        });
    }

    pub(crate) fn set_up(
        &mut self,
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
    ) {
        self.state = PipeStage::SetUp {
            raw,
            ephemeride,
            template,
        }
    }

    fn argument_entry(
        ui: &mut egui::Ui,
        ephemeride_app: &EphemerideApp,
        raw: &mut String,
        ephemeride: &mut i32,
        template: &mut i32,
        active: bool,
    ) {
        egui::Grid::new("args")
            .num_columns(2)
            .spacing([32.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.label("Path / ID");
                ui.end_row();

                // ----------------------------------------------------------------
                ui.label("Raw file");
                ui.add_enabled(active, egui::TextEdit::singleline(raw));
                ui.end_row();

                // ----------------------------------------------------------------
                ui.label("Ephemeride");
                ui.add_enabled(
                    active,
                    egui::DragValue::new(ephemeride).range(0..=i32::MAX),
                );

                let select_par = IconicButton::new(ICON_ARROW)
                    .on_hover_text("Grab selected from ephemeride tab.");

                if let Some(id) = ephemeride_app.selected() {
                    let select_par = ui.add(select_par.enabled(true));
                    if select_par.clicked() {
                        *ephemeride = id;
                    }
                } else {
                    ui.add(select_par.enabled(false));
                }
                ui.end_row();

                // ----------------------------------------------------------------
                ui.label("Template");
                ui.add_enabled(
                    active,
                    egui::DragValue::new(template).range(0..=i32::MAX),
                );

                ui.add(
                    IconicButton::new(ICON_ARROW)
                        // .enabled(enabled)
                        .on_hover_text("Grab selected."),
                );
                ui.end_row();
            });
    }

    fn set_up_field(
        ui: &mut egui::Ui,
        raw: &RawMeta,
        par: Option<&ParMeta>,
        tmp: &TemplateMeta,
    ) {
        egui::Grid::new("args")
            .num_columns(3)
            .spacing([32.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.label("ID");
                ui.label("Path");
                ui.end_row();

                ui.label("Raw file");
                ui.label(raw.id.to_string());
                ui.label(&raw.file_path);
                ui.end_row();

                ui.label("Ephemeride");
                if let Some(eph) = par {
                    ui.label(eph.id.to_string());
                    ui.label(&eph.file_path);
                } else {
                    ui.label(RichText::new(MISSING_DATA).italics());
                    ui.label(RichText::new(MISSING_DATA).italics());
                }
                ui.end_row();

                ui.label("Template");
                ui.label(tmp.id.to_string());
                ui.label(&tmp.file_path);
                ui.end_row();
            });
    }

    fn relaxed_buttons(
        &mut self,
        archivist: &Syncher,
        ui: &mut egui::Ui,
        raw: String,
        ephemeride: i32,
        template: i32,
    ) {
        let mut new_state = false;
        ui.horizontal(|ui| {
            let clear =
                ui.add(IconicButton::new(ICON_CLEAR).on_hover_text("Reset."));

            let write = ui.add(
                IconicButton::new(ICON_WRITE)
                    .enabled(!raw.is_empty() && template > 0)
                    .on_hover_text("Load files and proceed to the next step."),
            );

            ui.add(IconicButton::new(ICON_RUN).enabled(false).on_hover_text(
                "Run the pipeline.\nFirst you must load files.",
            ));

            if clear.clicked() {
                self.state = PipeStage::default();
                new_state = true;
            }
            if write.clicked() {
                archivist.request(Request::SetupPipes {
                    raw: raw.to_string(),
                    ephemeride: ephemeride.to_string(),
                    template: template.to_string(),
                });

                self.state = PipeStage::SettingUp {
                    raw: raw.to_string(),
                    ephemeride,
                    template,
                };
                new_state = true;
            }
        });

        if !new_state {
            self.state = PipeStage::Relaxed {
                raw,
                ephemeride,
                template,
            };
        }
    }

    fn setting_up_buttons(ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                IconicButton::new(ICON_CLEAR)
                    .enabled(false)
                    .on_hover_text("Reset."),
            );

            ui.add_sized(
                [IconicButton::WIDTHS[1], IconicButton::HEIGHTS[1]],
                egui::Spinner::new(),
            )
            .on_hover_text("Files are currently being loaded.");

            ui.add(IconicButton::new(ICON_RUN).enabled(false).on_hover_text(
                "Run the pipeline.\nFirst you must load files.",
            ));
        });
    }

    fn running_buttons(
        &mut self,
        archivist: &Syncher,
        ui: &mut egui::Ui,
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
    ) {
        ui.horizontal(|ui| {
            let clear =
                ui.add(IconicButton::new(ICON_CLEAR).on_hover_text("Reset"));

            ui.add(
                IconicButton::new(ICON_WRITE)
                    .enabled(false)
                    .on_hover_text("This step is already finished."),
            );

            let run = ui.add(
                IconicButton::new(ICON_RUN).on_hover_text("Run the pipeline."),
            );

            if clear.clicked() {
                self.state = PipeStage::default();
            } else if run.clicked() {
                archivist.run_pipeline(raw, ephemeride, template);
            } else {
                self.state = PipeStage::SetUp {
                    raw,
                    ephemeride,
                    template,
                };
            }
        });
    }

    fn running(&mut self, ui: &mut egui::Ui, info: &RunInfo) {
        ui.label(RichText::new("Running pipeline...").strong());
        let msg_index = match &info.status {
            Status::Idle | Status::Error(_) | Status::Starting { .. } => 0,
            Status::Copying(_, _) => 1,
            Status::InstallingEphemeride => 2,
            Status::Manipulating => 3,
            Status::VerifyingTemplate => 4,
            Status::GeneratingTOAs | Status::GotTOAs(_) => 5,
            Status::LoggingProcess => 6,
            Status::ParsingTOAs | Status::ArchivedTOAs(_) => 7,
            Status::Diagnosing(_)
            | Status::FinishedDiagnostic { .. }
            | Status::ArchivedTOAPlots(_) => 8,
            Status::Finished(_) => 9,
        };

        for (i, msg) in MESSAGES.iter().take(msg_index).enumerate() {
            ui.horizontal(|ui| {
                ui.label(*msg);
                match i {
                    5 => {
                        if let Some(n) = info.generated_toas {
                            ui.label(format!("➡ Got {n} TOA(s)!"));
                        }
                    }
                    7 => {
                        if let Some(n) = info.archived_toas {
                            ui.label(format!("➡ Archived {n} TOA(s)!"));
                        }
                    }
                    8 => {
                        ui.label(format!(
                            "{} / {}",
                            info.diagnosed.0,
                            info.diagnosed.1.len(),
                        ));
                    }
                    _ => {
                        ui.label(ICON_CHECK);
                    }
                }
            });

            if i == 8 {
                for (diag, ok) in &info.diagnosed.1 {
                    ui.label(format!(
                        "\t{} {}",
                        diag,
                        if *ok { ICON_CHECK } else { ICON_CROSS },
                    ));
                }
                if let Some(n) = info.archived_plots {
                    if n > 0 {
                        ui.label(format!("\tArchived {n} psrchive plots!"));
                    } else {
                        ui.label("\tFailed to archive psrchive plots!");
                    }
                }
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(MESSAGES[msg_index]).strong());

            if info.errored {
                ui.label(ICON_CROSS);
            } else if let Some(duration) = &info.done {
                ui.label(format!(
                    "Time elapsed {}",
                    display_elapsed_time(*duration),
                ));
            } else {
                ui.spinner();
            }
        });

        if msg_index == 9 || info.errored {
            let restart = ui.add(Button::new("Start over"));
            if restart.clicked() {
                log::info!("Redoing!");
                self.state = PipeStage::default();
            }
        }
    }

    pub(crate) fn interrupt(&mut self) {
        self.state = match replace(&mut self.state, PipeStage::Invalid) {
            PipeStage::Running(_) | PipeStage::Invalid => PipeStage::default(),
            s => s,
        }
    }

    pub(crate) fn set_status(&mut self, status: Status) {
        let mut info =
            match std::mem::replace(&mut self.state, PipeStage::Invalid) {
                PipeStage::Running(info) => info,
                _ => RunInfo::default(),
            };

        match &status {
            Status::Error(_) => info.errored = true,

            Status::GotTOAs(n) => info.generated_toas = Some(*n),
            Status::ArchivedTOAs(n) => info.archived_toas = Some(*n),
            Status::Diagnosing(n) => info.diagnosed = (*n, Vec::new()),
            Status::FinishedDiagnostic { diagnostic, passed } => {
                info.diagnosed.1.push((diagnostic.clone(), *passed));
            }
            Status::ArchivedTOAPlots(n) => info.archived_plots = *n,
            Status::Finished(duration) => info.done = Some(*duration),

            _ => {}
        }

        if let Status::Error(_) = status {
        } else {
            info.status = status;
        }

        self.state = PipeStage::Running(info);
    }

    pub(crate) fn reset(&mut self) {
        self.state = PipeStage::default();
    }
}
