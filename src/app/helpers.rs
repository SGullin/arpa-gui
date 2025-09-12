use egui::{Align, Layout, RichText, Ui};
use std::fmt::Display;

pub mod downloader;
mod iconic_button;

pub use iconic_button::IconicButton;

pub const MISSING_DATA: &str = "N/A";
pub const ICON_CLEAR: &str = "🗋";
pub const ICON_CROSS: &str = "❌";
pub const ICON_INSERT: &str = "➕";
pub const ICON_CHECK : &str = "✔";
pub const ICON_DELETE: &str = "🗑";
pub const ICON_WRITE: &str = "📝";
pub const ICON_SAVE: &str = "💾";
pub const ICON_OPEN: &str = "🗁";
pub const ICON_ARROW: &str = "⤵";
pub const ICON_REVERT: &str = "⮪";
pub const ICON_SYNC: &str = "🔄";
pub const ICON_RUN: &str = "🚂";

pub struct StatusMessage {
    pub severity: StatusMessageSeverity,
    pub message: String,
}
pub enum StatusMessageSeverity {
    Info,
    Warning,
    Error,
}
impl StatusMessage {
    pub fn widget(&self) -> egui::Label {
        egui::Label::new(RichText::new(&self.message).color(
            match self.severity {
                StatusMessageSeverity::Info => egui::Color32::GREEN,
                StatusMessageSeverity::Warning => egui::Color32::ORANGE,
                StatusMessageSeverity::Error => egui::Color32::RED,
            },
        ))
    }

    pub fn wrong() -> Self {
        Self {
            severity: StatusMessageSeverity::Warning,
            message: "Something went wrong.".into(),
        }
    }
}

pub fn format_data_option<T>(data: &Option<T>) -> RichText
where
    T: Display,
{
    if let Some(value) = data.as_ref() {
        RichText::new(value.to_string()).strong()
    } else {
        RichText::new(MISSING_DATA)
    }
}

pub fn format_unique_data_option<T>(data: &Option<T>, other: &T) -> RichText
where
    T: PartialEq + ToString,
{
    match data {
        Some(d) if other == d => RichText::new(d.to_string()).italics(),
        Some(d) => RichText::new(d.to_string()).strong(),
        None => RichText::new(MISSING_DATA),
    }
}

pub fn table_header(ui: &mut Ui, text: &str, hint: &str) -> bool {
    ui.set_height(IconicButton::HEIGHTS[0]);

    ui.label(
        RichText::new(text)
            .strong()
            .text_style(egui::TextStyle::Button),
    )
    .on_hover_text(hint);

    ui.add(IconicButton::new("⏷").small().on_hover_text("Sort"))
        .clicked()
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
        }
        None => {
            if ui.button("Set").clicked() {
                *data = Some(String::new());
                return true;
            }
            false
        }
    }
}

/// Adds a pop-up to confirm button press.
pub fn confirm_button(button: egui::response::Response, caution: &str) -> bool {
    let mut confirmed = false;
    egui::Popup::menu(&button).show(|ui| {
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

/// For the main tabs.
pub fn icon(text: &str) -> RichText {
    RichText::new(text).size(52.0)
}

/// Adds a delete button aligned to the right.
pub fn ra_delete(ui: &mut Ui, enabled: bool) -> bool {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let delete = ui.add(
            IconicButton::new(ICON_DELETE)
                .enabled(enabled)
                .on_hover_text("Delete"),
        );

        confirm_button(delete, "Delete selected?")
    })
    .inner
}
