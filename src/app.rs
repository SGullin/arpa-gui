use arpa::ARPAError;
use egui::{Align, FontId, Layout};
use log::{debug, info, warn, error};

pub(crate) mod helpers;
pub(crate) mod ephemerides;
pub(crate) mod pulsars;
mod pipeline;

use ephemerides::EphemerideApp;
use helpers::{
    confirm_button, icon, IconicButton, StatusMessage, StatusMessageSeverity, ICON_CROSS, ICON_REVERT, ICON_SAVE
};
use pulsars::PulsarsApp;

mod syncher;
pub(crate) use syncher::{Message, Request, Syncher, DataType};

use crate::app::pipeline::PipelineApp;

pub struct Application {
    archivist: Syncher,

    /// State
    tab: Tab,
    has_live_transaction: bool,

    /// Message queue
    messages: Vec<StatusMessage>,

    /// Applets
    pulsars: PulsarsApp,
    ephemerides: EphemerideApp,
    pipeline: PipelineApp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Pulsars,
    Ephemerides,
    Templates,
    Observatories,
    Pipeline,
}
const TAB_FORMATS: &[(Tab, &str, &str)] = &[
    (Tab::Pulsars, "★", "Pulsars"),
    (Tab::Ephemerides, "⚙", "Ephemerides"),
    (Tab::Templates, "📄", "Templates"),
    (Tab::Observatories, "📡", "Observatories"),
    (Tab::Pipeline, "🔩", "Pipeline"),
];

impl Application {
    pub(crate) fn new() -> Result<Self, ARPAError> {
        let archivist = Syncher::new()?;

        Ok(Self {
            archivist,

            tab: Tab::Pulsars,
            has_live_transaction: false,

            messages: Vec::new(),

            pulsars: PulsarsApp::new(),
            ephemerides: EphemerideApp::new(),
            pipeline: PipelineApp::new(),
        })
    }

    pub(crate) fn init(self, cc: &eframe::CreationContext<'_>) -> Self {
        use egui::FontFamily as FF;
        use egui::TextStyle as TS;
        let text_styles: std::collections::BTreeMap<_, _> = [
            (TS::Heading, FontId::new(30.0, FF::Proportional)),
            (TS::Small, FontId::new(10.0, FF::Proportional)),
            (TS::Body, FontId::new(18.0, FF::Proportional)),
            (TS::Monospace, FontId::new(18.0, FF::Proportional)),
            (TS::Button, FontId::new(22.0, FF::Proportional)),
        ]
        .into();

        // Mutate global styles with new text styles
        cc.egui_ctx.all_styles_mut(move |style| {
            style.text_styles = text_styles.clone();
        });

        self
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side-bar").show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.set_width(80.0);
                ui.add_space(24.0);

                for (t, i, h) in TAB_FORMATS {
                    ui.selectable_value(&mut self.tab, *t, icon(i))
                        .on_hover_text(*h);
                };

                ui.with_layout(
                    Layout::bottom_up(egui::Align::Center)
                        .with_cross_justify(true),
                    |ui| {
                        ui.add_space(24.0);
                        self.sql_buttons(ui);
                    },
                );
            })
        });
    }

    fn sql_buttons(&self, ui: &mut egui::Ui) {
        // Rollback button
        let rollback_button = ui.add(
            IconicButton::new(ICON_REVERT)
                .enabled(self.has_live_transaction)
                .large()
                .on_hover_text("Roll back current transaction.")
                .on_disabled_hover_text(
                    "There is no transaction to roll back.",
                ),
        );

        // Save button
        let save = ui.add(
            IconicButton::new(ICON_SAVE)
                .enabled(self.has_live_transaction)
                .large()
                .on_hover_text("Commit current transaction.")
                .on_disabled_hover_text("There is no transaction to commit."),
        );

        if save.clicked() {
            self.archivist.request(Request::Commit);
        }
        if confirm_button(rollback_button, "Roll back?") {
            self.archivist.request(Request::Rollback);
        }
    }

    fn message_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("messages")
            .resizable(true)
            .show(ctx, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let btn = ui.add(
                        IconicButton::new(ICON_CROSS)
                            .small()
                            .enabled(!self.messages.is_empty())
                            .on_hover_text("Clear all messages"),
                    );

                    if btn.clicked() {
                        self.messages.clear();
                    }

                    ui.separator();

                    ui.with_layout(
                        Layout::top_down_justified(Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for m in &self.messages {
                                    ui.add(m.widget());
                                }
                            });
                        },
                    );
                })
            });
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Error(err) => {
                        self.error(&err);
                        self.pulsars.reset_ui();
                        self.ephemerides.reset_ui();
                    },
            Message::Connected => self.info(&"Connected!"),
            Message::CommitSuccess => {
                        self.info(&"Commit successful! (list not updated)");

                        self.has_live_transaction = false;
                    },
            Message::RollbackSuccess => {
                        self.info(&"Rollback successful!");
                        self.has_live_transaction = false;
                    },
            Message::ItemAdded(dt, id) => {
                        self.info(&format!("Successfully added {dt} #{id}"));
                        self.reset_part(dt);
                        self.has_live_transaction = true;
                    },
            Message::ItemDeleted(dt, id) => {
                        self.info(&format!("Successfully deleted {dt} #{id}"));
                        self.reset_part(dt);
                        self.has_live_transaction = true;
                    },
            Message::ItemUpdated(dt, id) => {
                        self.info(&format!("Successfully updated {dt} #{id}"));
                        self.has_live_transaction = true;
                    },
            Message::Pulsars(pulsars) => {
                        if pulsars.is_empty() {
                            self.warn(&"No pulsars to download!");
                        }
                        self.pulsars.set_pulsars(pulsars)
                    },
            Message::SinglePulsar(pulsar) => self.pulsars.add_pulsar(pulsar),
            Message::Ephemerides(pars) => {
                        if pars.is_empty() {
                            self.warn(&"No ephemerides to download!");
                        }
                        self.ephemerides.set_pars(pars);
                    },
            Message::SingleEphemeride(par) => self.ephemerides.add_par(par),
            Message::PipesSetUp(raw_meta, par_meta, template_meta) => 
                self.pipeline.set_up(raw_meta, par_meta, template_meta),
            Message::PipelineFinished => self.pipeline.finished(),
        }
    }

    fn info(&mut self, message: &impl ToString) {
        info!("{}", message.to_string());
        self.messages.push(StatusMessage {
            severity: StatusMessageSeverity::Info,
            message: message.to_string(),
        });
    }

    fn warn(&mut self, message: &impl ToString) {
        warn!("{}", message.to_string());
        self.messages.push(StatusMessage {
            severity: StatusMessageSeverity::Warning,
            message: message.to_string(),
        });
    }

    fn error(&mut self, error: &ARPAError) {
        error!("{}", error.to_string());
        self.messages.push(StatusMessage {
            severity: StatusMessageSeverity::Error,
            message: format!("Error: {error}"),
        });
        self.pipeline.interrupt();
    }
    
    fn reset_part(&mut self, dt: DataType) {
        match dt {
            DataType::Pulsar => self.pulsars.deselect(),
            DataType::Ephemeride => self.ephemerides.deselect(),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // ---- Check inbox ---------------------------------------------------
        if let Some(message) = self.archivist.check_inbox() {
            debug!("Incoming message: {message:?}");
            self.handle_message(message);
        }

        // ---- Display menubars and such -------------------------------------
        self.menu_bar(ctx);
        self.message_bar(ctx);

        // ---- Display current applet ----------------------------------------
        match self.tab {
            Tab::Pulsars => self.pulsars.show(ctx, &self.archivist),
            Tab::Ephemerides => {
                self.ephemerides.show(ctx, &self.archivist);
                if let Some(id) = self.ephemerides.select_pulsar() {
                    self.tab = Tab::Pulsars;
                    self.pulsars.select_with_id(id);
                }
            },
            Tab::Pipeline => self.pipeline.show(ctx, &self.archivist),

            _ => {
                egui::CentralPanel::default()
                    .show(ctx, |ui| ui.label("Nothing here yet!"));
            }
        }

        // Collect any and all messasges
        self.messages.append(self.pulsars.messages());
    }
}
