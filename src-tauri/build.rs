/// Common Controls v6 is required by wry/Tauri on Windows: it provides
/// `TaskDialogIndirect` and the other comctl32 v6 exports.
#[cfg(windows)]
const WINDOWS_APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#;

fn main() {
    #[cfg(windows)]
    {
        use std::{env, fs, path::Path};

        // tauri-build links its resource library (icon, version info, app
        // manifest) only into binary targets via `rustc-link-arg-bins`. Cargo
        // links library unit-test executables separately, so they would carry
        // no application manifest and Windows would load comctl32 v5, which
        // lacks `TaskDialogIndirect`: 0xC0000139 before a single test runs.
        //
        // This is the upstream standard fix (tauri-apps/winres#42 and
        // tauri-apps/tauri#16320): keep tauri-build for the icon/version
        // resources, and link a manifest-only resource into *every* artifact
        // with embed-resource's `compile_for_everything`, which emits
        // `cargo:rustc-link-arg` (all targets). Once tauri-build ships
        // `WindowsAttributes::link_resource_into_all_targets(true)`, this
        // block can be replaced by that single opt-in option.
        let out_dir =
            Path::new(&env::var("OUT_DIR").expect("OUT_DIR is set by Cargo")).to_path_buf();

        tauri_build::try_build(
            tauri_build::Attributes::new()
                .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest()),
        )
        .expect("failed to run tauri-build");

        let manifest_path = out_dir.join("common-controls.manifest");
        fs::write(&manifest_path, WINDOWS_APP_MANIFEST)
            .expect("failed to write the Windows application manifest");

        // RT_MANIFEST (24), CREATEPROCESS_MANIFEST_RESOURCE_ID (1). rc.exe
        // needs escaped separators in absolute paths.
        let rc_path = out_dir.join("common-controls.rc");
        let escaped = manifest_path.to_string_lossy().replace('\\', "\\\\");
        fs::write(&rc_path, format!("1 24 \"{}\"\n", escaped))
            .expect("failed to write the resource script");

        embed_resource::compile_for_everything(&rc_path, embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed the Windows application manifest");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}
