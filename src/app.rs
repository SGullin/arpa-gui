use arpa::ARPAError;
use egui::{Align, FontId, Layout};
use log::debug;

use crate::ephemerides::EphemerideApp;
use crate::pulsars::PulsarsApp;
use crate::helpers::{confirm_button, icon, IconicButton, StatusMessage, StatusMessageSeverity};

mod syncher;
pub use syncher::{Syncher, Message, Request}; 

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
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Pulsars,
    Ephemerides,
    Templates,
    Observatories,
}
const TAB_FORMATS: &'static [(Tab, &'static str, &'static str)] = &[
    (Tab::Pulsars, "★", "Pulsars"),
    (Tab::Ephemerides, "⚙", "Ephemerides"),
    (Tab::Templates, "📄", "Templates"),
    (Tab::Observatories, "🔭", "Observatories"),
];

impl Application {
    pub fn new() -> Result<Self, ARPAError> {
        let archivist = Syncher::new()?;

        Ok(Self {
            archivist,

            tab: Tab::Pulsars,
            has_live_transaction: false,

            messages: Vec::new(),

            pulsars: PulsarsApp::new(),
            ephemerides: EphemerideApp::new(),
        })
    }

    pub fn init(self, cc: &eframe::CreationContext<'_>) -> Self {
        use egui::FontFamily as FF;
        use egui::TextStyle as TS;
        let text_styles: std::collections::BTreeMap<_, _> = [
            (TS::Heading,   FontId::new(30.0, FF::Proportional)),
            (TS::Small,     FontId::new(10.0, FF::Proportional)),
            (TS::Body,      FontId::new(18.0, FF::Proportional)),
            (TS::Monospace, FontId::new(18.0, FF::Proportional)),
            (TS::Button,    FontId::new(22.0, FF::Proportional)),
        ].into();

        // Mutate global styles with new text styles
        cc.egui_ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());

        self
    }

    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("side-bar")
        .show(ctx, |ui| ui.vertical_centered_justified(|ui| {
            ui.set_width(80.0);
            ui.add_space(24.0);

            TAB_FORMATS
            .iter()
            .for_each(|(t, i, h)| { 
                ui.selectable_value(
                    &mut self.tab, 
                    *t, 
                    icon(*i),
                ).on_hover_text(*h);
            });

            ui.with_layout(
                Layout::bottom_up(egui::Align::Center).with_cross_justify(true), 
                |ui| {
                    ui.add_space(24.0);
                    self.sql_buttons(ui);
                }
            );
        }));
    }

    fn sql_buttons(&self, ui: &mut egui::Ui) {
        // Rollback button
        let rollback_button = ui.add(
            IconicButton::new("⮪")
            .enabled(self.has_live_transaction)
            .large()
            .on_hover_text("Roll back current transaction.")
            .on_disabled_hover_text("There is no transaction to roll back.")
        );

        // Save button
        let save = ui.add(
            IconicButton::new("💾")
            .enabled(self.has_live_transaction)
            .on_hover_text("Commit current transaction.")
            .on_disabled_hover_text("There is no transaction to commit.")
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
        .show(ctx, |ui| ui.with_layout(
            Layout::right_to_left(Align::Center), 
            |ui| {
                let btn = ui.add(
                    IconicButton::new("❌")
                    .small()
                    .enabled(!self.messages.is_empty())
                    .on_hover_text("Clear all messages")
                );

                if btn.clicked() {
                    self.messages.clear();
                }
                
                ui.separator();

                ui.with_layout(Layout::top_down_justified(Align::Min), |ui| { 
                    egui::ScrollArea::vertical().show(ui, 
                        |ui| for m in &self.messages { ui.add(m.widget()); }
                    );
                });
            }
        )); 
    }
    
    fn handle_message(&mut self, message: Message) {
        match message {
            Message::Error(err) => self.handle_error(err),
            Message::Connected => self.inform("Connected!"),
            Message::CommitSuccess => {
                        self.inform("Commit successful! (list not updated)");

                        self.has_live_transaction = false;
                    },
            Message::RollbackSuccess => {
                        self.inform("Rollback successful!");
                        self.has_live_transaction = false;
                    },
            Message::Pulsars(pulsars) => self.pulsars.set_pulsars(pulsars),
            Message::SinglePulsar(pulsar) => self.pulsars.add_pulsar(pulsar),
            Message::PulsarAdded(id) => {
                        self.inform(format!("Successfully added pulsar #{:x}", id));
                        self.pulsars.deselect();
                        self.has_live_transaction = true;
                    },
            Message::PulsarDeleted(id) => {
                        self.inform(format!("Successfully deleted pulsar #{:x}", id));
                        self.pulsars.deselect();
                        self.has_live_transaction = true;
                    },
            Message::PulsarUpdated(id) => {
                        self.inform(format!("Successfully updated pulsar #{:x}", id));
                        self.has_live_transaction = true;
                    },

            Message::Ephemerides(pars) => self.ephemerides.set_pars(pars),
            Message::SingleEphemeride(par) => self.ephemerides.add_par(par),
        }
    }

    fn inform(&mut self, message: impl ToString) {
        self.messages.push(StatusMessage {
            severity: StatusMessageSeverity::Info,
            message: message.to_string(),
        })
    }

    fn handle_error(&mut self, error: ARPAError) {
        self.messages.push(StatusMessage {
            severity: StatusMessageSeverity::Error,
            message: format!("Error: {}", error),
        });
        self.pulsars.reset_ui();
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // ---- Check inbox ---------------------------------------------------
        if let Some(message) = self.archivist.check_inbox() {
            debug!("Incoming message: {:?}", message);
            self.handle_message(message);
        }

        // ---- Display menubars and such -------------------------------------
        self.menu_bar(ctx);
        self.message_bar(ctx);

        // ---- Display current applet ----------------------------------------
        match self.tab {
            Tab::Pulsars => self.pulsars.show(ctx, &self.archivist),
            Tab::Ephemerides => self.ephemerides.show(ctx, &self.archivist),

            _ => {
                egui::CentralPanel::default().show(
                    ctx, 
                    |ui| ui.label("Nothing here yet!")
                );
            },
        }

        // Collect any and all messasges 
        self.messages.append(self.pulsars.messages());
    }
}
