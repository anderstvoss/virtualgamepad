use gr_curated_controllers::registry::FAMILIES;
use std::{collections::BTreeSet, path::Path};
use toml::Value;

fn strings(value: &Value) -> Option<BTreeSet<&str>> {
    value.as_array()?.iter().map(Value::as_str).collect()
}

fn live_evidence_exists(root: &Path, cell: &Value) -> bool {
    cell.get("live_evidence")
        .and_then(Value::as_str)
        .is_some_and(|reference| {
            let path = reference
                .split_once('#')
                .map_or(reference, |(path, _)| path);
            let path = Path::new(path);
            path.starts_with("docs")
                && path
                    .components()
                    .all(|part| matches!(part, std::path::Component::Normal(_)))
                && path.extension().is_some_and(|extension| extension == "md")
                && root.join(path).is_file()
        })
}

fn valid(root: &Path, value: &Value, id: &str, targets: &BTreeSet<&str>) -> bool {
    let Some(revision) = value["research_revision"].as_str() else {
        return false;
    };
    if value["schema_version"].as_integer() != Some(1)
        || value["controller"].as_str() != Some(id)
        || revision.len() != 40
        || !revision.bytes().all(|b| b.is_ascii_hexdigit())
        || strings(&value["realizations"]).as_ref() != Some(targets)
        || strings(&value["refused_operations"]).is_none()
        || strings(&value["persistent_resources"]).is_none()
    {
        return false;
    }
    let Some(functions) = value["functions"].as_array() else {
        return false;
    };
    let mut ids = BTreeSet::new();
    !functions.is_empty()
        && functions.iter().all(|function| {
            let Some(id) = function["id"].as_str().filter(|id| !id.is_empty()) else {
                return false;
            };
            let Some(cells) = function["cells"].as_table() else {
                return false;
            };
            ids.insert(id)
                && cells.keys().map(String::as_str).collect::<BTreeSet<_>>() == *targets
                && cells.values().all(|cell| {
                    let status = cell["status"].as_str().unwrap_or_default();
                    let Some(evidence) = strings(&cell["evidence"]) else {
                        return false;
                    };
                    matches!(
                        status,
                        "not-supported"
                            | "wip"
                            | "source-evidence"
                            | "virtual-live"
                            | "physical-evidence"
                    ) && cell["test_id"].as_str().is_some_and(|s| !s.is_empty())
                        && evidence
                            .iter()
                            .all(|id| !id.is_empty() && !id.contains('/'))
                        && match status {
                            "source-evidence" | "physical-evidence" => !evidence.is_empty(),
                            "virtual-live" => {
                                live_evidence_exists(root, cell)
                                    && cell["reason"].as_str().is_some_and(|s| !s.is_empty())
                            }
                            _ => cell["reason"].as_str().is_some_and(|s| !s.is_empty()),
                        }
                })
        })
}

#[test]
fn every_compiled_family_has_complete_declared_realization_cells() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = repo.join("acceptance");
    let files: BTreeSet<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
        })
        .collect();
    assert_eq!(
        files,
        FAMILIES
            .iter()
            .map(|f| format!("{}.toml", f.id.as_str()))
            .collect()
    );
    for family in FAMILIES {
        let text =
            std::fs::read_to_string(root.join(format!("{}.toml", family.id.as_str()))).unwrap();
        let value: Value = toml::from_str(&text).unwrap();
        let manifest = (family.manifest)();
        let targets = manifest
            .entries()
            .iter()
            .map(|e| e.target.as_str())
            .collect();
        assert!(
            valid(&repo, &value, family.id.as_str(), &targets),
            "{}",
            family.id.as_str()
        );
        let mut invalid = value.clone();
        invalid["functions"][0]["cells"]
            .as_table_mut()
            .unwrap()
            .remove(*targets.first().unwrap());
        assert!(
            !valid(&repo, &invalid, family.id.as_str(), &targets),
            "missing cell"
        );
        let mut invalid = value.clone();
        let cell = invalid["functions"][0]["cells"]
            .as_table_mut()
            .unwrap()
            .iter_mut()
            .next()
            .unwrap()
            .1;
        cell["status"] = Value::String("accepted".into());
        assert!(
            !valid(&repo, &invalid, family.id.as_str(), &targets),
            "unknown status"
        );
        let mut invalid = value.clone();
        invalid["functions"][0]["cells"]
            .as_table_mut()
            .unwrap()
            .insert("undeclared".into(), Value::Table(toml::Table::default()));
        assert!(
            !valid(&repo, &invalid, family.id.as_str(), &targets),
            "undeclared realization"
        );
    }
}

#[test]
fn virtual_live_requires_a_repository_evidence_report() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = repo.join("acceptance/virtualgamepad.steam-controller-2026.toml");
    let mut value: Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let family = "virtualgamepad.steam-controller-2026";
    let targets = strings(&value["realizations"])
        .unwrap()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let targets = targets.iter().map(String::as_str).collect();
    assert!(valid(&repo, &value, family, &targets));

    for missing_or_invalid in [
        None,
        Some("docs/missing-run.md"),
        Some("docs/../docs/C6_D9_ACCEPTANCE.md"),
    ] {
        let cell = value["functions"][0]["cells"]["linux.uinput"]
            .as_table_mut()
            .unwrap();
        assert_eq!(cell["status"].as_str(), Some("virtual-live"));
        cell.remove("live_evidence");
        if let Some(reference) = missing_or_invalid {
            cell.insert("live_evidence".into(), Value::String(reference.into()));
        }
        assert!(!valid(&repo, &value, family, &targets));
    }
}
