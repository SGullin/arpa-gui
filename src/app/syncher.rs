use arpa::{
    ARPAError, Archivist,
    data_types::{ParMeta, RawMeta, TemplateMeta},
    pipeline::Status,
};
use log::{debug, error};
use tokio::task::JoinHandle;

mod request;
pub use request::{DataType, Message, Request};

#[derive(Debug)]
/// Keeps a tokio runtime with a loop running async commands.
pub struct Syncher {
    _runtime: tokio::runtime::Runtime,
    _handle: JoinHandle<()>,
    requester: tokio::sync::mpsc::UnboundedSender<Request>,
    message_receiver: std::sync::mpsc::Receiver<Message>,
    message_sender: std::sync::mpsc::Sender<Message>,
}

impl Syncher {
    pub(crate) fn new() -> Result<Self, ARPAError> {
        let runtime = tokio::runtime::Runtime::new()?;
        let (txr, rxr) = tokio::sync::mpsc::unbounded_channel();
        let (txm, rxm) = std::sync::mpsc::channel();

        let handle = runtime.spawn(core(txm.clone(), rxr));

        // Wait on connection confirmation
        loop {
            let message = match rxm.recv() {
                Ok(m) => m,
                Err(err) => todo!("{}", err),
            };

            match message {
                Message::Error(err) => return Err(err),
                Message::Connected => debug!("We're in!"),
                _ => continue,
            }

            break;
        }

        let s = Self {
            _runtime: runtime,
            _handle: handle,
            requester: txr,
            message_receiver: rxm,
            message_sender: txm,
        };

        Ok(s)
    }

    /// Checks for pending messages, will not block.
    pub fn check_inbox(&self) -> Option<Message> {
        self.message_receiver.try_recv().ok()
    }

    /// Send a request to the async loop.
    pub fn request(&self, request: Request) {
        if let Err(err) = self.requester.send(request) {
            error!("Could not send {:?}", err.0);
        }
    }

    pub(crate) fn run_pipeline(
        &self,
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
    ) {
        let sender = self.message_sender.clone();
        let callback = Box::new(move |s: Status| {
            let result = sender.send(Message::PipelineStatus(s));
            if let Err(err) = result {
                error!("Send error: {err}");
            }
        });

        self.request(Request::RunPipeline {
            raw,
            ephemeride,
            template,
            callback,
        });
    }
}

async fn core(
    sender: std::sync::mpsc::Sender<Message>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Request>,
) {
    fn send(
        message: Message,
        channel: &std::sync::mpsc::Sender<Message>,
    ) -> bool {
        let result = channel.send(message);
        if let Err(err) = result {
            error!("Send error: {err}");
            return false;
        }
        true
    }

    let mut archvist =
        match Archivist::new("../test-data/config.toml", "../arpa/sql").await {
            Ok(a) => a,
            Err(err) => {
                send(Message::Error(err), &sender);
                return;
            }
        };

    // Tell user we're in
    if !send(Message::Connected, &sender) {
        return;
    };

    loop {
        let Some(request) = receiver.recv().await else {
            debug!("Connection closed!");
            return;
        };

        let response = request.handle(&mut archvist).await;

        match response {
            Message::Error(err) => {
                if !send(Message::Error(err), &sender) {
                    return;
                }
            }

            msg => {
                if !send(msg, &sender) {
                    return;
                }
            }
        }
    }
}
