use std::env::args_os;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod error;
mod img2sdat;
mod install;
mod sdat2img;
mod tlist;

/// Android block-based OTA tools
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    nested: Cmd,
}

#[derive(Parser, Debug)]
#[command(multicall = true)]
struct MulticallArgs {
    #[command(subcommand)]
    nested: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(name = "sdat2img")]
    Sdat2Img(sdat2img::Cmd),
    #[command(name = "img2sdat")]
    Img2Sdat(img2sdat::Cmd),
    #[command(name = "install")]
    Install(install::Cmd),
}

fn main() -> ExitCode {
    let command = {
        let prog = args_os().next().map(PathBuf::from);
        let is_multicall = prog.is_some_and(|a| a.file_prefix().is_some_and(|p| p != "sdat-tools"));

        if is_multicall {
            MulticallArgs::parse().nested
        } else {
            Args::parse().nested
        }
    };

    let result = match command {
        Cmd::Sdat2Img(mut cmd) => cmd.run(),
        Cmd::Img2Sdat(cmd) => cmd.run(),
        Cmd::Install(cmd) => cmd.run(),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
