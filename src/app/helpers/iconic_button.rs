use egui::{RichText, Ui};

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
        let call = |ui: &mut Ui| {
            ui.add_sized(
                [Self::WIDTHS[self.size], Self::HEIGHTS[self.size]],
                egui::Button::new(
                    RichText::new(self.text).size(Self::SIZES[self.size]),
                ),
            )
        };

        let mut response = match self.enabled {
            Some(enabled) => ui.add_enabled_ui(enabled, call).inner,
            None => call(ui),
        };

        if let Some(hint) = self.hint {
            response =
                response.on_hover_text(&hint).on_disabled_hover_text(hint)
        }

        if let Some(hint) = self.disabled_hint {
            response = response.on_disabled_hover_text(hint)
        }

        response
    }
}
