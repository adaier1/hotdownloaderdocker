use crate::cmd::Commands;
use anyhow::Result;
use clap::Parser;
use std::process::exit;
use std::time::Instant;

mod buffered_decrypt;
mod cmd;

/// um_cli (rust ver.)
/// A cli-tool to unlock encrypted audio files.
#[derive(Parser)]
#[command(name = "um_cli")]
#[command(version = "0.1")]
pub struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,

    /// Be more verbose about what is going on.
    #[clap(long, short, action=clap::ArgAction::SetTrue, default_value_t=false)]
    verbose: bool,

    /// Preferred buffer size when reading file, in bytes.
    /// Default to 4MiB.
    #[clap(long, short = 'B', default_value_t=4*1024*1024)]
    buffer_size: usize,
}

fn run_command(cli: &Cli) -> Result<i32> {
    match &cli.command {
        Some(Commands::Joox(cmd)) => cmd.run(cli),
        Some(Commands::Kgm(cmd)) => cmd.run(cli),
        Some(Commands::Migu3D(cmd)) => cmd.run(cli),
        Some(Commands::Ncm(cmd)) => cmd.run(cli),
        Some(Commands::QMCv1(cmd)) => cmd.run(cli),
        Some(Commands::QMCv2(cmd)) => cmd.run(cli),
        Some(Commands::QtFM(cmd)) => cmd.run(cli),
        Some(Commands::Xiami(cmd)) => cmd.run(cli),
        Some(Commands::Xmly(cmd)) => cmd.run(cli),
        None => {
            // https://github.com/clap-rs/clap/issues/3857#issuecomment-1161796261
            todo!("implement a sensible default command, similar to um/cli");
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let start = Instant::now();
    let code = run_command(&cli).unwrap_or_else(|err| {
        eprintln!("failed to run command: {}", err);
        -1
    });
    let duration = start.elapsed();
    if cli.verbose {
        eprintln!("time: {:?}", duration);
    };
    exit(code);
}
