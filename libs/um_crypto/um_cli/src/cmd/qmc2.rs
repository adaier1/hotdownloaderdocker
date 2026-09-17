use crate::buffered_decrypt::buffered_decrypt;
use crate::Cli;
use anyhow::{bail, Result};
use clap::Args;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use umc_qmc::{footer, QMCv2Cipher};
use umc_utils::base64;

/// Decrypt a QMCv2 file (QQMusic)
#[derive(Args)]
pub struct ArgsQMCv2 {
    /// Path to output file, e.g. /export/Music/song.flac
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Path to input file, e.g. /export/Music/song.qmcflac
    #[arg(name = "input")]
    input: PathBuf,

    /// Override EKey for this file.
    /// Prefix with "decrypted:" to use base64 encoded raw key.
    /// Prefix with "@" to read ekey from external file
    #[arg(short = 'K', long = "ekey")]
    ekey: Option<String>,

    /// Print info about this file, and do not perform decryption.
    #[arg(short = 'I', long, action=clap::ArgAction::SetTrue, default_value_t=false)]
    info_only: bool,
}

fn read_ekey(ekey: &str) -> Result<Vec<u8>> {
    let mut external_file = false;
    let mut decrypt_ekey = true;

    let mut ekey = ekey;
    loop {
        if let Some(stripped) = ekey.strip_prefix("@") {
            ekey = stripped;
            external_file = true;
        } else if let Some(stripped) = ekey.strip_prefix("decrypted:") {
            ekey = stripped;
            decrypt_ekey = false;
        } else {
            break;
        }
    }

    let ekey = match external_file {
        true => fs::read_to_string(ekey)?,
        false => ekey.into(),
    };
    let ekey = ekey.trim();
    let ekey = match decrypt_ekey {
        true => umc_qmc::ekey::decrypt(ekey)?,
        false => base64::decode(ekey)?,
    };
    Ok(ekey)
}

impl ArgsQMCv2 {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let mut file_input = File::open(&self.input)?;
        let mut footer_detection_buffer = vec![0u8; footer::INITIAL_DETECTION_LEN];
        file_input.seek(SeekFrom::End(-(footer::INITIAL_DETECTION_LEN as i64)))?;
        file_input.read_exact(&mut footer_detection_buffer)?;
        let input_size = file_input.stream_position()?;
        file_input.seek(SeekFrom::Start(0))?;

        let (footer_len, ekey) = match footer::from_byte_slice(&footer_detection_buffer) {
            Ok(Some(metadata)) => {
                if self.info_only || cli.verbose {
                    println!("metadata: {:?}", metadata);
                }
                (metadata.size, metadata.ekey.or_else(|| self.ekey.clone()))
            }
            Ok(None) => {
                eprintln!("[warn] could not find any qmc metadata.");
                (0usize, self.ekey.clone())
            }
            Err(err) => {
                eprintln!("[warn] failed to parse qmc metadata: {}", err);
                (0usize, self.ekey.clone())
            }
        };

        if self.info_only {
            return Ok(0);
        }

        let key = match ekey {
            None => bail!("--ekey is required when embedded ekey is not present."),
            Some(ekey) => read_ekey(ekey.as_str())?,
        };
        let cipher = QMCv2Cipher::new(key)?;

        let mut file_output = match &self.output {
            None => bail!("--output is required"),
            Some(output) => File::create(output)?,
        };

        let reader = BufReader::with_capacity(cli.buffer_size, file_input);
        let mut reader = reader.take(input_size - footer_len as u64);
        buffered_decrypt(
            &mut reader,
            &mut file_output,
            cli.buffer_size,
            |buffer, offset| {
                cipher.decrypt(buffer, offset);
            },
        )?;

        Ok(0)
    }
}
