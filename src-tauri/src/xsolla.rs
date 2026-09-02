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
                if event.name().as_ref() == "action" =>
            {
                let attrs = event
                    .attributes()
                    .map(|attr| attr.map_err(|err| err.to_string()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|message| LauncherError::InvalidData {
                        context: "parsing Xsolla action attributes".into(),
                        message,
                    })?;
                let get = |name: &str| -> Option<String> {
                    attrs
                        .iter()
                        .find(|attr| attr.key.as_ref() == name)
                        .map(|attr| attr.value.to_string())
                };
                match get("type").as_deref() {
                    Some("torrent_download") => actions.push(XsollaAction::Download {
                        url: get("alt_data_link").unwrap_or_default(),
                        size: get("data_size")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0),
                        to: get("alt_to").unwrap_or_default(),
                    }),
                    Some("extract") => actions.push(XsollaAction::Extract {
                        file: get("file").unwrap_or_default(),
                        to: get("to").unwrap_or_default(),
                    }),
                    Some("patch") => actions.push(XsollaAction::Patch {
                        binaries: get("binaries").unwrap_or_default(),
                        patch: get("patch").unwrap_or_default(),
                    }),
                    Some("wait_actions") => actions.push(XsollaAction::Wait),
                    Some("version") => {
                        let version = get("version")
                            .and_then(|value| value.parse::<i32>().ok())
                            .ok_or_else(|| LauncherError::InvalidData {
                                context: "parsing Xsolla version action".into(),
                                message: "version action missing numeric version".into(),
                            })?;
                        // Xsolla emits a bare version="-1" marker when the
                        // install is already up to date; it carries no update
                        // target and must not produce a Version action.
                        if version >= 0 {
                            let version =
                                u32::try_from(version).map_err(|_| LauncherError::InvalidData {
                                    context: "parsing Xsolla version action".into(),
                                    message: format!("version action out of range: {version}"),
                                })?;
                            target_version = Some(version);
                            actions.push(XsollaAction::Version { version });
                        }
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
        // Xsolla roots rule paths at the install dir (leading '/'); empty and
        // "." components can't escape the game root, so skip them. ".." can.
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
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
    fn normalizes_rooted_patch_paths() {
        // Real 190 rule from a multi-box update log: Xsolla roots macOS rule
        // paths at the install dir with a leading '/'.
        assert_eq!(
            normalize_relative_patch_path(
                "/Star Trek Fleet Command.app/Contents/Resources/Data/StreamingAssets/Pre-Bundles/stationwarning/materials"
            )
            .expect("rooted path"),
            "Star Trek Fleet Command.app/Contents/Resources/Data/StreamingAssets/Pre-Bundles/stationwarning/materials"
        );
        assert_eq!(
            normalize_relative_patch_path("a//b/").expect("empty components"),
            "a/b"
        );
        // Traversal stays rejected.
        assert!(normalize_relative_patch_path("a/../b").is_err());
        assert!(normalize_relative_patch_path("/").is_err());
    }

    #[test]
    fn rejects_patch_path_escape() {
        let error = normalize_relative_patch_path("../escape").expect_err("path escape rejected");
        assert!(error.to_string().contains("invalid patch path"));
    }

    #[test]
    fn up_to_date_marker_yields_no_target_version() {
        // Real Xsolla response for an install that is already current: a bare
        // version="-1" action and nothing else.
        let plan = parse_update_plan(include_str!("../tests/fixtures/xsolla_no_update.xml"))
            .expect("parse no-update plan");

        assert_eq!(plan.target_version, None);
        assert!(plan.actions.is_empty());
    }
}
