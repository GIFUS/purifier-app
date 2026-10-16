use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, REG_BINARY,
    REG_EXPAND_SZ, REG_SZ,
};
use winreg::{RegKey, RegValue};

use super::{
    SCOPE_MACHINE, SCOPE_USER, SOURCE_POLICY_RUN, SOURCE_RUN, SOURCE_RUN32, SOURCE_RUNONCE,
};
use crate::error::{AppError, Result};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUNONCE_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\RunOnce";
const POLICY_RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Policies\Explorer\Run";
const WOW_RUN_KEY: &str = r"SOFTWARE\Wow6432Node\Microsoft\Windows\CurrentVersion\Run";
const WOW_RUNONCE_KEY: &str = r"SOFTWARE\Wow6432Node\Microsoft\Windows\CurrentVersion\RunOnce";
const APPROVED_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

const STATE_ENABLED: u8 = 0x02;
const STATE_DISABLED: u8 = 0x03;

pub(super) struct RegistrySource {
    pub(super) source: &'static str,
    pub(super) scope: &'static str,
    pub(super) machine: bool,
    pub(super) run_key: &'static str,
    pub(super) approved_cell: &'static str,
    /// Windows keeps no state for such a key, so it is left out of the listing.
    pub(super) toggleable: bool,
    pub(super) reason: &'static str,
}

impl RegistrySource {
    pub(super) fn name(&self) -> String {
        let hive = if self.machine { "HKLM" } else { "HKCU" };
        format!("{hive}\\{}", self.run_key)
    }
}

pub(super) fn listed_sources() -> impl Iterator<Item = &'static RegistrySource> {
    REGISTRY_SOURCES.iter().filter(|source| source.toggleable)
}

const RUNONCE_REASON: &str =
    "runs once at the next sign-in and then removes itself; it cannot be disabled here";
const POLICY_REASON: &str =
    "set by a system policy and managed elsewhere; it cannot be disabled here";

pub(super) const REGISTRY_SOURCES: [RegistrySource; 8] = [
    RegistrySource {
        source: SOURCE_RUN,
        scope: SCOPE_USER,
        machine: false,
        run_key: RUN_KEY,
        approved_cell: "Run",
        toggleable: true,
        reason: "",
    },
    RegistrySource {
        source: SOURCE_RUN,
        scope: SCOPE_MACHINE,
        machine: true,
        run_key: RUN_KEY,
        approved_cell: "Run",
        toggleable: true,
        reason: "",
    },
    RegistrySource {
        source: SOURCE_RUNONCE,
        scope: SCOPE_USER,
        machine: false,
        run_key: RUNONCE_KEY,
        approved_cell: "Run",
        toggleable: false,
        reason: RUNONCE_REASON,
    },
    RegistrySource {
        source: SOURCE_RUNONCE,
        scope: SCOPE_MACHINE,
        machine: true,
        run_key: RUNONCE_KEY,
        approved_cell: "Run",
        toggleable: false,
        reason: RUNONCE_REASON,
    },
    RegistrySource {
        source: SOURCE_POLICY_RUN,
        scope: SCOPE_USER,
        machine: false,
        run_key: POLICY_RUN_KEY,
        approved_cell: "Run",
        toggleable: false,
        reason: POLICY_REASON,
    },
    RegistrySource {
        source: SOURCE_POLICY_RUN,
        scope: SCOPE_MACHINE,
        machine: true,
        run_key: POLICY_RUN_KEY,
        approved_cell: "Run",
        toggleable: false,
        reason: POLICY_REASON,
    },
    RegistrySource {
        source: SOURCE_RUN32,
        scope: SCOPE_MACHINE,
        machine: true,
        run_key: WOW_RUN_KEY,
        // Windows reads StartupApproved\Run32 here; "Run" is a key it never reads.
        approved_cell: "Run32",
        toggleable: true,
        reason: "",
    },
    RegistrySource {
        source: SOURCE_RUNONCE,
        scope: SCOPE_MACHINE,
        machine: true,
        run_key: WOW_RUNONCE_KEY,
        approved_cell: "Run",
        toggleable: false,
        reason: RUNONCE_REASON,
    },
];

/// No record means the user never touched it, which Windows allows.
pub(super) fn is_enabled(approved: &HashMap<String, u8>, name: &str) -> bool {
    approved
        .get(&name.to_lowercase())
        .is_none_or(|byte| byte & 1 == 0)
}

pub(super) fn approved_states(machine: bool, cell: &str) -> HashMap<String, u8> {
    let path = format!(r"{APPROVED_KEY}\{cell}");
    let Ok(key) =
        RegKey::predef(hive(machine)).open_subkey_with_flags(path, KEY_QUERY_VALUE | KEY_READ)
    else {
        return HashMap::new();
    };

    let mut states = HashMap::new();
    for (name, value) in values_of(&key) {
        // An unrecognised state still counts; dropping it would enable the entry.
        if let Some(state) = value.bytes.first().copied() {
            states.insert(name.to_lowercase(), state);
        }
    }
    states
}

pub(super) fn write_approved(machine: bool, cell: &str, name: &str, enabled: bool) -> Result<()> {
    let path = format!(r"{APPROVED_KEY}\{cell}");
    let (key, _) = RegKey::predef(hive(machine))
        .create_subkey_with_flags(path, KEY_QUERY_VALUE | KEY_SET_VALUE)
        .map_err(|e| registry_error("cannot open StartupApproved", e))?;

    let blob = approved_blob(enabled);
    if let Some(current) = values_of(&key)
        .into_iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
    {
        if current.bytes.len() >= 4 && blob[..4] == current.bytes[..4] {
            return Ok(());
        }
    }

    // Windows ignores a REG_SZ here and silently keeps the entry running.
    key.set_raw_value(
        name,
        &RegValue {
            bytes: blob,
            vtype: REG_BINARY,
        },
    )
    .map_err(|e| registry_error("cannot write the startup state", e))
}

/// State, three reserved, and a FILETIME stamp of the change.
fn approved_blob(enabled: bool) -> Vec<u8> {
    let mut blob = vec![0u8; 12];
    blob[0] = if enabled {
        STATE_ENABLED
    } else {
        STATE_DISABLED
    };
    if !enabled {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_nanos() as u64 / 100)
            .unwrap_or_default();
        blob[4..12].copy_from_slice(&stamp.to_le_bytes());
    }
    blob
}

fn hive(machine: bool) -> winreg::HKEY {
    if machine {
        HKEY_LOCAL_MACHINE
    } else {
        HKEY_CURRENT_USER
    }
}

pub(super) fn open_run_key(source: &RegistrySource) -> Result<RegKey> {
    RegKey::predef(hive(source.machine))
        .open_subkey_with_flags(source.run_key, KEY_QUERY_VALUE | KEY_READ)
        .map_err(|e| registry_error("cannot open the startup key", e))
}

pub(super) fn values_of(key: &RegKey) -> Vec<(String, RegValue)> {
    key.enum_values().filter_map(|item| item.ok()).collect()
}

pub(super) fn decode(value: &RegValue) -> Option<String> {
    if !matches!(value.vtype, REG_SZ | REG_EXPAND_SZ) {
        return None;
    }
    let pairs = value.bytes.as_chunks::<2>().0;
    let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    let text = String::from_utf16(&units).ok()?;
    let trimmed = text.trim_end_matches('\0').trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn registry_error(action: &str, error: std::io::Error) -> AppError {
    AppError::Other(format!("{action}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_record_means_the_user_never_touched_it() {
        assert!(is_enabled(&HashMap::new(), "Steam"));
    }

    #[test]
    fn a_disabled_record_switches_the_entry_off() {
        let map = HashMap::from([("steam".to_string(), STATE_DISABLED)]);
        assert!(!is_enabled(&map, "Steam"));
        assert!(!is_enabled(&map, "steam"));
        assert!(is_enabled(&map, "Discord"));
    }

    #[test]
    fn an_even_state_byte_reads_as_enabled_and_an_odd_one_as_disabled() {
        for state in [0x00u8, 0x02, 0x06, 0x0E] {
            let map = HashMap::from([("a".to_string(), state)]);
            assert!(is_enabled(&map, "A"), "{state:#04x} should read as enabled");
        }
        for state in [0x01u8, 0x03, 0x07, 0x99] {
            let map = HashMap::from([("a".to_string(), state)]);
            assert!(
                !is_enabled(&map, "A"),
                "{state:#04x} should read as disabled"
            );
        }
    }

    #[test]
    fn the_windows_11_state_is_not_read_as_enabled() {
        let map = HashMap::from([("steam".to_string(), 0x07)]);
        assert!(!is_enabled(&map, "Steam"));
    }

    #[test]
    fn a_value_with_no_state_byte_is_read_as_no_record() {
        let map = HashMap::from([("empty".to_string(), 0x00)]);
        assert!(is_enabled(&map, "Empty"));
    }

    #[test]
    fn enabling_writes_state_and_leaves_the_stamp_clear() {
        let bytes = approved_blob(true);
        assert_eq!(bytes[0], STATE_ENABLED);
        assert_eq!(bytes.len(), 12);
        assert!(bytes[4..].iter().all(|b| *b == 0));
    }

    #[test]
    fn disabling_writes_state_and_stamps_the_change() {
        let bytes = approved_blob(false);
        assert_eq!(bytes[0], STATE_DISABLED);
        assert_eq!(bytes.len(), 12);
        assert!(bytes[4..].iter().any(|b| *b != 0));
    }

    #[test]
    fn the_state_byte_never_leaks_into_the_reserved_slots() {
        for enabled in [true, false] {
            assert_eq!(approved_blob(enabled)[1..4], [0, 0, 0]);
        }
    }

    #[test]
    fn the_32_bit_view_reads_its_state_from_run32() {
        let source = REGISTRY_SOURCES
            .iter()
            .find(|s| s.run_key == WOW_RUN_KEY)
            .expect("the 32-bit Run key is listed");
        assert_eq!(
            source.approved_cell, "Run32",
            "StartupApproved\\Run is a different key and Windows never reads it for these"
        );
    }

    #[test]
    fn a_32_bit_location_is_unique() {
        let cell = "Run32";
        let hits = REGISTRY_SOURCES
            .iter()
            .filter(|s| s.approved_cell == cell && s.run_key.contains("Wow6432Node"))
            .count();
        assert_eq!(hits, 1);
    }

    #[test]
    fn run_once_and_policy_sources_are_not_toggleable() {
        for source in REGISTRY_SOURCES
            .iter()
            .filter(|s| s.source == SOURCE_RUNONCE || s.source == SOURCE_POLICY_RUN)
        {
            assert!(
                !source.toggleable,
                "{} / {} must not claim a toggle Windows cannot honour",
                source.source, source.scope
            );
            assert!(
                !source.reason.is_empty(),
                "{} / {} needs a reason for the refusal",
                source.source,
                source.scope
            );
        }
    }

    #[test]
    fn plain_run_sources_stay_toggleable() {
        let toggleable = listed_sources().count();
        assert_eq!(toggleable, 3, "two Run keys and the 32-bit Run key");
        for source in listed_sources() {
            assert!(
                source.source == SOURCE_RUN || source.source == SOURCE_RUN32,
                "{} should not be listed",
                source.source
            );
        }
    }

    #[test]
    fn a_run_once_toggle_is_refused_instead_of_claimed_as_done() {
        let source = REGISTRY_SOURCES
            .iter()
            .find(|s| s.source == SOURCE_RUNONCE && s.scope == SCOPE_USER)
            .expect("the user RunOnce key is listed in the table");
        assert!(!source.toggleable);
        assert!(source.reason.contains("cannot be disabled"));
    }

    #[test]
    fn a_policy_toggle_is_refused_instead_of_claimed_as_done() {
        let source = REGISTRY_SOURCES
            .iter()
            .find(|s| s.source == SOURCE_POLICY_RUN)
            .expect("the policy key is listed in the table");
        assert!(!source.toggleable);
        assert!(source.reason.contains("policy"));
    }

    #[test]
    fn an_ordinary_location_is_named_with_its_hive_and_key() {
        let user = REGISTRY_SOURCES
            .iter()
            .find(|s| s.source == SOURCE_RUN && s.scope == SCOPE_USER)
            .expect("the user Run key is listed");
        assert_eq!(user.name(), format!("HKCU\\{RUN_KEY}"));

        let machine = REGISTRY_SOURCES
            .iter()
            .find(|s| s.source == SOURCE_RUN && s.scope == SCOPE_MACHINE)
            .expect("the machine Run key is listed");
        assert_eq!(machine.name(), format!("HKLM\\{RUN_KEY}"));
    }

    #[test]
    fn every_registry_location_is_unique() {
        for (index, first) in REGISTRY_SOURCES.iter().enumerate() {
            for second in REGISTRY_SOURCES.iter().skip(index + 1) {
                let same = first.source == second.source
                    && first.scope == second.scope
                    && first.run_key.eq_ignore_ascii_case(second.run_key)
                    && first.machine == second.machine;
                assert!(
                    !same,
                    "{} / {} is listed twice",
                    first.source, first.run_key
                );
            }
        }
    }

    #[test]
    fn the_listing_covers_both_hives_and_the_32_bit_view() {
        let listed: Vec<&str> = listed_sources().map(|source| source.run_key).collect();
        for expected in [RUN_KEY, WOW_RUN_KEY] {
            assert!(listed.contains(&expected), "{expected} is not listed");
        }
        assert!(listed_sources().any(|source| !source.machine));
        assert!(listed_sources().any(|source| source.machine));

        for hidden in [RUNONCE_KEY, POLICY_RUN_KEY, WOW_RUNONCE_KEY] {
            assert!(
                REGISTRY_SOURCES.iter().any(|s| s.run_key == hidden),
                "{hidden} should still be known to the table"
            );
            assert!(
                !listed.contains(&hidden),
                "{hidden} has no state Windows reads and must not be listed"
            );
        }
    }

    #[test]
    fn only_string_valued_entries_are_decoded() {
        assert_eq!(
            decode(&RegValue {
                bytes: to_bytes("%APPDATA%\\OneDrive.exe"),
                vtype: REG_EXPAND_SZ,
            }),
            Some("%APPDATA%\\OneDrive.exe".to_string())
        );
        assert_eq!(
            decode(&RegValue {
                bytes: to_bytes("plain.exe"),
                vtype: REG_SZ,
            }),
            Some("plain.exe".to_string())
        );
        assert_eq!(
            decode(&RegValue {
                bytes: vec![1, 0, 0, 0],
                vtype: REG_BINARY,
            }),
            None
        );
    }

    fn to_bytes(text: &str) -> Vec<u8> {
        let mut units: Vec<u16> = text.encode_utf16().collect();
        units.push(0);
        units.iter().flat_map(|unit| unit.to_le_bytes()).collect()
    }
}
