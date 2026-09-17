use crate::Cli;
use anyhow::{bail, Result};
use clap::Args;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, clap::ValueEnum)]
enum XimalayaType {
    /// Android: *.x2m
    X2M,
    /// Android: *.x3m
    X3M,
    /// PC: *.xm
    XM,
}

/// Decrypt a X2M/X3M/XM file (Ximalaya)
#[derive(Args)]
pub struct ArgsXimalaya {
    /// Path to output file, e.g. /export/Music/song.flac
    #[clap(short, long)]
    output: PathBuf,

    /// Path to input file, e.g. /export/Music/song.x3m
    #[arg(name = "input")]
    input: PathBuf,

    #[clap(short = 't', long = "type")]
    file_type: Option<XimalayaType>,
}

impl ArgsXimalaya {
    pub fn run(&self, cli: &Cli) -> Result<i32> {
        let file_type = match &self.file_type {
            Some(x) => x.clone(),
            None => match self.input.extension().and_then(|ext: &OsStr| ext.to_str()) {
                Some("x2m") => XimalayaType::X2M,
                Some("x3m") => XimalayaType::X3M,
                Some("xm") => XimalayaType::XM,
                Some(ext) => bail!("invalid ext: {ext}"),
                _ => bail!("ext not found"),
            },
        };
        let mut reader = BufReader::with_capacity(cli.buffer_size, File::open(&self.input)?);
        let mut writer = BufWriter::with_capacity(cli.buffer_size, File::create(&self.output)?);

        match file_type {
            XimalayaType::X2M | XimalayaType::X3M => {
                let android_type = match file_type {
                    XimalayaType::X2M => umc_xmly::android::FileType::X2M,
                    XimalayaType::X3M => umc_xmly::android::FileType::X3M,
                    _ => bail!("this should not happen"),
                };
                let mut header = [0u8; 0x400];
                reader.read_exact(&mut header)?;
                umc_xmly::android::decrypt_android(android_type, &mut header);
                writer.write_all(&header)?;
                io::copy(&mut reader, &mut writer)?;
            }
            XimalayaType::XM => {
                let mut header = vec![0u8; 1024];
                reader.read_exact(&mut header)?;
                let xm_file = match umc_xmly::pc::Header::from_buffer(&header) {
                    Ok(hdr) => hdr,
                    Err(umc_xmly::XmlyError::MetadataTooSmall(n)) => {
                        let old_size = header.len();
                        header.resize(n, 0);
                        reader.read_exact(&mut header[old_size..])?;
                        umc_xmly::pc::Header::from_buffer(&header)?
                    }
                    Err(err) => bail!("failed to parse file: {err}"),
                };

                // Copy header
                writer.write_all(xm_file.copy_m4a_header().as_slice())?;

                // Process encrypted data
                reader.seek(SeekFrom::Start(xm_file.data_start_offset as u64))?;
                let mut header = vec![0u8; xm_file.encrypted_header_size];
                reader.read_exact(&mut header[..])?;
                writer.write_all(xm_file.decrypt(&mut header[..])?)?;

                // Copy rest of the file
                io::copy(&mut reader, &mut writer)?;
            }
        }

        Ok(0)
    }
}
