use std::{fs, process::Command};

use tempfile::Builder;

#[test]
fn transitive_specta_serde_enables_macro_metadata() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests crate should live inside workspace root");
    let tempdir = Builder::new()
        .prefix("specta-transitive-serde-")
        .tempdir_in(workspace_root)
        .expect("failed to create temp workspace");
    let root = tempdir.path();
    let framework_dir = root.join("framework");
    let app_dir = root.join("app");
    fs::create_dir_all(framework_dir.join("src")).expect("failed to create framework src");
    fs::create_dir_all(app_dir.join("src")).expect("failed to create app src");
    fs::create_dir_all(app_dir.join("tests")).expect("failed to create app tests");

    let specta_path = workspace_root.join("specta").display().to_string();
    let specta_serde_path = workspace_root.join("specta-serde").display().to_string();
    let specta_typescript_path = workspace_root
        .join("specta-typescript")
        .display()
        .to_string();

    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"framework\", \"app\"]\nresolver = \"2\"\n",
    )
    .expect("failed to write workspace manifest");

    fs::write(
        framework_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"framework\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nspecta = {{ path = \"{specta_path}\" }}\nspecta-serde = {{ path = \"{specta_serde_path}\" }}\nspecta-typescript = {{ path = \"{specta_typescript_path}\" }}\n"
        ),
    )
    .expect("failed to write framework manifest");
    fs::write(
        framework_dir.join("src/lib.rs"),
        r#"
pub fn export<T: specta::Type>() -> String {
    let types = specta::Types::default().register::<T>();
    let resolved = specta_serde::apply(types).expect("serde apply should succeed");
    specta_typescript::Typescript::default()
        .export(&resolved)
        .expect("typescript export should succeed")
}
"#,
    )
    .expect("failed to write framework source");

    fs::write(
        app_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nframework = {{ path = \"../framework\" }}\nspecta = {{ path = \"{specta_path}\", features = [\"derive\"] }}\nserde = {{ version = \"1\", features = [\"derive\"] }}\n"
        ),
    )
    .expect("failed to write app manifest");
    fs::write(
        app_dir.join("src/lib.rs"),
        r#"
use serde::{Deserialize, Serialize};

#[derive(specta::Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Demo {
    pub renamed_field: String,
}
"#,
    )
    .expect("failed to write app source");
    fs::write(
        app_dir.join("tests/runtime.rs"),
        r#"
#[test]
fn serde_metadata_flows_transitively() {
    let output = framework::export::<app::Demo>();
    assert!(output.contains("renamedField"), "unexpected output: {output}");
}
"#,
    )
    .expect("failed to write app test");

    let output = Command::new("cargo")
        .arg("test")
        .arg("-p")
        .arg("app")
        .current_dir(root)
        .output()
        .expect("failed to run cargo test for temp workspace");

    assert!(
        output.status.success(),
        "cargo test failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
