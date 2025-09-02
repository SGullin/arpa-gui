use std::fmt::Display;
use egui::{Align, Layout, RichText, Ui};

pub mod downloader;

pub const MISSING_DATA: &'static str = "N/A";

pub struct StatusMessage {
    pub severity: StatusMessageSeverity,
    pub message: String,
}
pub enum StatusMessageSeverity {
    Info,
    Error,
}
impl StatusMessage {
    pub fn widget(&self) -> egui::Label {
        egui::Label::new(
            RichText::new(&self.message)
            .color(match self.severity {
                StatusMessageSeverity::Info => egui::Color32::GREEN,
                StatusMessageSeverity::Error => egui::Color32::RED,
            })
        )
    }
}

pub fn format_data_option<T>(data: &Option<T>) -> RichText where T: Display {
    if let Some(value) = data.as_ref() {
        RichText::new(value.to_string()).strong()
    }
    else {
        RichText::new(MISSING_DATA)
    }
}

pub fn format_unique_data_option<T>(
    data: &Option<T>,
    other: &T,
) -> RichText where T:PartialEq + ToString {
    match data {
        Some(d) if other == d => 
            RichText::new(d.to_string()).italics(),
        Some(d) => 
            RichText::new(d.to_string()).strong(),
        None => 
            RichText::new(MISSING_DATA),
    }
}

pub fn table_header(ui: &mut Ui, text: &str, hint: &str) -> bool {
    ui.set_height(IconicButton::HEIGHTS[0]);

    ui.label(RichText::new(text)
    .strong()
    .text_style(egui::TextStyle::Button))
    .on_hover_text(hint);

    ui.add(IconicButton::new("⏷").small().on_hover_text("Sort")).clicked()
}

/// Returns whether the data changed or not.
pub fn enter_data_option(ui: &mut Ui, data: &mut Option<String>) -> bool {
    match data {
        Some(str) => { 
            let tes = ui.text_edit_singleline(str);
            let changed = tes.changed();

            tes.context_menu(|ui| {
                if ui.button("Delete").clicked() {
                    *data = None;
                    ui.close();
                }
            });

            changed
        },
        None => {
            if ui.button("Set").clicked() {
                *data = Some(String::new());
                return true;
            }
            false
        },
    }
}

pub fn confirm_button(
    button: egui::response::Response,
    caution: &str,
) -> bool {
    let mut confirmed = false;
    egui::Popup::menu(&button)
        .show(|ui| {
            ui.set_min_width(120.0);
            ui.label(caution);
            ui.separator();

            if ui.button("Yes").clicked() {
                confirmed = true;
                ui.close();
            }
            if ui.button("No").clicked() {
                ui.close();
            }
        });

    confirmed
}

pub fn icon(text: &str) -> RichText {
    RichText::new(text).size(64.0)
}

pub fn small_icon(text: impl Into<String>) -> RichText {
    RichText::new(text).size(28.0)
}

pub fn ra_delete(ui: &mut Ui, enabled: bool) -> bool {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let delete = ui.add(
            IconicButton::new("🗑")
            .enabled(enabled)
            .on_hover_text("Delete")
        );

        confirm_button(delete, "Delete selected?")
    }).inner
}

pub struct IconicButton {
    text: String,
    size: usize,
    hint: Option<String>,
    disabled_hint: Option<String>,
    enabled: Option<bool>,
}

impl IconicButton {
    pub const HEIGHTS: [f32; 3] = [20.0, 40.0, 72.0];
    pub const WIDTHS: [f32; 3] = [20.0, 60.0, 72.0];
    pub const SIZES: [f32; 3] = [22.0, 28.0, 64.0];

    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 1,
            hint: None,
            disabled_hint: None,
            enabled: None,
        }
    }

    pub fn on_hover_text(mut self, text: impl Into<String>) -> Self {
        self.hint = Some(text.into());
        self
    }

    pub fn on_disabled_hover_text(mut self, text: impl Into<String>) -> Self {
        self.disabled_hint = Some(text.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled.into());
        self
    }

    pub fn large(mut self) -> Self {
        self.size = 2;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = 0;
        self
    }
}

impl egui::Widget for IconicButton {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let call = |ui: &mut Ui| ui.add_sized(
            [Self::WIDTHS[self.size], Self::HEIGHTS[self.size]], 
            egui::Button::new(
                RichText::new(self.text)
                .size(Self::SIZES[self.size])
            )
        );

        let mut response = match self.enabled {
            Some(enabled) => ui.add_enabled_ui(enabled, call).inner,
            None => call(ui),
        };

        if let Some(hint) = self.hint {
            response = response
            .on_hover_text(&hint)
            .on_disabled_hover_text(hint)
        }
        
        if let Some(hint) = self.disabled_hint {
            response = response
            .on_disabled_hover_text(hint)
        }
        
        response
    }
}
