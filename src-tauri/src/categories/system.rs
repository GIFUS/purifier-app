use super::{fixed, CleanupMode, Danger, Locator};
use crate::model::CategoryDef;

pub const SYSTEM: &[CategoryDef] = &[
    CategoryDef {
        id: "system.temp",
        group: "system",
        label: "Windows Temp",
        description: "Temporary files created by the installer and by system services.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::WINDOWS_TEMP)],
    },
    CategoryDef {
        id: "system.software-distribution",
        group: "system",
        label: "Windows Update Downloads",
        description: "Downloaded update packages left behind by Windows Update.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::SOFTWARE_DISTRIBUTION)],
    },
    CategoryDef {
        id: "system.delivery-optimization-cache",
        group: "system",
        label: "Delivery Optimization Cache",
        description: "Peer-to-peer update cache. Windows re-downloads what it needs.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::DELIVERY_OPTIMIZATION_CACHE)],
    },
    CategoryDef {
        id: "system.prefetch",
        group: "system",
        label: "Prefetch",
        description:
            "Windows application launch cache. Clearing it slows the next start of each app.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::PREFETCH)],
    },
    CategoryDef {
        id: "system.cbs-logs",
        group: "system",
        label: "Component Store Logs",
        description: "Servicing and system restore logs. Useful when diagnosing a broken install.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed(fixed::CBS_LOGS),
            Locator::Fixed(fixed::WINDOWS_LOGS),
        ],
    },
    CategoryDef {
        id: "system.wu-errors",
        group: "system",
        label: "Windows Error Reports",
        description: "Queued and archived crash reports waiting to be sent to Microsoft.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed(fixed::WER_REPORT_QUEUE),
            Locator::Fixed(fixed::WER_REPORT_ARCHIVE),
        ],
    },
    CategoryDef {
        id: "system.diagnostics",
        group: "system",
        label: "Diagnostic Traces",
        description: "ETL traces and network cache written by Windows diagnostics.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::DIAG_ETL)],
    },
    CategoryDef {
        id: "system.crash-dumps",
        group: "system",
        label: "System Crash Dumps",
        description: "Memory dumps produced by a blue screen or a hung system.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed(fixed::MINIDUMP),
            Locator::Fixed(fixed::MEMORY_DMP),
        ],
    },
];
