use crate::Cli;
use anyhow::{bail, Result};
use clap::Args;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use umc_qtfm::nonce::make_decipher_iv;
use umc_qtfm::Decipher;

/// Decrypt a qta file (QingTingFM)
#[derive(Args)]
pub struct ArgsQingTingFM {
    /// Path to output file, e.g. /export/Music/song.flac
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.xm
    #[arg(name = "input")]
    input: PathBuf,

    /// Device info (CSV), in the order of "product,device,manufacturer,brand,board,model".
    #[clap(short, long = "device")]
    device: Option<String>,

    /// Device key, in hex.
    #[clap(short = 'k', long = "device-key")]
    device_key: Option<String>,

    /// Override name. default to file name.
    #[clap(short, long)]
    name: Option<String>,
}

impl ArgsQingTingFM {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let mut reader = BufReader::with_capacity(cli.buffer_size, File::open(&self.input)?);
        let mut writer = BufWriter::with_capacity(cli.buffer_size, File::create(&self.output)?);

        let file_name = match &self.name {
            Some(x) => x.clone(),
            None => match self.input.file_name() {
                Some(x) => x.to_string_lossy().to_string(),
                None => bail!("failed to get file name"),
            },
        };

        let device_key = self.make_device_key()?;
        let iv = make_decipher_iv(&file_name)?;
        if cli.verbose {
            eprintln!(" file_name: {}", file_name);
            eprintln!("device_key: {}", hex::encode(device_key));
            eprintln!("   file_iv: {}", hex::encode(iv));
        }

        let decipher = Decipher::new(&device_key, &iv);

        let mut offset = 0;
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

    fn make_device_key(&self) -> Result<[u8; 0x10]> {
        let key = match &self.device_key {
            Some(key) => {
                let mut result = [0u8; 0x10];
                hex::decode_to_slice(key, &mut result)?;
                result
            }
            None => match &self.device {
                Some(device) => {
                    let mut device_parts = vec![];
                    for item in device.split(',') {
                        device_parts.push(item);
                    }
                    if device_parts.len() != 6 {
                        bail!(
                            "device needs 6 parts. current: {} part(s)",
                            device_parts.len()
                        );
                    }
                    umc_qtfm::secret::make_device_secret(
                        device_parts[0],
                        device_parts[1],
                        device_parts[2],
                        device_parts[3],
                        device_parts[4],
                        device_parts[5],
                    )
                }
                None => bail!("one of device/device-key is required."),
            },
        };
        Ok(key)
    }
}
