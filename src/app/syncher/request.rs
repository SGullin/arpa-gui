use arpa::{data_types::{par_meta::ParMeta, pulsar_meta::PulsarMeta}, ARPAError, Archivist};

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
    pub async fn handle(
        self,
        archvist: &mut Archivist
    ) -> Message {
        use Message as M;
        use Request as R;
        let response = match self {
            R::Commit => archvist.commit_transaction().await
                .map(|_| M::CommitSuccess),
            R::Rollback => archvist.rollback_transaction().await
                .map(|_| M::RollbackSuccess),

            // ---- Pulsars ---------------------------------------------------
            R::DownloadAllPulsars => archvist.get_all().await
                .map(M::Pulsars),
            R::DownloadPulsarById(id) => archvist.get(id).await
                .map(M::SinglePulsar),
            R::AddPulsar(meta) => archvist.insert(meta).await
                .map(M::PulsarAdded),
            R::DeletePulsar(id) => archvist.delete::<PulsarMeta>(id).await
                .map(|_| M::PulsarDeleted(id)),
            R::UpdatePulsar(id, meta) => 
                archvist.update_from_cache::<PulsarMeta>(&meta, id).await
                .map(|_| M::PulsarUpdated(id)),

            // ---- Ephemerides -----------------------------------------------
            R::DownloadAllEphemerides => archvist.get_all().await
                .map(M::Ephemerides),
            R::DownloadEphemerideById(id) => archvist.get(id).await
                .map(M::SingleEphemeride),
        };

        match response {
            Ok(m) => m,
            Err(e) => Message::Error(e.into()),
        }
    }
}
