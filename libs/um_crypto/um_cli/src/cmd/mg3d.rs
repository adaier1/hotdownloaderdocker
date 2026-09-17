use crate::Cli;
use anyhow::{bail, Result};
use clap::Args;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use umc_mg3d::{guess_key, Decipher};

/// Decrypt a mg3d file (Migu 3D Audio)
#[derive(Args)]
pub struct ArgsMigu3D {
    /// Path to output file, e.g. /export/Music/song.wav
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.mg3d
    #[arg(name = "input")]
    input: PathBuf,

    /// File key (androidFileKey/iosFileKey). Leave empty to guess the key.
    #[clap(short = 'k', long = "file-key")]
    file_key: Option<String>,
}

impl ArgsMigu3D {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let mut reader = BufReader::with_capacity(cli.buffer_size, File::open(&self.input)?);
        let mut writer = BufWriter::with_capacity(cli.buffer_size, File::create(&self.output)?);

        let decipher = self.make_decipher(&mut reader)?;
        if cli.verbose {
            println!("final key: {}", hex::encode(decipher.get_key()));
        }

        let mut offset = 0usize;
        let mut buffer = vec![0u8; cli.buffer_size];
        loop {
            let n = reader.read(&mut buffer[..])?;
            if n == 0 {
                break;
            }
            decipher.decrypt(&mut buffer[..n], offset);
            writer.write_all(&buffer[..n])?;
            offset += n;
        }

        Ok(0)
    }

    fn make_decipher<T>(&self, reader: &mut T) -> Result<Decipher>
    where
        T: Read + Seek + ?Sized,
    {
        let decipher = match &self.file_key {
            None => {
                let mut buffer = [0u8; 0x100];
                reader.read_exact(&mut buffer)?;
                reader.seek(SeekFrom::Current(-0x100))?;
                let key = match guess_key(&buffer) {
                    Some(key) => key,
                    None => bail!("failed to guess a valid key"),
                };
                Decipher::new_from_final_key(&key)?
            }
            Some(key) => Decipher::new_from_file_key(key)?,
        };

        Ok(decipher)
    }
}
