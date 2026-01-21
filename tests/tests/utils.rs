use std::{
    fs,
    path::{Path, PathBuf},
};

/// Get a `String` representation of the filesystem.
/// This is used for snapshot testing multi-file exports.
pub fn fs_to_string(path: &Path) -> Result<String, std::io::Error> {
    let mut output = String::new();

    // Handle single file case
    if path.is_file() {
        let contents = fs::read(path)?;
        let size = contents.len();
        let name = path.file_name().unwrap().to_string_lossy();

        output.push_str(&format!("{} ({} bytes)\n", name, size));
        output.push_str("────────────────────────────────────────\n");

        match String::from_utf8(contents) {
            Ok(text) => {
                for line in text.lines() {
                    output.push_str(&format!("{}\n", line));
                }
            }
            Err(_) => {
                output.push_str("[Binary file]\n");
            }
        }

        output.push_str("════════════════════════════════════════\n");
    } else {
        fs_to_string_impl(path, path, &mut output, "")?;
    }

    Ok(output)
}

fn fs_to_string_impl(
    root: &Path,
    current: &Path,
    output: &mut String,
    indent: &str,
) -> Result<(), std::io::Error> {
    let mut entries: Vec<PathBuf> = fs::read_dir(current)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();

    // Sort entries: directories first, then files, both alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let name = entry.file_name().unwrap().to_string_lossy();

        if entry.is_dir() {
            output.push_str(&format!("{}{}/\n", indent, name));
            fs_to_string_impl(root, &entry, output, &format!("{}  ", indent))?;
        } else {
            let contents = fs::read(&entry)?;
            let size = contents.len();

            output.push_str(&format!("{}{} ({} bytes)\n", indent, name, size));
            output.push_str(&format!(
                "{}────────────────────────────────────────\n",
                indent
            ));

            // Try to read as UTF-8, otherwise show as binary
            match String::from_utf8(contents) {
                Ok(text) => {
                    for line in text.lines() {
                        output.push_str(&format!("{}{}\n", indent, line));
                    }
                }
                Err(_) => {
                    output.push_str(&format!("{}[Binary file]\n", indent));
                }
            }

            output.push_str(&format!(
                "{}════════════════════════════════════════\n\n",
                indent
            ));
        }
    }

    Ok(())
}
