use arpa::TableItem;

use super::{IconicButton, ra_delete};

pub struct Downloader<T> {
    data: Vec<T>,
    selected: Option<usize>,
    fetch_type: FetchType,
    fetching: bool,
    action: DownloaderAction,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FetchType {
    All,
    Id(i32),
}

#[derive(Clone, Copy)]
pub enum DownloaderAction {
    None,
    Delete(Option<i32>),
    Download(FetchType),
}

impl<T> Downloader<T>
where
    T: TableItem,
{
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            selected: None,
            fetch_type: FetchType::All,
            fetching: false,
            action: DownloaderAction::None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("downloader").show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                self.download_menu(ui);

                let delete = ra_delete(ui, self.selected.is_some());
                if delete {
                    self.action = DownloaderAction::Delete(self.selected_id());
                }
            });
            ui.add_space(12.0);
        });
    }

    fn download_menu(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.set_height(IconicButton::HEIGHTS[1]);

            let download = match self.fetching {
                true => ui
                    .add_sized(
                        [IconicButton::WIDTHS[1], IconicButton::HEIGHTS[1]],
                        egui::Spinner::new(),
                    )
                    .on_hover_text("Synching..."),

                false => ui.add(
                    IconicButton::new("🔄")
                        .enabled(!self.fetching)
                        .on_hover_text("Download pulsars"),
                ),
            };

            ui.radio_value(&mut self.fetch_type, FetchType::All, "All");

            let (mut id, enabled) = match self.fetch_type {
                FetchType::Id(id) => (id, true),
                _ => (0, false),
            };
            ui.radio_value(&mut self.fetch_type, FetchType::Id(id), "With ID");
            ui.add_enabled(
                enabled,
                egui::DragValue::new(&mut id).range(1..=0x7FFF_FFFE),
            );
            match &mut self.fetch_type {
                FetchType::Id(i) => *i = id,
                _ => {}
            }

            if download.clicked() {
                self.fetching = true;
                self.action = DownloaderAction::Download(self.fetch_type);
            }
        });
    }

    pub fn add(&mut self, item: T) {
        let pos = self.data.iter().position(|i| i.id() == item.id());

        match pos {
            Some(i) => self.data[i] = item,
            None => self.data.push(item),
        }

        self.select(pos.unwrap_or(self.data.len() - 1));
    }

    pub fn action(&mut self) -> DownloaderAction {
        let a = self.action;
        self.action = DownloaderAction::None;
        a
    }

    pub fn select(&mut self, index: usize) -> Option<usize> {
        if index >= self.data.len() {
            self.deselect();
        }

        match self.selected {
            Some(i) if i == index => self.selected = None,
            _ => self.selected = Some(index),
        };

        self.selected
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_id(&self) -> Option<i32> {
        self.selected.map(|i| self.data[i].id())
    }

    pub fn deselect(&mut self) {
        self.selected = None;
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<T> {
        &mut self.data
    }

    pub fn stop_fetching(&mut self) {
        self.fetching = false;
    }
}
