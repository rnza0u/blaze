use std::{borrow::Cow, collections::HashMap, str::FromStr};

use actix_cors::Cors;
use actix_files::NamedFile;
use actix_web::{get, middleware::Logger, web::Json, App, HttpRequest, HttpServer, Responder};
use env_logger::Env;
use error::StringError;
use serde::Serialize;
use versions::{Build, Platform, VersionEntry};

use crate::{
    error::{Error, Result},
    versions::{get_package_download, list_builds, list_versions, VersionIdentifier},
};

const PORT_VAR: &str = "PORT";
const ADDR_VAR: &str = "ADDR";
const WEBSITE_ORIGIN_VAR: &str = "WEBSITE_ORIGIN";

const DEV_PORT: u16 = 3001;
const DEV_ADDR: &str = "127.0.0.1";

mod error;
mod versions;

#[derive(Serialize)]
struct VersionsResponse(Vec<String>);

impl From<Vec<VersionEntry>> for VersionsResponse {
    fn from(value: Vec<VersionEntry>) -> Self {
        Self(
            value
                .iter()
                .map(|entry| entry.version.to_string())
                .collect::<Vec<_>>(),
        )
    }
}

#[get("/versions")]
async fn versions_endpoint() -> Result<impl Responder> {
    Ok(Json(VersionsResponse::from(list_versions().await?)))
}

#[derive(Serialize)]
struct BuildsResponse(HashMap<Platform, Build>);

impl From<HashMap<Platform, Build>> for BuildsResponse {
    fn from(value: HashMap<Platform, Build>) -> Self {
        Self(value)
    }
}

#[get("/versions/{version}/builds")]
async fn builds_endpoint(req: HttpRequest) -> Result<impl Responder> {
    let version_identifier = VersionIdentifier::from_str(req.match_info().get("version").unwrap())?;
    Ok(Json(BuildsResponse::from(
        list_builds(&version_identifier).await?,
    )))
}

#[get("/versions/{version}/builds/{platform}/package")]
async fn package_endpoint(req: HttpRequest) -> Result<NamedFile> {
    let version_identifier = VersionIdentifier::from_str(req.match_info().get("version").unwrap())?;
    let platform = Platform::from_str(req.match_info().get("platform").unwrap())?;
    let download = get_package_download(&version_identifier, platform).await?;
    NamedFile::from_file(
        download.file.try_into_std().map_err(|_| {
            Error::any(StringError::from(
                "could not convert tokio file to std file",
            ))
        })?,
        format!("blaze_{}_{}.tar.gz", download.version, platform),
    )
    .map_err(Error::any)
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::default().filter_or("LOG_LEVEL", "off"));

    let addr = match std::env::var(ADDR_VAR) {
        Ok(addr) => Cow::Owned(addr),
        Err(std::env::VarError::NotPresent) => Cow::Borrowed(DEV_ADDR),
        Err(other) => return Err(Error::any(other)),
    };

    let port = match std::env::var(PORT_VAR) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|_| Error::Configuration("bad port was provided"))?,
        Err(std::env::VarError::NotPresent) => DEV_PORT,
        Err(other) => return Err(Error::any(other)),
    };

    let website_origin = std::env::var(WEBSITE_ORIGIN_VAR)
        .map_err(|_| Error::Configuration("frontend origin must be provided"))?;

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&website_origin)
            .allowed_methods(vec!["GET"])
            .max_age(3600);

        App::new()
            .service(versions_endpoint)
            .service(builds_endpoint)
            .service(package_endpoint)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind((addr.as_ref(), port))
    .map_err(Error::any)?
    .workers(1)
    .run()
    .await
    .map_err(Error::any)
}
