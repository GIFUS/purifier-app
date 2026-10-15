fn main() {
    // Embed our own manifest so the app requests administrator rights. Tauri
    // does not expose this through `tauri.conf.json`, and the cleaner cannot
    // touch `%ProgramData%` or `C:\Windows\SoftwareDistribution` without it.
    //
    // `app_manifest` takes the manifest *content*, not a path. It replaces
    // Tauri's default manifest, so ours must keep the Common-Controls
    // dependency the WebView2 dialogs rely on.
    let manifest = std::fs::read_to_string("windows-app-manifest.xml")
        .expect("windows-app-manifest.xml must be readable from the crate root");

    let attributes = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new().app_manifest(manifest));

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
