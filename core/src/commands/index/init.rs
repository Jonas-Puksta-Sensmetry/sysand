use std::{
    fs::{self},
    io::{ErrorKind, Write},
};

use camino::{Utf8Path, Utf8PathBuf};
use fluent_uri::Iri;
use thiserror::Error;

use crate::{
    env::index::{IndexJson, VersionsJson},
    index::INDEX_PATH,
    project::{
        CanonicalizationError, ProjectRead as _,
        local_kpar::{LocalKParError, LocalKParProject},
        utils::{FsIoError, wrapfs},
    },
    purl::{self, SysandPurlError, is_valid_name, is_valid_publisher, parse_sysand_purl},
};

#[derive(Error, Debug)]
pub enum IndexInitError {
    #[error("`sysand index init` cannot be run on an existing index")]
    AlreadyExists,
    #[error("failed to serialize index.json")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to write index.json")]
    WriteError(#[from] Box<FsIoError>),
}

// impl From<FsIoError> for IndexInitError {
//     fn from(v: FsIoError) -> Self {
//         IndexInitError::WriteError(Box::new(v))
//     }
// }

pub fn do_index_init() -> Result<(), IndexInitError> {
    let creating = "Creating";
    let header = crate::style::get_style_config().header;
    log::info!("{header}{creating:>12}{header:#} index");
    let index = IndexJson { projects: vec![] };
    let index_serialized = serde_json::to_string(&index).map_err(serde_json::Error::from)?;
    let index_path = Utf8PathBuf::from(INDEX_PATH);
    // TODO(JP): ask to review this
    let mut file = fs::File::create_new(&index_path).map_err(|err| match err.kind() {
        ErrorKind::AlreadyExists => IndexInitError::AlreadyExists,
        _ => IndexInitError::WriteError(Box::new(FsIoError::CreateFile(index_path.clone(), err))),
    })?;
    file.write(index_serialized.as_bytes()).map_err(|err| {
        IndexInitError::WriteError(Box::new(FsIoError::WriteFile(index_path, err)))
    })?;
    Ok(())
}

// pub fn do_index_init() -> Result<(), IndexInitError> {
//     let creating = "Creating";
//     let header = crate::style::get_style_config().header;
//     log::info!("{header}{creating:>12}{header:#} index");
//     let index = IndexJson { projects: vec![] };
//     let index_serialized = serde_json::to_string(&index).map_err(serde_json::Error::from)?;
//     let index_path = Utf8PathBuf::from(INDEX_PATH);
//     if wrapfs::is_file(&index_path)? {
//         return Err(IndexInitError::AlreadyExists);
//     }
//     wrapfs::write(&index_path, index_serialized.as_bytes())?;
//     Ok(())
// }

#[derive(Error, Debug)]
pub enum IndexAddError {
    #[error(transparent)]
    FailedToAbsolute(#[from] Box<FsIoError>),
    #[error(".project.json file is missing from KPAR {0}")]
    MissingInfo(Utf8PathBuf),
    #[error(".meta.json file is missing from the KPAR {0}")]
    MissingMeta(Utf8PathBuf),
    #[error("Failed to compute project digest")]
    ProjectDigest(#[from] CanonicalizationError<LocalKParError>),
    #[error(transparent)]
    ProjectRead(#[from] LocalKParError),
    // TODO(JP): make sure sysand purl error states the problematic purl
    #[error("Provide")]
    InvalidSysandPurl(#[from] SysandPurlError),
    #[error("Invalid publisher in .project.json")]
    InvalidPublisherInProject,
    #[error("Invalid name in .project.json")]
    InvalidNameInProject,
}

/// Iri is only
pub fn do_index_add<P: AsRef<Utf8Path>>(
    kpar_path: P,
    // TODO(JP): review the type. Should it be Iri<&str>? But then conversion will happen somewhere
    // and it might report a worse error when e.g. publisher has a space
    iri: Option<&str>,
) -> Result<(), IndexAddError> {
    let kpar_path = wrapfs::absolute(kpar_path)?;
    // TODO(JP)(review): do we want to allow root to be in non-standard place?
    let local_project = LocalKParProject::new_guess_root(&kpar_path).map_err(LocalKParError::Io)?;
    let Some(info) = local_project.get_info()? else {
        return Err(IndexAddError::MissingInfo(kpar_path.clone()));
    };
    let Some(meta) = local_project.get_meta()? else {
        return Err(IndexAddError::MissingMeta(kpar_path));
    };
    let project_digest = local_project
        .checksum_canonical_hex()?
        .expect("This should only be None when .project.json or .meta.json is missing");

    let project_path: Utf8PathBuf = match (iri, info.publisher) {
        "COULD SAY that providing a sysand purl (or not providing a purl and constructing it from name and publisher)"
        "means you agree to be checked against the stricter sysand quality checks"
        (Some(iri), _) => {
            if let Some((publisher, name)) = purl::parse_sysand_purl(iri)? {
                "name (and if publisher exists) should coinside with those from purl, after normalization"
                "if not, return an error for now"
                Utf8PathBuf::from(format!(
                    "{}/{}",
                    purl::normalize_field(&publisher),
                    purl::normalize_field(&info.name)
                ))
            } else {
                todo!();
            }
        }
        (None, Some(publisher)) => {
            // TODO(JP): extract this into a purl function like create_sysand_purl
            if !purl::is_valid_publisher(&publisher) {
                return Err(IndexAddError::InvalidPublisherInProject);
            }
            if !purl::is_valid_name(&info.name) {
                return Err(IndexAddError::InvalidNameInProject);
            }
            Utf8PathBuf::from(format!(
                "{}/{}",
                purl::normalize_field(&publisher),
                purl::normalize_field(&info.name)
            ))
        }
    };
    "now deserialize versions json, insert the required version, save it, and save .project.json, .meta.json, and kpar in appropriate directory"
    VersionsJson

    Ok(())
}
