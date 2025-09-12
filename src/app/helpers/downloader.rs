use egui::RichText;
use egui_extras::{Column, TableBuilder};
use rayon::slice::ParallelSliceMut;

use super::ICON_SYNC;

use super::{IconicButton, ra_delete};

pub trait Item: Send {
    const NAME: &str;
    const COLUMNS: &[(&str, &str)];

    fn id(&self) -> i32;
    fn format(&self, row: &mut egui_extras::TableRow);
    fn cmp_by(&self, other: &Self, index: usize) -> std::cmp::Ordering;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FetchType {
    All,
    Id(i32),
    // Range(i32, i32),
}

#[derive(Clone, Copy)]
pub enum DownloaderAction {
    None,
    Delete(Option<i32>),
    Download(FetchType),
}

pub struct Downloader<T> {
    data: Vec<T>,

    selected: Option<usize>,
    sort_by: usize,

    fetch_type: FetchType,
    fetching: bool,
    action: DownloaderAction,
}

impl<T> Downloader<T>
where
    T: Item,
{
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),

            selected: None,
            sort_by: 0,

            fetch_type: FetchType::All,
            fetching: false,
            action: DownloaderAction::None,
        }
    }

    pub fn action_bar(&mut self, ctx: &egui::Context) {
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

            let download = if self.fetching {
                ui.add_sized(
                    [IconicButton::WIDTHS[1], IconicButton::HEIGHTS[1]],
                    egui::Spinner::new(),
                )
                .on_hover_text("Synching...")
            } else {
                ui.add(
                    IconicButton::new(ICON_SYNC)
                        .enabled(!self.fetching)
                        .on_hover_text("Download pulsars"),
                )
            };

            ui.radio_value(&mut self.fetch_type, FetchType::All, "All");

            let (mut id, enabled) = match self.fetch_type {
                FetchType::Id(id) => (id, true),
                FetchType::All => (0, false),
            };
            ui.radio_value(&mut self.fetch_type, FetchType::Id(id), "With ID");
            ui.add_enabled(
                enabled,
                egui::DragValue::new(&mut id).range(1..=0x7FFF_FFFE),
            );
            if let FetchType::Id(i) = &mut self.fetch_type {
                *i = id;
            }

            if download.clicked() {
                self.fetching = true;
                self.action = DownloaderAction::Download(self.fetch_type);
            }
        });
    }

    pub fn table(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        if self.data.is_empty() {
            ui.label(format!(
                "No {}s in memory!\n (Sync button below)",
                T::NAME,
            ));
            return None;
        }

        let height = ui.available_height();
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .columns(Column::remainder(), T::COLUMNS.len() - 2)
            .column(Column::auto())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .sense(egui::Sense::click());

        let mut selected = None;

        table
            .header(24.0, |mut header| {
                T::COLUMNS.iter().enumerate().for_each(|(i, (col, hint))| {
                    header.col(|ui| {
                        let sort = format_header(ui, col, hint);

                        if sort {
                            self.sort_by = i;
                            self.data
                                .par_sort_by(|a, b| a.cmp_by(b, self.sort_by));
                        }
                    });
                });
            })
            .body(|mut body| {
                let mut clicked = None;
                for (index, item) in self.data.iter().enumerate() {
                    body.row(18.0, |mut row| {
                        row.set_selected(self.selected() == Some(index));

                        item.format(&mut row);
                        // format_pulsar_meta(item, &mut row);

                        if row.response().clicked() {
                            clicked = Some(index);
                        }
                    });
                }

                selected = clicked.and_then(|i| self.select(i));
            });

        selected
    }

    pub fn add(&mut self, item: T) {
        let pos = self.data.iter().position(|i| i.id() == item.id());

        match pos {
            Some(i) => self.data[i] = item,
            None => self.data.push(item),
        }

        self.select(pos.unwrap_or(self.data.len() - 1));
        self.fetching = false;
    }

    pub fn set(&mut self, items: Vec<T>) {
        self.data = items;
        self.fetching = false;
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

    pub const fn selected(&self) -> Option<usize> {
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

    pub fn stop_fetching(&mut self) {
        self.fetching = false;
    }
}

fn format_header(ui: &mut egui::Ui, text: &str, hint: &str) -> bool {
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
