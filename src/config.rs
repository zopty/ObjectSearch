use std::path::PathBuf;

use aviutl2::{AnyResult, anyhow};

pub fn get_current_dir() -> AnyResult<PathBuf> {
    let mut current_dir = std::env::current_exe()?;
    current_dir.pop();
    Ok(current_dir)
}

pub fn get_ini_path() -> AnyResult<PathBuf> {
    let current_dir = get_current_dir()?;
    let candidate_paths = [
        current_dir.join("data\\aviutl2.ini"),
        PathBuf::from("C:\\ProgramData\\aviutl2\\aviutl2.ini"),
        current_dir.join("aviutl2.ini"),
    ];
    candidate_paths
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| anyhow::anyhow!("cannot find aviutl2.ini"))
}
