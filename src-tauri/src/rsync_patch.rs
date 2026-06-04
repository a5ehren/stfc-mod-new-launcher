use crate::errors::{io_context, LauncherError, LauncherResult};
use librsync::whole::patch;
use std::fs;
use std::path::Path;

pub fn apply_rsync_patch(source: &Path, patch_file: &Path, output: &Path) -> LauncherResult<()> {
    let mut source_file = fs::File::open(source)
        .map_err(|err| io_context(format!("opening {}", source.display()), err))?;
    let mut patch_file = fs::File::open(patch_file)
        .map_err(|err| io_context(format!("opening {}", patch_file.display()), err))?;
    let mut output_file = fs::File::create(output)
        .map_err(|err| io_context(format!("creating {}", output.display()), err))?;

    patch(&mut source_file, &mut patch_file, &mut output_file)
        .map(|_| ())
        .map_err(|err| LauncherError::Operation {
            context: "applying rsync patch".into(),
            message: err.to_string(),
        })
}
