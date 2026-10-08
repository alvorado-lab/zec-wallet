//! The sync-server status spells its URLs as the offered entry they name
//! (Relim found it).
//!
//! The picker marks the row in use, and hides "App default", by comparing
//! strings. A host default of `https://zec.rocks` beside the catalog's
//! `https://zec.rocks:443` therefore listed zec.rocks twice and marked neither.
//! The same-server rule is the core's ([`rw::LightServerEndpoint::same_server`],
//! the one P3-13 names); it is applied HERE, in the DTO, so the picker never
//! needs a copy of it (a Dart copy was written and withdrawn the same day).
//!
//! Its own module, not a helper in `convert.rs`: that file's test lines carry
//! registered mutant citations, and a helper above them would move every one.

use zec_wallet_core as rw;

use crate::api::config as api_config;

/// The status DTO with both URLs spelled as the offered entry they name.
pub(crate) fn as_offered(
    mut status: rw::SyncServerStatus,
    offered: &[rw::SyncServer],
) -> api_config::SyncServerStatus {
    status.effective = spelled_as_offered(status.effective, offered);
    status.default = spelled_as_offered(status.default, offered);
    status.into()
}

/// `endpoint` as it is when an offered entry already spells it EXACTLY (a
/// predefined choice resolves to its own entry's endpoint, so the row it names
/// is the one marked — never a same-server sibling listed earlier, which may
/// carry other auth; review); otherwise the first offered entry naming
/// the same server; otherwise as it is.
fn spelled_as_offered(
    endpoint: rw::LightServerEndpoint,
    offered: &[rw::SyncServer],
) -> rw::LightServerEndpoint {
    let endpoints = || offered.iter().map(rw::SyncServer::endpoint);
    if endpoints().any(|e| e.as_str() == endpoint.as_str()) {
        return endpoint;
    }
    endpoints()
        .find(|e| e.same_server(&endpoint))
        .cloned()
        .unwrap_or(endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relim's shape — a bare default in use, the catalog entry offered — is
    /// one row, marked in use. A different port is a different server and
    /// keeps its own spelling; nothing offered leaves a URL as it is.
    #[test]
    fn the_status_spells_a_url_as_the_offered_server_it_names() {
        let url = |u: &str| rw::LightServerEndpoint::new(u).expect("endpoint");
        let offered = [rw::SyncServer::new(
            rw::SyncServerId::new("zec-rocks").expect("id"),
            "zec.rocks",
            url("https://zec.rocks:443"),
            None,
        )
        .expect("server")];
        let status = as_offered(
            rw::SyncServerStatus {
                effective: url("https://ZEC.rocks"),
                default: url("https://zec.rocks"),
                choice: None,
                fallback: None,
            },
            &offered,
        );
        assert_eq!(status.effective_url, "https://zec.rocks:443");
        assert_eq!(status.default_url, "https://zec.rocks:443");
        assert_eq!(
            spelled_as_offered(url("https://zec.rocks:9067"), &offered).as_str(),
            "https://zec.rocks:9067"
        );
        assert_eq!(
            spelled_as_offered(url("https://other.example"), &[]).as_str(),
            "https://other.example"
        );

        // Two offered entries naming one server, spelled differently: the
        // choice resolved to the SECOND, whose own string it carries, so that
        // is the row marked — never the first same-server sibling.
        let entry = |id: &str, u: &str| {
            rw::SyncServer::new(rw::SyncServerId::new(id).expect("id"), id, url(u), None)
                .expect("server")
        };
        let twins = [
            entry("bare", "https://zec.rocks"),
            entry("ported", "https://zec.rocks:443"),
        ];
        let chosen = as_offered(
            rw::SyncServerStatus {
                effective: url("https://zec.rocks:443"),
                default: url("https://zec.rocks"),
                choice: Some(rw::SyncServerChoice::Predefined(
                    rw::SyncServerId::new("ported").expect("id"),
                )),
                fallback: None,
            },
            &twins,
        );
        assert_eq!(chosen.effective_url, "https://zec.rocks:443");
    }
}
