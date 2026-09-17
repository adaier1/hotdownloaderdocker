use crate::buffered_decrypt::buffered_decrypt;
use crate::Cli;
use clap::Args;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use umc_ncm::header::NCMFile;

/// Decrypt a NCM file (NetEase Cloud Music)
#[derive(Args)]
pub struct ArgsNCM {
    /// Path to output file, e.g. /export/Music/song.flac
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Path to input file, e.g. /export/Music/song.ncm
    #[arg(name = "input")]
    input: PathBuf,

    /// Path to export cover image. Could be either JPG or PNG.
    /// File will not be created if not found.
    #[arg(short)]
    cover: Option<PathBuf>,

    /// Path to export metadata. JSON string.
    #[arg(short)]
    metadata: Option<PathBuf>,
}

impl ArgsNCM {
    fn write_metadata(&self, cli: &Cli, ncm: &NCMFile) -> anyhow::Result<()> {
        if let Some(metadata_path) = &self.metadata {
            if !ncm.metadata.is_empty() {
                let metadata = ncm.get_metadata()?;
                File::create(metadata_path)?.write_all(&metadata)?;
                if cli.verbose {
                    let metadata_path = metadata_path.display();
                    let len = metadata.len();
                    println!("metadata written to {metadata_path} ({len} bytes)");
                }
            } else {
                println!("metadata is empty, skip.");
            }
        }
        Ok(())
    }

    fn write_cover(&self, cli: &Cli, ncm: &NCMFile) -> anyhow::Result<()> {
        if let Some(cover_path) = &self.cover {
            if let Some(cover) = &ncm.image1 {
                File::create(cover_path)?.write_all(cover)?;
                if cli.verbose {
                    let cover_path = cover_path.display();
                    let len = cover.len();
                    println!("cover written to {cover_path} ({len} bytes)");
                }
            } else {
                println!("cover#1 is empty, skip.");
            }
        }
        Ok(())
    }

    pub fn run(&self, cli: &Cli) -> anyhow::Result<i32> {
        let mut file_input = File::open(&self.input)?;
        let ncm = NCMFile::new_from_readable(&mut file_input)?;

        self.write_metadata(cli, &ncm)?;
        self.write_cover(cli, &ncm)?;

        if let Some(output_path) = &self.output {
            file_input.seek(SeekFrom::Start(ncm.audio_data_offset as u64))?;
            let mut file_output = File::create(output_path)?;

            buffered_decrypt(
                &mut file_input,
                &mut file_output,
                cli.buffer_size,
                |buffer, offset| {
                    ncm.decrypt(buffer, offset);
                },
            )?;
        }

        Ok(0)
    }
}
