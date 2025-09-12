use std::path::PathBuf;

use arpa::{
    ARPAError, Archivist, TableItem,
    data_types::{ParMeta, PulsarMeta, RawMeta, TOAInfo, TemplateMeta},
    pipeline,
};
use log::info;

use crate::app::{
    ephemerides::ParData, helpers::downloader::FetchType, toas::TOAData,
};

#[derive(Debug)]
pub enum DataType {
    Pulsar,
    Ephemeride,
    Toa,
}
impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pulsar => write!(f, "pulsar"),
            Self::Ephemeride => write!(f, "ephemeride"),
            Self::Toa => write!(f, "TOA"),
        }
    }
}

#[derive(Debug)]
pub enum Message {
    Error(ARPAError),
    /// Sent out when an `Archivist` has been successfully created.
    Connected,
    /// Response for attempting a commit.
    CommitSuccess,
    /// Response for attempting a rollback.
    RollbackSuccess,

    // ---- Generics ----------------------------------------------------------
    /// Response for adding something.
    ItemAdded(DataType, i32),
    /// Response for deleting something.
    ItemDeleted(DataType, i32),
    /// Response for updating something.
    ItemUpdated(DataType, i32),

    // ---- Pulsars -----------------------------------------------------------
    /// Downloaded pulsar info.
    Pulsars(Vec<PulsarMeta>),
    /// Downloaded pulsar info.
    SinglePulsar(PulsarMeta),

    // ---- Ephemerides -------------------------------------------------------
    /// Downloaded par info.
    Ephemerides(Vec<ParData>),
    /// Downloaded par info.
    SingleEphemeride(ParData),

    // ---- TOAs --------------------------------------------------------------
    /// Downloaded TOAs.
    TOAs(Vec<TOAData>),
    /// Downloaded TOA.
    SingleTOA(TOAData),

    // ---- Pipeline ----------------------------------------------------------
    /// Response if set up is ok.
    PipesSetUp(RawMeta, Option<ParMeta>, TemplateMeta),
    /// Response if pipeline cooked properly.
    PipelineFinished,
    /// Status message.
    PipelineStatus(pipeline::Status),
}

pub enum Request {
    /// Commit a live transaction.
    Commit,
    /// Roll back a live transaction.
    Rollback,

    // ---- Generics ----------------------------------------------------------
    /// Download some data.
    Download(DataType, FetchType),
    /// Delete something froma a table.
    DeleteItem(DataType, i32),

    // ---- Pulsars -----------------------------------------------------------
    AddPulsar(PulsarMeta),
    /// Overwrite an existing pulsar.
    UpdatePulsar(i32, PulsarMeta),

    // ---- Ephemerides -------------------------------------------------------
    /// Add one ephemeride
    AddPar {
        path: PathBuf,
        pulsar: String,
        master: bool,
    },
    /// Overwrite an existing ephemeride.
    UpdatePar {
        id: i32,
        path: PathBuf,
        pulsar: String,
        master: bool,
    },

    // ---- Pipeline ----------------------------------------------------------
    /// Load files to set up pipeline job.
    SetupPipes {
        raw: String,
        ephemeride: String,
        template: String,
    },
    /// Run the pipeline with the selected files.
    RunPipeline {
        raw: RawMeta,
        ephemeride: Option<ParMeta>,
        template: TemplateMeta,
        callback: Box<dyn Fn(arpa::pipeline::Status) + Send + Sync>,
    },
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Commit => write!(f, "Commit"),
            Self::Rollback => write!(f, "Rollback"),
            Self::DeleteItem(t, i) => {
                f.debug_tuple("DeleteItem").field(t).field(i).finish()
            }

            // Self::DownloadAllPulsars => write!(f, "DownloadAllPulsars"),
            Self::Download(dt, ft) => {
                f.debug_tuple("Download").field(dt).field(ft).finish()
            }
            Self::AddPulsar(pm) => {
                f.debug_tuple("AddPulsar").field(pm).finish()
            }
            Self::UpdatePulsar(i, pm) => {
                f.debug_tuple("UpdatePulsar").field(i).field(pm).finish()
            }

            Self::AddPar {
                path,
                pulsar,
                master,
            } => f
                .debug_struct("AddPar")
                .field("path", path)
                .field("pulsar", pulsar)
                .field("master", master)
                .finish(),
            Self::UpdatePar {
                id,
                path,
                pulsar,
                master,
            } => f
                .debug_struct("UpdatePar")
                .field("id", id)
                .field("path", path)
                .field("pulsar", pulsar)
                .field("master", master)
                .finish(),

            Self::SetupPipes {
                raw,
                ephemeride,
                template,
            } => f
                .debug_struct("SetupPipes")
                .field("raw", raw)
                .field("ephemeride", ephemeride)
                .field("template", template)
                .finish(),
            Self::RunPipeline {
                raw,
                ephemeride,
                template,
                ..
            } => f
                .debug_struct("RunPipeline")
                .field("raw", raw)
                .field("ephemeride", ephemeride)
                .field("template", template)
                .finish_non_exhaustive(),
        }
    }
}

impl Request {
    pub async fn handle(self, archivist: &mut Archivist) -> Message {
        info!("Handling {self:?}");

        let response: Result<Message, ARPAError> = match self {
            Self::Commit => archivist
                .commit_transaction()
                .await
                .map(|()| Message::CommitSuccess)
                .map_err(ARPAError::from),
            Self::Rollback => archivist
                .rollback_transaction()
                .await
                .map(|()| Message::RollbackSuccess)
                .map_err(ARPAError::from),

            // ---- Generics --------------------------------------------------
            Self::DeleteItem(dt, id) => match dt {
                DataType::Pulsar => archivist.delete::<PulsarMeta>(id).await,
                DataType::Ephemeride => archivist.delete::<ParMeta>(id).await,
                DataType::Toa => archivist.delete::<TOAInfo>(id).await,
            }
            .map(|()| Message::ItemDeleted(dt, id))
            .map_err(ARPAError::from),

            // ---- Pulsars ---------------------------------------------------
            Self::Download(DataType::Pulsar, FetchType::All) => archivist
                .get_all()
                .await
                .map(Message::Pulsars)
                .map_err(ARPAError::from),
            Self::Download(DataType::Pulsar, FetchType::Id(id)) => archivist
                .get(id)
                .await
                .map(Message::SinglePulsar)
                .map_err(ARPAError::from),
            Self::AddPulsar(meta) => archivist
                .insert(meta)
                .await
                .map(|id| Message::ItemAdded(DataType::Pulsar, id))
                .map_err(ARPAError::from),
            Self::UpdatePulsar(id, meta) => archivist
                .update_from_cache::<PulsarMeta>(&meta, id)
                .await
                .map(|()| Message::ItemUpdated(DataType::Pulsar, id))
                .map_err(ARPAError::from),

            // ---- Ephemerides -----------------------------------------------
            Self::Download(DataType::Ephemeride, FetchType::All) => {
                get_pars(archivist).await.map(Message::Ephemerides)
            }
            Self::Download(DataType::Ephemeride, FetchType::Id(id)) => {
                get_par(archivist, id).await.map(Message::SingleEphemeride)
            }
            Self::AddPar {
                path,
                pulsar,
                master,
            } => add_par(archivist, path, &pulsar, master)
                .await
                .map(|id| Message::ItemAdded(DataType::Ephemeride, id)),
            Self::UpdatePar {
                id,
                path,
                pulsar,
                master,
            } => overwrite_par(archivist, id, path, &pulsar, master)
                .await
                .map(|id| Message::ItemAdded(DataType::Ephemeride, id)),

            // ---- TOAs ------------------------------------------------------
            Self::Download(DataType::Toa, FetchType::All) => {
                get_toas(archivist).await.map(Message::TOAs)
            }
            Self::Download(DataType::Toa, FetchType::Id(id)) => {
                get_toa(archivist, id).await.map(Message::SingleTOA)
            }

            // ---- Pipeline --------------------------------------------------
            Self::SetupPipes {
                raw,
                ephemeride,
                template,
            } => set_up_pipes(archivist, raw, ephemeride, template)
                .await
                .map(|(r, p, t)| Message::PipesSetUp(r, p, t)),
            Self::RunPipeline {
                raw,
                ephemeride,
                template,
                callback,
            } => pipeline::cook(
                archivist, raw, ephemeride, template, true, callback,
            )
            .await
            .map(|()| Message::PipelineFinished),
        };

        response.unwrap_or_else(Message::Error)
    }
}

async fn set_up_pipes(
    archivist: &mut Archivist,
    raw: String,
    ephemeride: String,
    template: String,
) -> Result<(RawMeta, Option<ParMeta>, TemplateMeta), ARPAError> {
    let raw = pipeline::parse_input_raw(archivist, raw.trim()).await?;

    let par_text = ephemeride.trim();
    let par = if par_text.is_empty() {
        None
    } else {
        Some(pipeline::parse_input_ephemeride(archivist, &raw, par_text).await?)
    };

    let template =
        pipeline::parse_input_template(archivist, &raw, template.trim())
            .await?;

    Ok((raw, par, template))
}

async fn get_toas(archivist: &Archivist) -> Result<Vec<TOAData>, ARPAError> {
    let metas = archivist.get_all::<TOAInfo>().await?;
    let mut toas = Vec::new();
    for meta in metas {
        toas.push(make_toa_data(archivist, meta).await?);
    }
    Ok(toas)
}

async fn get_toa(archivist: &Archivist, id: i32) -> Result<TOAData, ARPAError> {
    let meta = archivist.get::<TOAInfo>(id).await?;
    make_toa_data(archivist, meta).await
}

async fn make_toa_data(
    archivist: &Archivist,
    meta: TOAInfo,
) -> Result<TOAData, ARPAError> {
    let pulsar = archivist.get::<PulsarMeta>(meta.pulsar_id).await?.alias;

    let time = f64::from(meta.toa_int) + meta.toa_frac;

    Ok(TOAData {
        id: TableItem::id(&meta),
        process: meta.process_id,
        pulsar,
        observer: meta.observer_id,
        template: meta.template_id,
        frequency: meta.frequency,
        time,
        error: meta.toa_err,
    })
}

async fn get_pars(archivist: &Archivist) -> Result<Vec<ParData>, ARPAError> {
    let metas = archivist.get_all::<ParMeta>().await?;
    let mut pars = Vec::new();
    for meta in metas {
        pars.push(make_par_data(archivist, meta).await?);
    }
    Ok(pars)
}

async fn get_par(archivist: &Archivist, id: i32) -> Result<ParData, ARPAError> {
    let meta = archivist.get::<ParMeta>(id).await?;
    make_par_data(archivist, meta).await
}

async fn make_par_data(
    archivist: &Archivist,
    meta: ParMeta,
) -> Result<ParData, ARPAError> {
    let pulsar_name = archivist.get::<PulsarMeta>(meta.pulsar_id).await?.alias;

    Ok(ParData {
        id: meta.id,
        pulsar_id: meta.pulsar_id,
        pulsar_name,
        path: meta.file_path,
    })
}

async fn overwrite_par(
    archivist: &mut Archivist,
    id: i32,
    path: PathBuf,
    pulsar: &str,
    master: bool,
) -> Result<i32, ARPAError> {
    let pid = parse_pulsar(archivist, pulsar).await?;
    let meta = ParMeta::new(path.to_string_lossy().to_string(), pid)?;
    archivist.update_from_cache(&meta, id).await?;

    if master {
        archivist
            .update(
                arpa::Table::PulsarMetas,
                pid,
                &format!("master_parfile_id={id}"),
            )
            .await?;
    }

    Ok(id)
}

async fn add_par(
    archivist: &mut Archivist,
    path: PathBuf,
    pulsar: &str,
    master: bool,
) -> Result<i32, ARPAError> {
    let pid = parse_pulsar(archivist, pulsar).await?;
    let meta = ParMeta::new(path.to_string_lossy().to_string(), pid)?;
    let id = archivist.insert(meta).await?;

    if master {
        archivist
            .update(
                arpa::Table::PulsarMetas,
                pid,
                &format!("master_parfile_id={id}"),
            )
            .await?;
    }

    Ok(id)
}

/// Parses a `&str` as either a pulsar id or alias.
async fn parse_pulsar(
    archivist: &Archivist,
    pulsar: &str,
) -> Result<i32, ARPAError> {
    if let Ok(id) = pulsar.parse::<i32>() {
        archivist.assert_id(arpa::Table::PulsarMetas, id).await?;
        Ok(id)
    } else {
        // We need to find by name...
        archivist
            .find::<PulsarMeta>(&format!("alias='{pulsar}'"))
            .await?
            .map_or_else(
                || {
                    Err(ARPAError::CantFind(format!(
                        "Pulsar with alias \"{pulsar}\""
                    )))
                },
                |meta| Ok(meta.id),
            )
    }
}
