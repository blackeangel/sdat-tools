use clap::{Parser, Subcommand};
use std::process::ExitCode;

mod error;
mod img2sdat;
mod sdat2img;
mod tlist;

/// Android block-based OTA tools
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    nested: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    #[command(name = "sdat2img")]
    Sdat2Img(sdat2img::Cmd),
    #[command(name = "img2sdat")]
    Img2Sdat(img2sdat::Cmd),
}

fn main() -> ExitCode {
    let command = Args::parse().nested;

    let result = match command {
        Cmd::Sdat2Img(mut cmd) => cmd.run(),
        Cmd::Img2Sdat(cmd) => cmd.run(),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
