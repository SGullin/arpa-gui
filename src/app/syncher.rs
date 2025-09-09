use arpa::{ARPAError, Archivist};
use log::{debug, error};
use tokio::task::JoinHandle;

mod request;
pub(crate) use request::{Message, Request, DataType};

#[derive(Debug)]
/// Keeps a tokio runtime with a loop running async commands.
pub(crate) struct Syncher {
    _runtime: tokio::runtime::Runtime,
    _handle: JoinHandle<()>,
    requester: tokio::sync::mpsc::UnboundedSender<Request>,
    messager: std::sync::mpsc::Receiver<Message>,
}

impl Syncher {
    pub(crate) fn new() -> Result<Self, ARPAError> {
        let runtime = tokio::runtime::Runtime::new()?;
        let (txr, rxr) = tokio::sync::mpsc::unbounded_channel();
        let (txm, rxm) = std::sync::mpsc::channel();

        let handle = runtime.spawn(core(txm, rxr));

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
            messager: rxm,
        };

        Ok(s)
    }

    /// Checks for pending messages, will not block.
    pub fn check_inbox(&self) -> Option<Message> {
        self.messager.try_recv().ok()
    }

    /// Send a request to the async loop.
    pub fn request(&self, request: Request) {
        if let Err(err) = self.requester.send(request) {
            error!("Could not send {:?}", err.0);
        }
    }
}

async fn core(
    sender: std::sync::mpsc::Sender<Message>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Request>,
) {
    async fn send(
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

    let mut archvist = match Archivist::new(
        "../test-data/config.toml",
        "../arpa/sql",
    ).await {
        Ok(a) => a,
        Err(err) => {
            send(Message::Error(err), &sender).await;
            return;
        }
    };

    // Tell user we're in
    if !send(Message::Connected, &sender).await {
        return;
    };

    loop {
        let Some(request) = receiver.recv().await else {
            debug!("Connection closed!");
            return;
        };

        let response = request.handle(&mut archvist).await;

        match response {
            Message::Error(err) => 
                if !send(Message::Error(err), &sender).await { return; },

            msg => 
                if !send(msg, &sender).await { return; },
        }
    }
}
