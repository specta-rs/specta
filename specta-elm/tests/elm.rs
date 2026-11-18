#![allow(clippy::unwrap_used, dead_code, missing_docs)]

//! Integration tests for the Elm exporter.
//!
//! Every test exports into a throwaway Elm project and snapshots the generated
//! source. When a compiler is available the generated module is also compiled,
//! which is the only way to be sure it is valid Elm: set `SPECTA_ELM` to the
//! compiler executable to make a missing compiler a hard failure instead of a
//! silent skip.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use specta::{Type, Types};
use specta_elm::{Elm, IntoExporter, exporter};

/// The Elm version the generated `elm.json` targets.
const ELM_VERSION: &str = "0.19.3";

/// A throwaway Elm project to export into.
struct TestProject {
    dir: tempfile::TempDir,
}

impl TestProject {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
        fs::create_dir_all(&root).unwrap();
        let dir = tempfile::TempDir::new_in(root).unwrap();

        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("elm.json"),
            format!(
                r#"{{
    "type": "application",
    "source-directories": ["src"],
    "elm-version": "{ELM_VERSION}",
    "dependencies": {{
        "direct": {{ "elm/core": "1.0.5", "elm/json": "1.1.4" }},
        "indirect": {{}}
    }},
    "test-dependencies": {{ "direct": {{}}, "indirect": {{}} }}
}}"#
            ),
        )
        .unwrap();

        Self { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn init(&self, types: Types) -> Elm {
        Elm::init(types, self.path()).unwrap()
    }

    /// Export, returning the generated sources keyed by their path in the project.
    fn export<O: IntoExporter>(&self, types: Types, output: O) -> BTreeMap<PathBuf, String> {
        self.init(types).export(output).unwrap();
        self.generated_sources()
    }

    fn generated_sources(&self) -> BTreeMap<PathBuf, String> {
        fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(root, &path, files);
                } else if path.extension().is_some_and(|ext| ext == "elm") {
                    let name = path.strip_prefix(root).unwrap().to_string_lossy();
                    files.insert(
                        PathBuf::from(name.into_owned()),
                        fs::read_to_string(&path).unwrap(),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        walk(self.path(), &self.path().join("src"), &mut files);
        files
    }

    /// Compile the generated modules with Elm, if a compiler is available.
    fn assert_compiles(&self) {
        let Some(elm) = elm_compiler() else {
            return;
        };

        for path in self.generated_sources().keys() {
            let output = Command::new(&elm)
                .current_dir(self.path())
                .arg("make")
                .arg(path)
                .arg("--output=/dev/null")
                .output()
                .expect("failed to run the Elm compiler");

            assert!(
                output.status.success(),
                "the Elm compiler rejected {}:\n{}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

/// The Elm compiler to use, if any.
///
/// `SPECTA_ELM` pins the executable (and makes its absence a failure);
/// otherwise `elm` on `PATH` is used when present so local runs still check the
/// output.
fn elm_compiler() -> Option<String> {
    if let Ok(elm) = std::env::var("SPECTA_ELM") {
        return Some(elm);
    }

    Command::new("elm")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "elm".to_string())
}

fn render(files: &BTreeMap<PathBuf, String>) -> String {
    files
        .iter()
        .map(|(path, content)| format!("// {}\n{content}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Type)]
struct Surname(String, String);

#[derive(Type)]
enum SurnameKind {
    Spanish(String, String),
    Other(String),
}

#[derive(Type)]
struct User {
    id: u32,
    name: String,
    surname: Option<SurnameKind>,
    email: Option<String>,
    role: UserRole,
}

#[derive(Type)]
enum UserRole {
    Guest,
    Member { permissions: Vec<String> },
    Admin { level: u8, department: String },
}

#[derive(Type)]
enum ApiResult {
    Success { data: String, status: u16 },
    Error { message: String, code: u32 },
    Loading { progress: f32 },
}

#[test]
fn readme_example() {
    let project = TestProject::new();
    let types = Types::default()
        .register::<User>()
        .register::<ApiResult>()
        .register::<Surname>();

    let files = project.export(types, exporter::SingleFile("Types"));
    insta::assert_snapshot!(render(&files));
    project.assert_compiles();
}

#[derive(Type)]
struct Collections {
    list: Vec<String>,
    set: HashSet<i32>,
    map: HashMap<String, Vec<Option<u8>>>,
    pair: (String, bool),
    triple: (u8, f64, String),
    nested: Vec<Vec<Option<String>>>,
}

#[test]
fn collections_are_parenthesised_correctly() {
    let project = TestProject::new();
    let files = project.export(
        Types::default().register::<Collections>(),
        exporter::SingleFile("Types"),
    );

    insta::assert_snapshot!(render(&files));
    project.assert_compiles();
}

#[derive(Type)]
struct OptionalFields {
    required: String,
    #[specta(optional)]
    optional: Option<String>,
}

#[derive(Type)]
struct Newtype(String);

#[derive(Type)]
struct TupleStruct(u8, String);

#[derive(Type)]
struct Empty;

#[derive(Type)]
enum Skipped {
    Kept,
    #[specta(skip)]
    Dropped,
}

#[test]
fn structs_and_skipped_variants() {
    let project = TestProject::new();
    let files = project.export(
        Types::default()
            .register::<OptionalFields>()
            .register::<Newtype>()
            .register::<TupleStruct>()
            .register::<Empty>()
            .register::<Skipped>(),
        exporter::SingleFile("Types"),
    );

    insta::assert_snapshot!(render(&files));
    project.assert_compiles();
}

#[test]
fn msg_file_is_written_to_the_expected_module() {
    let project = TestProject::new();
    let files = project.export(Types::default().register::<User>(), exporter::MsgFile);

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [&PathBuf::from("src/Msg.elm")]
    );
    project.assert_compiles();
}

#[test]
fn nested_modules_are_named_after_their_path() {
    let project = TestProject::new();
    let files = project.export(
        Types::default().register::<User>(),
        exporter::SingleFile("Api/Types"),
    );

    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        [&PathBuf::from("src/Api/Types.elm")]
    );
    assert!(render(&files).starts_with("// src/Api/Types.elm\nmodule Api.Types exposing (..)"));
    project.assert_compiles();
}

#[derive(Type)]
struct Generic<T> {
    value: T,
}

#[derive(Type)]
struct TooWideTuple(String, String, String, String);

#[derive(Type)]
enum NoVariants {
    #[specta(skip)]
    Only,
}

#[derive(Type)]
struct BigInt(i64);

#[derive(Type)]
struct LowerCased {
    value: String,
}

#[test]
fn unsupported_types_return_errors() {
    let project = TestProject::new();

    let generic = project
        .init(Types::default().register::<Generic<String>>())
        .export(exporter::SingleFile("Types"));
    assert!(generic.unwrap_err().to_string().contains("generic"));

    let tuple = project
        .init(Types::default().register::<TooWideTuple>())
        .export(exporter::SingleFile("Types"));
    assert!(tuple.unwrap_err().to_string().contains("tuples"));

    let empty = project
        .init(Types::default().register::<NoVariants>())
        .export(exporter::SingleFile("Types"));
    assert!(
        empty
            .unwrap_err()
            .to_string()
            .contains("at least one variant")
    );

    let big_int = project
        .init(Types::default().register::<BigInt>())
        .export(exporter::SingleFile("Types"));
    assert!(big_int.unwrap_err().to_string().contains("`Int`"));

    // Elm module names must be capitalised.
    let module = project
        .init(Types::default().register::<LowerCased>())
        .export(exporter::SingleFile("types"));
    assert!(module.unwrap_err().to_string().contains("module name"));
}

#[test]
fn missing_elm_json_is_an_error() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    fs::create_dir_all(&dir).unwrap();
    let dir = tempfile::TempDir::new_in(dir).unwrap();

    let error = Elm::init(Types::default(), dir.path()).unwrap_err();
    assert!(error.to_string().contains("elm.json"));
}
