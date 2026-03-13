use std::env::args_os;
use std::fs::hard_link;
use std::path::{Path, PathBuf};

use clap::Args;

use crate::error::Error;

/// Install hardlinks to bundled commands
#[derive(Args, Debug)]
pub struct Cmd {
    /// Directory to install hardlinks into
    dir: PathBuf,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let Some(prog) = args_os().next() else {
            return Err(Error::Executable);
        };
        let ext = Path::new(&prog).extension().unwrap_or_default();

        for subcmd in ["sdat2img", "img2sdat"] {
            let mut target = self.dir.join(subcmd);
            target.set_extension(ext);
            hard_link(&prog, &target).map_err(|e| Error::Io(target, e))?;
        }

        Ok(())
    }
}
