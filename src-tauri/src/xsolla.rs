use crate::errors::{LauncherError, LauncherResult};
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XsollaPlan {
    pub target_version: Option<u32>,
    pub actions: Vec<XsollaAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XsollaAction {
    Download { url: String, size: u64, to: String },
    Extract { file: String, to: String },
    Patch { binaries: String, patch: String },
    Wait,
    Version { version: u32 },
}

pub fn parse_update_plan(xml: &str) -> LauncherResult<XsollaPlan> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut actions = Vec::new();
    let mut target_version = None;
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Empty(event)) | Ok(Event::Start(event))
                if event.name().as_ref() == b"action" =>
            {
                let attrs = event
                    .attributes()
                    .map(|attr| attr.map_err(|err| err.to_string()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|message| LauncherError::InvalidData {
                        context: "parsing Xsolla action attributes".into(),
                        message,
                    })?;
                let get = |name: &[u8]| -> Option<String> {
                    attrs
                        .iter()
                        .find(|attr| attr.key.as_ref() == name)
                        .map(|attr| String::from_utf8_lossy(attr.value.as_ref()).to_string())
                };
                match get(b"type").as_deref() {
                    Some("torrent_download") => actions.push(XsollaAction::Download {
                        url: get(b"alt_data_link").unwrap_or_default(),
                        size: get(b"data_size")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                        to: get(b"alt_to").unwrap_or_default(),
                    }),
                    Some("extract") => actions.push(XsollaAction::Extract {
                        file: get(b"file").unwrap_or_default(),
                        to: get(b"to").unwrap_or_default(),
                    }),
                    Some("patch") => actions.push(XsollaAction::Patch {
                        binaries: get(b"binaries").unwrap_or_default(),
                        patch: get(b"patch").unwrap_or_default(),
                    }),
                    Some("wait_actions") => actions.push(XsollaAction::Wait),
                    Some("version") => {
                        let version = get(b"version")
                            .and_then(|value| value.parse().ok())
                            .ok_or_else(|| LauncherError::InvalidData {
                                context: "parsing Xsolla version action".into(),
                                message: "version action missing numeric version".into(),
                            })?;
                        target_version = Some(version);
                        actions.push(XsollaAction::Version { version });
                    }
                    Some("extracted_size") => {}
                    Some(other) => {
                        return Err(LauncherError::InvalidData {
                            context: "parsing Xsolla action".into(),
                            message: format!("unknown action type {other}"),
                        });
                    }
                    None => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(LauncherError::InvalidData {
                    context: "parsing Xsolla XML".into(),
                    message: err.to_string(),
                });
            }
        }
        buffer.clear();
    }

    Ok(XsollaPlan {
        target_version,
        actions,
    })
}

pub fn normalize_relative_patch_path(path: &str) -> LauncherResult<String> {
    let mut components = Vec::new();
    for component in path.trim().split(['/', '\\']) {
        if component.is_empty() || component == "." || component == ".." {
            return Err(LauncherError::InvalidData {
                context: "normalizing patch path".into(),
                message: format!("invalid patch path {path}"),
            });
        }
        components.push(component);
    }
    if components.is_empty() {
        return Err(LauncherError::InvalidData {
            context: "normalizing patch path".into(),
            message: format!("invalid patch path {path}"),
        });
    }
    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_update_actions() {
        let plan =
            parse_update_plan(include_str!("../tests/fixtures/xsolla_plan.xml")).expect("parse");

        assert_eq!(plan.target_version, Some(169));
        assert_eq!(plan.actions.len(), 5);
        assert!(matches!(plan.actions[0], XsollaAction::Download { .. }));
        assert!(matches!(
            plan.actions[4],
            XsollaAction::Version { version: 169 }
        ));
    }

    #[test]
    fn rejects_patch_path_escape() {
        let error = normalize_relative_patch_path("../escape").expect_err("path escape rejected");
        assert!(error.to_string().contains("invalid patch path"));
    }
}
