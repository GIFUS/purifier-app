use super::{env, CleanupMode, Danger, Locator};
use crate::cleanup::model::CategoryDef;

pub const APPLICATIONS: &[CategoryDef] = &[
    CategoryDef {
        id: "applications.npm-cache",
        group: "applications",
        label: "npm Cache",
        description: "Downloaded npm tarballs. Re-downloaded on the next install.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed("%LOCALAPPDATA%\\npm-cache")],
    },
    CategoryDef {
        id: "applications.yarn-cache",
        group: "applications",
        label: "Yarn Cache",
        description: "Downloaded Yarn packages.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed("%LOCALAPPDATA%\\Yarn\\Cache"),
            Locator::Join(&[Locator::Env(env::APPDATA)], "Yarn\\Cache"),
        ],
    },
    CategoryDef {
        id: "applications.pnpm-store",
        group: "applications",
        label: "pnpm Store",
        description: "Content-addressable package store used by pnpm.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed("%LOCALAPPDATA%\\pnpm\\store")],
    },
    CategoryDef {
        id: "applications.pip-cache",
        group: "applications",
        label: "pip Cache",
        description: "Downloaded Python wheels and sdists.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed("%LOCALAPPDATA%\\pip\\Cache"),
            Locator::Join(&[Locator::Env(env::USERPROFILE)], ".cache\\pip"),
        ],
    },
    CategoryDef {
        id: "applications.nuget",
        group: "applications",
        label: "NuGet Packages",
        description: "Restored .NET packages. Visual Studio re-restores them on the next build.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed("%USERPROFILE%\\.nuget\\packages"),
            Locator::Fixed("%LOCALAPPDATA%\\NuGet\\v3-cache"),
        ],
    },
    CategoryDef {
        id: "applications.gradle",
        group: "applications",
        label: "Gradle Caches",
        description: "Gradle dependency and build caches. The next Android build starts cold.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed("%USERPROFILE%\\.gradle\\caches"),
            Locator::Fixed("%USERPROFILE%\\.gradle\\daemon"),
        ],
    },
    CategoryDef {
        id: "applications.cargo",
        group: "applications",
        label: "Cargo Registry Cache",
        description: "Downloaded crates. Only the download cache, never `src` or `bin`.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed("%USERPROFILE%\\.cargo\\registry\\cache")],
    },
    CategoryDef {
        id: "applications.chocolatey",
        group: "applications",
        label: "Chocolatey Temp",
        description: "Package downloads left behind by Chocolatey.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Join(&[Locator::Env(env::PROGRAMDATA)], "chocolatey\\temp"),
            Locator::Join(&[Locator::Env(env::PROGRAMDATA)], "chocolatey\\cache-bad"),
            Locator::Join(
                &[Locator::Env(env::PROGRAMDATA)],
                "chocolatey\\cache-bad\\tools",
            ),
        ],
    },
    CategoryDef {
        id: "applications.discord-cache",
        group: "applications",
        label: "Discord Cache",
        description: "Cached files and temporary data left behind by Discord.",
        danger: Danger::Safe,
        mode: CleanupMode::RemoveDir,
        locators: &[
            Locator::Fixed("%APPDATA%\\discord\\Cache"),
            Locator::Fixed("%APPDATA%\\discord\\Code Cache"),
            Locator::Fixed("%APPDATA%\\discord\\GPUCache"),
        ],
    },
];
