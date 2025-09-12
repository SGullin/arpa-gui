use std::path::PathBuf;

use arpa::{
    data_types::{ParMeta, PulsarMeta, RawMeta, TOAInfo, TemplateMeta}, pipeline, ARPAError, Archivist
};
use log::info;

use crate::app::{ephemerides::ParData, helpers::downloader::FetchType};

#[derive(Debug)]
pub(crate) enum DataType {
    Pulsar,
    Ephemeride,
    TOA,
}
impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Pulsar => write!(f, "pulsar"),
            DataType::Ephemeride => write!(f, "ephemeride"),
            DataType::TOA => write!(f, "TOA"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Message {
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
    TOAs(Vec<TOAInfo>),
    /// Downloaded TOA.
    SingleTOA(TOAInfo),
    
    // ---- Pipeline ----------------------------------------------------------
    /// Response if set up is ok.
    PipesSetUp(RawMeta, Option<ParMeta>, TemplateMeta),
    /// Response if pipeline cooked properly.
    PipelineFinished,
    /// Status message.
    PipelineStatus(pipeline::Status),
}

pub(crate) enum Request {
    /// Commit a live transaction.
    Commit,
    /// Roll back a live transaction.
    Rollback,
    
    // ---- Generics ----------------------------------------------------------
    /// Delete something froma a table.
    DeleteItem(DataType, i32),

    // ---- Pulsars -----------------------------------------------------------
    /// Download _all_ pulsars.
    DownloadAllPulsars,
    /// Download _one_ pulsar.
    DownloadPulsarById(i32),
    /// Add a new pulsar.
    AddPulsar(PulsarMeta),
    /// Overwrite an existing pulsar.
    UpdatePulsar(i32, PulsarMeta),

    // ---- Ephemerides -------------------------------------------------------
    /// Download _all_ ephemeride.
    DownloadAllEphemerides,
    /// Download _one_ ephemerides.
    DownloadEphemerideById(i32),
    /// Add one ephemeride
    AddPar { path: PathBuf, pulsar: String, master: bool },
    /// Overwrite an existing ephemeride.
    UpdatePar { id: i32, path: PathBuf, pulsar: String, master: bool },

    // ---- TOAs --------------------------------------------------------------
    /// Download some TOAs.
    DownloadTOAs(FetchType),

    // ---- Pipeline ----------------------------------------------------------
    /// Load files to set up pipeline job.
    SetupPipes { raw: String, ephemeride: String, template: String },
    /// Run the pipeline with the selected files.
    RunPipeline { 
        raw: RawMeta, 
        ephemeride: Option<ParMeta>, 
        template: TemplateMeta,
        callback: Box<dyn Fn(arpa::pipeline::Status)+Send+Sync>,
    },
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Commit => write!(f, "Commit"),
            Self::Rollback => write!(f, "Rollback"),
            Self::DeleteItem(t, i) => 
                f.debug_tuple("DeleteItem").field(t).field(i).finish(),

            Self::DownloadAllPulsars => write!(f, "DownloadAllPulsars"),
            Self::DownloadPulsarById(i) => 
                f.debug_tuple("DownloadPulsarById").field(i).finish(),
            Self::AddPulsar(pm) => 
                f.debug_tuple("AddPulsar").field(pm).finish(),
            Self::UpdatePulsar(i, pm) => 
                f.debug_tuple("UpdatePulsar").field(i).field(pm).finish(),

            Self::DownloadAllEphemerides => 
                write!(f, "DownloadAllEphemerides"),
            Self::DownloadEphemerideById(i) => 
                f.debug_tuple("DownloadEphemerideById").field(i).finish(),
            Self::AddPar { path, pulsar, master } => 
                f.debug_struct("AddPar")
                .field("path", path)
                .field("pulsar", pulsar)
                .field("master", master)
                .finish(),
            Self::UpdatePar { id, path, pulsar, master } => 
                f.debug_struct("UpdatePar")
                .field("id", id)
                .field("path", path)
                .field("pulsar", pulsar)
                .field("master", master)
                .finish(),

            // Self::DownloadAllTOAs => write!(f, "DownloadAllTOAs"),
            // Self::DownloadTOAById(id) => 
            //     f.debug_tuple("DownloadTOAById")
            //     .field(id)
            //     .finish(),
            Self::DownloadTOAs(ft) => 
                f.debug_tuple("DownloadTOAs")
                .field(ft)
                .finish(),
            
            Self::SetupPipes { raw, ephemeride, template } => 
                f.debug_struct("SetupPipes")
                .field("raw", raw)
                .field("ephemeride", ephemeride)
                .field("template", template)
                .finish(),
            Self::RunPipeline { raw, ephemeride, template, .. } => 
                f.debug_struct("RunPipeline")
                .field("raw", raw)
                .field("ephemeride", ephemeride)
                .field("template", template)
                .finish_non_exhaustive(),
        }
    }
}

impl Request {
    pub async fn handle(self, archivist: &mut Archivist) -> Message {
        info!("Handling {:?}", self);
        use Message as M;

        let response: Result<M, ARPAError> = match self {
            Self::Commit => archivist
                .commit_transaction().await
                .map(|()| M::CommitSuccess)
                .map_err(ARPAError::from),
            Self::Rollback => archivist
                .rollback_transaction().await
                .map(|()| M::RollbackSuccess)
                .map_err(ARPAError::from),

            // ---- Generics --------------------------------------------------
            Self::DeleteItem(dt, id) => 
                match dt {
                    DataType::Pulsar => 
                        archivist.delete::<PulsarMeta>(id).await,
                    DataType::Ephemeride => 
                       archivist.delete::<ParMeta>(id).await,
                    DataType::TOA => 
                       archivist.delete::<TOAInfo>(id).await,
                }
                .map(|()| M::ItemDeleted(dt, id))
                .map_err(ARPAError::from),

            // ---- Pulsars ---------------------------------------------------
            Self::DownloadAllPulsars => archivist.get_all().await
                .map(M::Pulsars)
                .map_err(ARPAError::from),
            Self::DownloadPulsarById(id) => archivist.get(id).await
                .map(M::SinglePulsar)
                .map_err(ARPAError::from),
            Self::AddPulsar(meta) => archivist.insert(meta).await
                .map(|id| M::ItemAdded(DataType::Pulsar, id))
                .map_err(ARPAError::from),
            Self::UpdatePulsar(id, meta) => archivist
                .update_from_cache::<PulsarMeta>(&meta, id).await
                .map(|()| M::ItemUpdated(DataType::Pulsar, id))
                .map_err(ARPAError::from),

            // ---- Ephemerides -----------------------------------------------
            Self::DownloadAllEphemerides => get_pars(archivist).await
                .map(M::Ephemerides)
                .map_err(ARPAError::from),
            Self::DownloadEphemerideById(id) => get_par(archivist, id).await
                .map(M::SingleEphemeride)
                .map_err(ARPAError::from),
            Self::AddPar{ path, pulsar, master } => 
                add_par(archivist, path, &pulsar, master).await
                .map(|id| M::ItemAdded(DataType::Ephemeride, id)),
            Self::UpdatePar { id, path, pulsar, master } =>
                overwrite_par(archivist, id, path, &pulsar, master).await
                .map(|id| M::ItemAdded(DataType::Ephemeride, id)),     

            // ---- TOAs ------------------------------------------------------
            // Self::DownloadAllTOAs => archivist.get_all().await
            //     .map(M::TOAs)
            //     .map_err(ARPAError::from),
            // Self::DownloadTOAById(id) => archivist.get(id).await
            //     .map(M::SingleTOA)
            //     .map_err(ARPAError::from),
            Self::DownloadTOAs(FetchType::All) => archivist.get_all().await
                .map(M::TOAs)
                .map_err(ARPAError::from),
            Self::DownloadTOAs(FetchType::Id(id)) => archivist.get(id).await
                .map(M::SingleTOA)
                .map_err(ARPAError::from),
          
            // ---- Pipeline --------------------------------------------------
            Self::SetupPipes { raw, ephemeride, template } => 
                set_up_pipes(archivist, raw, ephemeride, template).await
                .map(|(r, p, t)| M::PipesSetUp(r, p, t)),
            Self::RunPipeline { raw, ephemeride, template, callback } =>
                pipeline::cook(
                    archivist, 
                    raw, 
                    ephemeride, 
                    template, 
                    true,
                    callback,
                ).await
                .map(|()| M::PipelineFinished)
        };

        response.map_or_else(
            Message::Error, 
            |m| m
        )
    }
}

async fn set_up_pipes(
    archivist: &mut Archivist,
    raw: String, 
    ephemeride: String, 
    template: String
) -> Result<(
    RawMeta,
    Option<ParMeta>,
    TemplateMeta,
), ARPAError> {
    let raw = pipeline::parse_input_raw(archivist, raw.trim()).await?;
    
    let par_text = ephemeride.trim();
    let par = if par_text.is_empty() { None } else {
        Some(pipeline::parse_input_ephemeride(
            archivist, 
            &raw, 
            par_text,
        ).await?)
    };

    let template = pipeline::parse_input_template(
        archivist, 
        &raw, 
        template.trim(),
    ).await?;

    Ok((
        raw,
        par,
        template,
    ))
}

async fn get_pars(
    archivist: &mut Archivist,
) -> Result<Vec<ParData>, ARPAError> {
    let metas = archivist.get_all::<ParMeta>().await?;
    let mut pars = Vec::new();
    for meta in metas {
        pars.push(make_par_data(archivist, meta).await?);
    }
    Ok(pars)
}

async fn get_par(
    archivist: &mut Archivist,
    id: i32,
) -> Result<ParData, ARPAError> {
    let meta = archivist.get::<ParMeta>(id).await?;
    make_par_data(archivist, meta).await
}

async fn make_par_data(
    archivist: &mut Archivist, 
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
        archivist.update(
            arpa::Table::PulsarMetas, 
            pid, 
            &format!("master_parfile_id={id}"),
        ).await?;
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
        archivist.update(
            arpa::Table::PulsarMetas, 
            pid, 
            &format!("master_parfile_id={id}"),
        ).await?;
    }

    Ok(id)
}

/// Parses a `&str` as either a pulsar id or alias.
async fn parse_pulsar(
    archivist: &mut Archivist, 
    pulsar: &str,
) -> Result<i32, ARPAError> {
    if let Ok(id) = pulsar.parse::<i32>() {
        archivist.assert_id(arpa::Table::PulsarMetas, id).await?;
        Ok(id)
    }
    else {
        // We need to find by name...
        archivist.find::<PulsarMeta>(&format!("alias='{pulsar}'")).await?
            .map_or_else(
                || Err(ARPAError::CantFind(format!("Pulsar with alias \"{pulsar}\""))),
                |meta| Ok(meta.id)
            )
    }
}
