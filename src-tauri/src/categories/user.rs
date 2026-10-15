use super::{env, fixed, CleanupMode, Danger, Locator};
use crate::model::CategoryDef;

pub const USER: &[CategoryDef] = &[
    CategoryDef {
        id: "user.temp",
        group: "user",
        label: "User Temp",
        description: "Temporary files created by the applications you run.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Env(env::TEMP)],
    },
    CategoryDef {
        id: "user.thumbnail-cache",
        group: "user",
        label: "Thumbnail Cache",
        description: "Previews of pictures and videos. Explorer rebuilds them on demand.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Pattern {
                base: env::LOCAL_APPDATA,
                pattern: fixed::THUMBNAIL_PATTERNS,
            },
            Locator::Join(&[Locator::Env(env::LOCAL_APPDATA)], fixed::ICON_CACHE),
        ],
    },
    CategoryDef {
        id: "user.graphics-cache",
        group: "user",
        label: "Graphics Shader Cache",
        description: "Compiled DirectX shaders. The first launch of each game runs slower once.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[
            Locator::Fixed(fixed::D3DSCACHE),
            Locator::Fixed(fixed::NVIDIA_DX_CACHE),
            Locator::Fixed(fixed::NVIDIA_GL_CACHE),
            Locator::Fixed(fixed::AMD_DX_CACHE),
            Locator::Fixed(fixed::AMD_GL_CACHE),
            Locator::Fixed(fixed::INTEL_SHADER_CACHE),
        ],
    },
    CategoryDef {
        id: "user.inet-cache",
        group: "user",
        label: "Internet Cache",
        description: "Copies of downloaded web content kept by the shell.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::INET_PROPERTIES_CACHE)],
    },
    CategoryDef {
        id: "user.crash-dumps",
        group: "user",
        label: "Application Crash Dumps",
        description: "Crash dumps of the applications you run, used for debugging.",
        danger: Danger::Safe,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::CRASH_DUMPS)],
    },
    CategoryDef {
        id: "user.explorer-cache",
        group: "user",
        label: "Explorer Cache",
        description: "Folder and icon cache entries. Desktop icons rebuild once.",
        danger: Danger::Caution,
        mode: CleanupMode::ClearContents,
        locators: &[Locator::Fixed(fixed::EXPLORER_CACHE)],
    },
];
