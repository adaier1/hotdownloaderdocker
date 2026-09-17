use crate::buffered_decrypt::buffered_decrypt;
use crate::Cli;
use clap::Args;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use umc_kgm::header::Header;
use umc_kgm::Decipher;

/// Decrypt a KGM/VPR file (Kugou Music)
#[derive(Args)]
pub struct ArgsKGM {
    /// Path to output file, e.g. /export/Music/song.flac
    #[arg(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.kgm
    #[arg(name = "input")]
    input: PathBuf,

    /// File mode, one of "kgm" or "db", default to "kgm"
    #[arg(short, long, default_value = "kgm")]
    file_mode: String,
}

impl ArgsKGM {
    pub fn run(&self, cli: &Cli) -> anyhow::Result<i32> {
        match self.file_mode.as_str() {
            "kgm" => self.decrypt_kgm_file(cli),
            "db" => self.decrypt_db_file(),
            _ => anyhow::bail!("Invalid file mode: {}", self.file_mode),
        }
    }

    fn decrypt_db_file(&self) -> anyhow::Result<i32> {
        let mut file_input = File::open(&self.input)?;

        let mut buffer = Vec::new();
        file_input.read_to_end(&mut buffer)?;
        umc_kgm::decrypt_db(&mut buffer)?;
        let mut file_output = File::create(&self.output)?;
        file_output.write_all(&buffer)?;

        Ok(0)
    }

    fn decrypt_kgm_file(&self, cli: &Cli) -> anyhow::Result<i32> {
        let mut file_input = File::open(&self.input)?;
        let mut header = [0u8; 0x40];
        file_input.read_exact(&mut header)?;
        let kgm_header = Header::from_buffer(header)?;
        let decipher = Decipher::new(&kgm_header)?;
        file_input.seek(SeekFrom::Start(kgm_header.offset_to_data as u64))?;

        let mut file_output = File::create(&self.output)?;
        buffered_decrypt(
            &mut file_input,
            &mut file_output,
            cli.buffer_size,
            |buffer, offset| {
                decipher.decrypt(buffer, offset);
            },
        )?;

        Ok(0)
    }
}
