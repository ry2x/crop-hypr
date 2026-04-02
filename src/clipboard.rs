use std::path::Path;

use crate::cmd::{CMD_WL_COPY, run_cmd_status_with_stdin};
use crate::error::{AppError, Result};

pub fn copy_to_clipboard(path: &Path) -> Result<()> {
    let file =
        std::fs::File::open(path).map_err(|e| AppError::FileSystem(path.to_path_buf(), e))?;

    run_cmd_status_with_stdin(CMD_WL_COPY, ["--type", "image/png"], file)?;

    Ok(())
}
