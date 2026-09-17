use crate::Cli;
use anyhow::Result;
use clap::Args;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use umc_joox::decrypt::JooxDecipher;
use umc_joox::header::Header;

/// Decrypt a ofl_en file (Joox Music, Encryption V4)
#[derive(Args)]
pub struct ArgsJoox {
    /// Path to output file, e.g. /export/Music/song.flac
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.ofl_en
    #[arg(name = "input")]
    input: PathBuf,

    /// Device GUID
    #[clap(short = 'u', long)]
    device_guid: String,
}

impl ArgsJoox {
    pub fn run(&self, _cli: &Cli) -> Result<i32> {
        let mut file_input = File::open(&self.input)?;

        let mut header_buffer = [0u8; 0x0c];
        file_input.read_exact(&mut header_buffer)?;
        let header = Header::from_buffer(&header_buffer, &self.device_guid)?;
        file_input.seek(SeekFrom::Start(header.audio_start_offset as u64))?;

        let reader = BufReader::new(file_input);
        let mut input_stream = reader.take(header.original_file_len);

        let file_output = File::create(&self.output)?;
        let mut writer = BufWriter::new(file_output);

        let mut buffer = vec![0u8; header.get_audio_block_size()].into_boxed_slice();
        while let Ok(()) = input_stream.read_exact(&mut buffer) {
            let result = header.decrypt_audio_block(&mut buffer)?;
            writer.write_all(result)?;
        }

        Ok(0)
    }
}
