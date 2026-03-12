use crate::error::Error;
use clap::Args;
use std::{env::args_os, fs::hard_link, path::PathBuf};

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

        for subcmd in ["sdat2img", "img2sdat"] {
            let target = self.dir.join(subcmd);
            hard_link(&prog, &target).map_err(|e| Error::Io(target, e))?;
        }

        Ok(())
    }
}
