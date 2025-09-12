use arpa::data_types::TOAInfo;

use crate::app::{helpers::{downloader::{Downloader, DownloaderAction, FetchType}, StatusMessage, StatusMessageSeverity}, DataType, Request, Syncher};

pub(crate) struct TOAsApp {
    pub downloader: Downloader<TOAInfo>,
    messages: Vec<StatusMessage>,
}
impl TOAsApp {
    pub fn new() -> Self {
        Self {
            downloader: Downloader::new(),
            messages: Vec::new(),
        }
    }
    
    pub(crate) fn show(&mut self, ctx: &egui::Context, archivist: &Syncher) {
        self.downloader.action_bar(ctx);
        
        match self.downloader.action() {
            DownloaderAction::None => {}
            DownloaderAction::Delete(index) => match index {
                Some(id) => archivist.request(Request::DeleteItem(DataType::TOA, id)),
                None => self.messages.push(StatusMessage {
                    severity: StatusMessageSeverity::Warning,
                    message: "Something went wrong...".into(),
                }),
            },

            DownloaderAction::Download(ft) => 
                archivist.request(Request::DownloadTOAs(ft)),
        }

        let response = egui::CentralPanel::default()
            .show(ctx, |ui| {ui.scope_builder(
                egui::UiBuilder::new().sense(egui::Sense::click()),
                |ui| egui::Frame::default()
                    .show(ui, |ui| self.body(ui, archivist))
                ).response
            })
            .inner;

        if response.clicked() {
            self.downloader.deselect();
        }
    }

    pub fn deselect(&mut self) {
        self.downloader.deselect();
    }
    
    fn body(&self, ui: &mut egui::Ui, archivist: &Syncher) {
        
    }
}
