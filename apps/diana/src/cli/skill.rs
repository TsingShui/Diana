use std::io::Write;
use std::path::PathBuf;

use super::output::{Failure, publish_new};

const CONTENT: &[u8] = include_bytes!("../../SKILL.md");

pub(super) fn run(install: bool) -> Result<(), Failure> {
    if install {
        let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let home = std::env::var_os(variable)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| Failure {
                code: 1,
                message: format!(
                    "cannot determine home directory: set {variable} to an absolute path"
                ),
            })?;
        let directory = home.join(".agents/skills/diana");
        std::fs::create_dir_all(&directory).map_err(Failure::output)?;
        let target = directory.join("SKILL.md");
        publish_new(&target, CONTENT)?;
        eprintln!("Installed Diana skill to {}", target.display());
        return Ok(());
    }

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(CONTENT).map_err(Failure::output)?;
    stdout.flush().map_err(Failure::output)
}
