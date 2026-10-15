pub mod auth;
pub mod automation;
pub mod ax25;
pub mod config;
pub mod digipeat;
pub mod i18n;
pub mod kiss;
pub mod link;
pub mod mailbox;
pub mod mheard;
pub mod mqtt;
#[cfg(unix)]
mod serial;
pub mod statistics;
pub mod storage;
pub mod telegram;
pub mod transport;
pub mod yapp;
pub mod update;

#[cfg(test)]
pub(crate) fn test_temp_dir(prefix: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-tmp")
        .join(format!("{prefix}-{}", std::process::id()))
}

mod history;
mod http;
mod json;
pub mod runtime;
mod station;
pub mod station_events;
mod transfers;
pub mod web_config;
