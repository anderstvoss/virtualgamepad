use gr_curated_controllers::registry::FAMILIES;
use std::{collections::BTreeSet, path::Path};
use toml::Value;

fn strings(value: &Value) -> Option<BTreeSet<&str>> {
    value.as_array()?.iter().map(Value::as_str).collect()
}

fn valid(value: &Value, id: &str, targets: &BTreeSet<&str>) -> bool {
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
                        "not-supported" | "wip" | "source-evidence" | "physical-evidence"
                    ) && cell["test_id"].as_str().is_some_and(|s| !s.is_empty())
                        && evidence
                            .iter()
                            .all(|id| !id.is_empty() && !id.contains('/'))
                        && if matches!(status, "source-evidence" | "physical-evidence") {
                            !evidence.is_empty()
                        } else {
                            cell["reason"].as_str().is_some_and(|s| !s.is_empty())
                        }
                })
        })
}

#[test]
fn every_compiled_family_has_complete_declared_realization_cells() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../acceptance");
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
            valid(&value, family.id.as_str(), &targets),
            "{}",
            family.id.as_str()
        );
        let mut invalid = value.clone();
        invalid["functions"][0]["cells"]
            .as_table_mut()
            .unwrap()
            .remove(*targets.first().unwrap());
        assert!(
            !valid(&invalid, family.id.as_str(), &targets),
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
            !valid(&invalid, family.id.as_str(), &targets),
            "unknown status"
        );
        let mut invalid = value.clone();
        invalid["functions"][0]["cells"]
            .as_table_mut()
            .unwrap()
            .insert("undeclared".into(), Value::Table(toml::Table::default()));
        assert!(
            !valid(&invalid, family.id.as_str(), &targets),
            "undeclared realization"
        );
    }
}
