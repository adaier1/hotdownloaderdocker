use crate::Cli;
use anyhow::Result;
use clap::Args;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

/// Decrypt a XM file (Xiami)
#[derive(Args)]
pub struct ArgsXiami {
    /// Path to output file, e.g. /export/Music/song.flac
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.xm
    #[arg(name = "input")]
    input: PathBuf,
}

impl ArgsXiami {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let mut reader = BufReader::with_capacity(cli.buffer_size, File::open(&self.input)?);
        let mut writer = BufWriter::with_capacity(cli.buffer_size, File::create(&self.output)?);

        let mut header = [0u8; 0x10];
        reader.read_exact(&mut header)?;
        let xm = umc_xiami::XiamiFile::from_header(&header)?;
        let mut copy_reader = (&mut reader).take(xm.copy_len as u64);
        io::copy(&mut copy_reader, &mut writer)?;

        let mut buffer = vec![0u8; cli.buffer_size];
        loop {
            let n = reader.read(&mut buffer[..])?;
            if n == 0 {
                break;
            }
            xm.decrypt(&mut buffer[..n]);
            writer.write_all(&buffer[..n])?;
        }

        Ok(0)
    }
}
