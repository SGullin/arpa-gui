use arpa::{
    ARPAError, Archivist,
    data_types::{ParMeta, PulsarMeta},
};
use log::info;

#[derive(Debug)]
pub enum Message {
    Error(ARPAError),
    /// Sent out when an `Archivist` has been successfully created.
    Connected,

    /// Response for attempting a commit.
    CommitSuccess,
    /// Response for attempting a rollback.
    RollbackSuccess,

    // ---- Pulsars -----------------------------------------------------------
    /// Downloaded pulsar info.
    Pulsars(Vec<PulsarMeta>),
    /// Downloaded pulsar info.
    SinglePulsar(PulsarMeta),
    /// Response for adding a pulsar.
    PulsarAdded(i32),
    /// Response for deleting a pulsar.
    PulsarDeleted(i32),
    /// Response for updating a pulsar.
    PulsarUpdated(i32),

    // ---- Ephemerides -------------------------------------------------------
    /// Downloaded par info.
    Ephemerides(Vec<ParMeta>),
    /// Downloaded par info.
    SingleEphemeride(ParMeta),
}

#[derive(Debug)]
pub enum Request {
    /// Commit a live transaction.
    Commit,
    /// Roll back a live transaction.
    Rollback,

    // ---- Pulsars -----------------------------------------------------------
    /// Download _all_ pulsars.
    DownloadAllPulsars,
    /// Download _one_ pulsar.
    DownloadPulsarById(i32),
    /// Add a new pulsar.
    AddPulsar(PulsarMeta),
    /// Delete an existing pulsar.
    DeletePulsar(i32),
    /// Overwrite an existing pulsar.
    UpdatePulsar(i32, PulsarMeta),

    // ---- Ephemerides -------------------------------------------------------
    /// Download _all_ ephemeride.
    DownloadAllEphemerides,
    /// Download _one_ ephemerides.
    DownloadEphemerideById(i32),
}

impl Request {
    pub async fn handle(self, archvist: &mut Archivist) -> Message {
        info!("Handling {:?}", self);
        use Message as M;
        let response = match self {
            Self::Commit => archvist
                .commit_transaction().await
                .map(|()| M::CommitSuccess),
            Self::Rollback => archvist
                .rollback_transaction().await
                .map(|()| M::RollbackSuccess),

            // ---- Pulsars ---------------------------------------------------
            Self::DownloadAllPulsars => archvist.get_all().await
                .map(M::Pulsars),
            Self::DownloadPulsarById(id) => archvist.get(id).await
                .map(M::SinglePulsar),
            Self::AddPulsar(meta) => archvist.insert(meta).await
                .map(M::PulsarAdded),
            Self::DeletePulsar(id) => archvist
                .delete::<PulsarMeta>(id).await
                .map(|()| M::PulsarDeleted(id)),
            Self::UpdatePulsar(id, meta) => archvist
                .update_from_cache::<PulsarMeta>(&meta, id).await
                .map(|()| M::PulsarUpdated(id)),

            // ---- Ephemerides -----------------------------------------------
            Self::DownloadAllEphemerides => archvist.get_all().await
                .map(M::Ephemerides),
            Self::DownloadEphemerideById(id) => archvist.get(id).await
                .map(M::SingleEphemeride),
        };

        match response {
            Ok(m) => m,
            Err(e) => Message::Error(e.into()),
        }
    }
}
