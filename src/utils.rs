use std::fs;
use std::io;
use std::path::Path;

pub fn ensure_directory_for_file(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the output path points to a directory",
        ));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(())
}
