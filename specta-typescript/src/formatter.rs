use std::{io, path::PathBuf, process::Command};

use crate::typescript::FormatterFn;

/// Format the specified file using [ESLint](https://eslint.org).
pub fn eslint(file: PathBuf) -> io::Result<()> {
    Command::new("eslint")
        .arg("--fix")
        .arg(file)
        .output()
        .map(|_| ())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// Assert that the function signature matches the expected type.
const _: FormatterFn = eslint;

/// Format the specified file using [Prettier](https://prettier.io).
pub fn prettier(file: PathBuf) -> io::Result<()> {
    Command::new("prettier")
        .arg("--write")
        .arg(file)
        .output()
        .map(|_| ())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// Assert that the function signature matches the expected type.
const _: FormatterFn = prettier;

/// Format the specified file using [Biome](https://prettier.io).
pub fn biome(file: PathBuf) -> io::Result<()> {
    Command::new("biome")
        .arg("format")
        .arg(file)
        .output()
        .map(|_| ())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

// Assert that the function signature matches the expected type.
const _: FormatterFn = biome;
