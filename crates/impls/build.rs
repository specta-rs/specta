use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

fn main() {
    // TODO: Windows support
    let path = PathBuf::from("/tmp") // std::env::var("CARGO_TARGET_DIR").unwrap())
        .join("_specta");
    create_dir_all(&path).ok();
    let mut file = File::create(path.join("impls.rs")).unwrap();
    file.write_all(
        b"use crate::{Type, DataType, DefOpts};
        
    impl Type for specta_impls::Testing {
        fn inline(_: DefOpts, _: &[DataType]) -> DataType {
            DataType::Any
        }
    }",
    )
    .unwrap();

    // println!("{:?}", std::env::var("OUT_DIR"));
    // std::env::set_var("SPECTA_DEMO", "123");
    // println!("cargo:rustc-env=SPECTA_DEMO=123");
}
