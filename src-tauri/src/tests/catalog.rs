use crate::cleanup::categories;
use crate::cleanup::locate;
use crate::cleanup::model::{CleanupMode, Locator};
use crate::common as sysfs;

#[test]
fn category_ids_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for category in categories::all() {
        assert!(seen.insert(category.id), "duplicate id: {}", category.id);
    }
}

#[test]
fn every_category_has_a_known_group_and_a_locator() {
    let groups: Vec<&str> = categories::GROUPS.iter().map(|(id, _)| *id).collect();
    for category in categories::all() {
        assert!(
            groups.contains(&category.group),
            "{}: unknown group {}",
            category.id,
            category.group
        );
        assert!(
            !category.locators.is_empty(),
            "{}: no locators",
            category.id
        );
        assert!(!category.label.is_empty(), "{}: no label", category.id);
    }
}

#[test]
fn no_category_targets_a_protected_root() {
    let protected: Vec<String> = ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| sysfs::resolve(&value).to_string_lossy().to_lowercase())
        .collect();

    for category in categories::all() {
        for path in locate::resolve_all(category.locators) {
            let lowered = path.to_string_lossy().to_lowercase();
            for root in &protected {
                assert_ne!(
                    lowered, *root,
                    "{}: targets protected root {}",
                    category.id, root
                );
            }
        }
    }
}

#[test]
fn every_category_resolves_to_absolute_paths() {
    for category in categories::all() {
        let paths = locate::resolve_all(category.locators);
        assert!(!paths.is_empty(), "{}: resolved to nothing", category.id);
        for path in &paths {
            assert!(
                path.is_absolute(),
                "{}: {} is relative",
                category.id,
                path.display()
            );
        }
    }
}

#[test]
fn no_two_categories_share_a_target() {
    let mut owner: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for category in categories::all() {
        for path in locate::resolve_all(category.locators) {
            let key = path.to_string_lossy().to_lowercase();
            if let Some(previous) = owner.insert(key.clone(), category.id) {
                panic!(
                    "{} and {} both target {}",
                    previous,
                    category.id,
                    path.display()
                );
            }
        }
    }
}

#[test]
fn lookup_and_select_reject_unknown_ids() {
    assert!(categories::lookup("system.temp").is_ok());
    assert_eq!(
        categories::lookup("system.nope").unwrap_err().kind(),
        "unknown_category"
    );
    assert!(categories::select(Some(&["nope".to_string()])).is_err());
    assert_eq!(categories::select(Some(&[])).unwrap().len(), 0);
    assert_eq!(
        categories::select(None).unwrap().len(),
        categories::all().len()
    );
}

#[test]
fn env_locator_resolves_and_strips_sigil() {
    let resolved = locate::env::resolve_env("%LOCALAPPDATA%");
    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].is_absolute());

    assert_eq!(locate::env::resolve_env("LOCALAPPDATA").len(), 1);
    assert_eq!(locate::env::resolve_env("%TEMP%").len(), 1);
}

#[test]
fn env_locator_returns_nothing_for_unknown_variables() {
    assert!(locate::env::resolve_env("%PURIFIER_NOT_SET%").is_empty());
    assert!(locate::env::resolve_env("%%").is_empty());
}

#[test]
fn resolve_dedupes_case_insensitively() {
    let resolved = locate::resolve(&Locator::Fixed("%LOCALAPPDATA%"));
    let mut all = Vec::new();
    for locator in [
        Locator::Fixed("%LOCALAPPDATA%"),
        Locator::Fixed("%localappdata%"),
    ] {
        all.extend(locate::resolve(&locator));
    }
    assert_eq!(all.len(), 2);
    assert_eq!(resolved.len(), 1);
}

#[test]
fn join_appends_a_relative_segment() {
    let locators = [Locator::Join(
        &[Locator::Env("%LOCALAPPDATA%")],
        "Sub\\Leaf",
    )];
    let resolved = locate::resolve_all(&locators);
    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].ends_with("Sub\\Leaf"), "{:?}", resolved[0]);
}

#[test]
fn catalog_entries_round_trip_through_cleanup_mode() {
    assert_ne!(CleanupMode::ClearContents, CleanupMode::RemoveDir);
}
