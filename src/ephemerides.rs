use arpa::data_types::par_meta::ParMeta;
use egui_extras::{Column, TableBuilder};

use crate::{app::{Request, Syncher}, helpers::{downloader::{Downloader, DownloaderAction, FetchType}, StatusMessage, StatusMessageSeverity}};

pub struct EphemerideApp {
    downloader: Downloader<ParMeta>,
    messages: Vec<StatusMessage>,
}

impl EphemerideApp {
    pub fn new() -> Self {
        Self { 
            downloader: Downloader::new(),
            messages: Vec::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, archivist: &Syncher) {
        self.downloader.show(ctx);

        match self.downloader.action() {
            DownloaderAction::None => {},
            DownloaderAction::Delete(index) => match index {
                Some(id) => archivist.request(Request::DeletePulsar(id)),
                None => self.messages.push(StatusMessage {
                    severity: StatusMessageSeverity::Error,
                    message: "Something went wrong...".into(),
                }),
            },

            DownloaderAction::Download(ft) => { 
                let request = match ft { 
                    FetchType::All => Request::DownloadAllEphemerides,
                    FetchType::Id(id) => Request::DownloadEphemerideById(id),
                };
                archivist.request(request);
            }
        }
    }
    
    fn par_table(&mut self, ui: &mut egui::Ui) {
        if self.downloader.data().is_empty() {
            ui.label("No ephemerides in memory!\n (Sync button below)");
            return;
        }

        let height = ui.available_height();
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            // .column(Column::auto())
            // .columns(Column::remainder(), 5)
            // .column(Column::auto())
            .min_scrolled_height(0.0)
            .max_scroll_height(height)
            .sense(egui::Sense::click());

        // table
        // .header(24.0, |mut header| (0..7).for_each(|i| { 
        //     header.col(|ui| {
        //         let sort = table_header(
        //             ui, 
        //             PULSAR_META_TABLE[i].0, 
        //             PULSAR_META_TABLE[i].1
        //         ); 

        //         if sort {
        //             self.sort_by = i;
        //             self.sort_table();
        //         }
        //     });
        // }))
        // .body(|mut body| {
        //     let mut clicked = None;
        //     for (index, item) in self.downloader.data().iter().enumerate() {
        //         body.row(18.0, |mut row| {
        //             row.set_selected(self.downloader.selected() == Some(index));
                    
        //             format_pulsar_meta(item, &mut row);
                    
        //             if row.response().clicked() {
        //                 clicked = Some(index);
        //             }
        //         });
        //     }

        //     if let Some(i) = clicked
        //     .map(|i| self.downloader.select(i))
        //     .flatten() {
        //         self.new_pulsar = self.downloader.data()[i].clone();
        //     }
        // });
    }
    
    pub fn set_pars(&mut self, pars: Vec<ParMeta>) {
        *self.downloader.data_mut() = pars;
        self.downloader.stop_fetching();
    }
    
    pub fn add_par(&mut self, par: ParMeta) {
        self.downloader.data_mut().push(par);
    }
}
