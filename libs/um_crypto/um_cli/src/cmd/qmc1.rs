use crate::buffered_decrypt::buffered_decrypt;
use crate::Cli;
use anyhow::Result;
use clap::Args;
use std::fs::File;
use std::path::PathBuf;

/// Decrypt a QMCv1 file (QQMusic)
#[derive(Args)]
pub struct ArgsQMCv1 {
    /// Path to output file, e.g. /export/Music/song.flac
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.qmcflac
    #[arg(name = "input")]
    input: PathBuf,
}

impl ArgsQMCv1 {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let mut file_input = File::open(&self.input)?;
        let mut file_output = File::create(&self.output)?;

        buffered_decrypt(
            &mut file_input,
            &mut file_output,
            cli.buffer_size,
            umc_qmc::v1::decrypt,
        )?;

        Ok(0)
    }
}
