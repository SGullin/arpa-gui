use std::mem::replace;

use arpa::data_types::{ParMeta, RawMeta, TemplateMeta};
use egui::{Context, RichText};

use crate::app::{helpers::{IconicButton, StatusMessage, ICON_CLEAR, ICON_RUN, ICON_WRITE, MISSING_DATA}, Request, Syncher};

#[derive(Debug)]
enum PipeStage {
    Invalid, 
    Relaxed {
        raw: String,
        ephemeride: String,
        template: String,
    },
    SettingUp {
        raw: String,
        ephemeride: String,
        template: String,
    },
    SetUp {
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
    },
    Running,
}

impl Default for PipeStage {
    fn default() -> Self {
        Self::Relaxed {
            raw: String::new(), 
            ephemeride: String::new(), 
            template: String::new(), 
        }
    }
}

pub struct PipelineApp {
    state: PipeStage,
    messages: Vec<StatusMessage>,
}

impl PipelineApp {
    pub(crate) fn new() -> Self {
        Self {
            state: PipeStage::Relaxed { 
                raw: String::new(), 
                ephemeride: String::new(), 
                template: String::new(),
            },
            messages: Vec::new(),
        }
    }
    
    pub(crate) fn show(&mut self, ctx: &Context, archivist: &Syncher) {
        let state = replace(&mut self.state, PipeStage::Invalid);

        egui::CentralPanel::default().show(ctx, |ui| match state {
            PipeStage::Invalid => self.state = PipeStage::default(), 
            
            PipeStage::Relaxed { mut raw, mut ephemeride, mut template } => {
                Self::argument_entry(
                    ui, 
                    &mut raw, 
                    &mut ephemeride, 
                    &mut template, 
                    true,
                );
                self.relaxed_buttons(archivist, ui, raw, ephemeride, template);
            },

            PipeStage::SettingUp { mut raw, mut ephemeride, mut template } => {
                Self::argument_entry(
                    ui, 
                    &mut raw, 
                    &mut ephemeride, 
                    &mut template, 
                    false,
                );
                Self::setting_up_buttons(ui);
                self.state = PipeStage::SettingUp { raw, ephemeride, template };
            },

            PipeStage::SetUp { raw, ephemeride, template  } => {
                Self::set_up_field(ui, &raw, ephemeride.as_ref(), &template);
                self.running_buttons(archivist, ui, raw, ephemeride, template);
            },

            PipeStage::Running => {
                Self::running(ui);
                self.state = PipeStage::Running;
            },
        });
    }

    pub(crate) fn set_up(
        &mut self, 
        raw: RawMeta, 
        ephemeride: Option<ParMeta>, 
        template: TemplateMeta
    ) {
        self.state = PipeStage::SetUp {
            raw,
            ephemeride,
            template,
        }
    }
    
    fn argument_entry(
        ui: &mut egui::Ui, 
        raw: &mut String, 
        ephemeride: &mut String, 
        template: &mut String,
        active: bool,
    ) {
        egui::Grid::new("args")
        .num_columns(2)
        .spacing([32.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Raw file path or id");
            if active {
                ui.text_edit_singleline(raw);
            } 
            else {
                ui.label(raw.as_str());
            }
            ui.end_row();

            ui.label("Ephemeride path or id");
            if active {
                ui.text_edit_singleline(ephemeride);
            } 
            else {
                ui.label(ephemeride.as_str());
            }
            ui.end_row();

            ui.label("Template path or id");
            if active {
                ui.text_edit_singleline(template);
            } 
            else {
                ui.label(template.as_str());
            }
            ui.end_row();
        });
    }
    
    fn set_up_field( 
        ui: &mut egui::Ui,
        raw: &RawMeta, 
        par: Option<&ParMeta>, 
        tmp: &TemplateMeta
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
            }
            else {
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
        ephemeride: String, 
        template: String,
    ) {
        let mut new_state = false;
        ui.horizontal(|ui| {
            let clear = ui.add(IconicButton::new(ICON_CLEAR)
                .on_hover_text("Reset.")
            );

            let write = ui.add(IconicButton::new(ICON_WRITE)
                .enabled(!raw.is_empty() && !template.is_empty())
                .on_hover_text("Load files and proceed to the next step.")
            );

            ui.add(IconicButton::new(ICON_RUN)
                .enabled(false)
                .on_hover_text("Run the pipeline.\nFirst you must load files.")
            );

            if clear.clicked() {
                self.state = PipeStage::Relaxed { 
                    raw: String::new(),
                    ephemeride: String::new(),
                    template: String::new(),
                };
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
                    ephemeride: ephemeride.to_string(),
                    template: template.to_string(),
                };
                new_state = true;
            }
        });

        if !new_state {
            self.state = PipeStage::Relaxed { raw, ephemeride, template };
        }
    }
    
    fn setting_up_buttons(ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(IconicButton::new(ICON_CLEAR)
                .enabled(false)
                .on_hover_text("Reset.")
            );

            ui.add_sized(
                [IconicButton::WIDTHS[1], IconicButton::HEIGHTS[1]],
                egui::Spinner::new(),
            )
            .on_hover_text("Files are currently being loaded.");

            ui.add(IconicButton::new(ICON_RUN)
                .enabled(false)
                .on_hover_text("Run the pipeline.\nFirst you must load files.")
            );
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
            let clear = ui.add(IconicButton::new(ICON_CLEAR)
                .on_hover_text("Reset")
            );

            ui.add(IconicButton::new(ICON_WRITE)
                .enabled(false)
                .on_hover_text("This step is already finished.")
            );

            let run = ui.add(IconicButton::new(ICON_RUN)
                .on_hover_text("Run the pipeline.")
            );

            if clear.clicked() {
                self.state = PipeStage::Relaxed { 
                    raw: String::new(),
                    ephemeride: String::new(),
                    template: String::new(),
                };
            }
            else if run.clicked() {
                archivist.request(Request::RunPipeline {
                    raw,
                    ephemeride,
                    template,
                });

                self.state = PipeStage::Running;
            }
            else {
                self.state = PipeStage::SetUp { raw, ephemeride, template };
            }
        });

    }
    
    fn running(ui: &mut egui::Ui) {
        ui.label("Running pipeline...");
        ui.spinner();
    }
    
    pub(crate) fn finished(&mut self) {
        self.messages.push(StatusMessage {
            severity: crate::app::helpers::StatusMessageSeverity::Info,
            message: "Pipeline finished!".into(),
        });
        self.state = PipeStage::default();
    }

    pub(crate) fn interrupt(&mut self) {
        self.state = match replace(&mut self.state, PipeStage::Invalid) {
            PipeStage::Running |
            PipeStage::Invalid => PipeStage::default(),
            s => s,
        }
    }
}
