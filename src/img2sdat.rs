use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use brotlic::{BrotliEncoderOptions, CompressorWriter, Quality};
use clap::Args;

use crate::error::{Error, ProcessError, check_file_alignment, file_prefix};
use crate::tlist::{self, Writer as ListWriter};

/// Convert raw image file to .new.dat or .new.dat.br
#[derive(Args, Debug)]
pub struct Cmd {
    /// Input raw image file
    file: PathBuf,
    /// Directory for output files [default: <FILE directory>]
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Enable brotli compression at specified level [default: 5]
    #[arg(
        short,
        long,
        value_name = "LEVEL",
        default_missing_value = "5",
        num_args = 0..=1,
        require_equals = false, value_parser = clap::value_parser!(u8).range(0..=11),
    )]
    brotli: Option<u8>,
    /// Force overwrite output files
    #[arg(short, long)]
    force: bool,
    /// Transfer list format version
    #[arg(
        long,
        value_name = "FMT",
        default_value_t = 4,
        value_parser = clap::value_parser!(u8).range(1..=4),
    )]
    format: u8,
    /// Block size in bytes
    #[arg(long, value_name = "N", default_value_t = 4096)]
    block_size: u32,
    /// Read/write buffer size in KiB
    #[arg(long, value_name = "N", default_value_t = 256)]
    buffer_size: usize,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Error> {
        let input_len = check_file_alignment(&self.file, self.block_size)?;

        let (dat_path, tlist_path, patch_path) = self.output_paths()?;

        let mut input_reader = {
            let f = File::open(&self.file).map_err(|e| Error::Io(self.file.clone(), e))?;
            BufReader::with_capacity(self.buffer_size * 1024, f)
        };

        let create_func = if self.force {
            File::create
        } else {
            File::create_new
        };

        create_func(&patch_path).map_err(|e| Error::Io(patch_path.clone(), e))?;

        let mut dat_writer = {
            let f = create_func(&dat_path).map_err(|e| Error::Io(dat_path.clone(), e))?;
            BufWriter::with_capacity(self.buffer_size * 1024, f)
        };

        let mut tlist_writer = {
            let f = create_func(&tlist_path).map_err(|e| Error::Io(tlist_path.clone(), e))?;
            let mut w = BufWriter::new(f);

            let total_blocks = u32::try_from(input_len / u64::from(self.block_size))
                .expect("block count overflows u32");

            let header = tlist::Header {
                version: self.format,
                total_blocks,
                stash_entries: 0,
                max_stash_blocks: 0,
            };
            write!(w, "{header}").map_err(|e| Error::Io(tlist_path.clone(), e))?;

            ListWriter::new(w)
        };

        let result = if let Some(level) = self.brotli {
            let level = BrotliEncoderOptions::new()
                .quality(Quality::new(level).unwrap())
                .build()
                .unwrap();
            let mut dat_writer = CompressorWriter::with_encoder(level, &mut dat_writer);
            img2sdat(
                &mut input_reader,
                &mut dat_writer,
                &mut tlist_writer,
                self.block_size,
            )
        } else {
            img2sdat(
                &mut input_reader,
                &mut dat_writer,
                &mut tlist_writer,
                self.block_size,
            )
        };

        result.map_err(|e| match e {
            ProcessError::Read(e) => Error::Io(self.file.clone(), e),
            ProcessError::Write(e) => Error::Io(dat_path.clone(), e),
            ProcessError::TransferListWrite(tlist::WriteError::Io(e)) => {
                Error::Io(tlist_path.clone(), e)
            }
            ProcessError::TransferListWrite(_) | ProcessError::TransferListRead(_) => {
                unreachable!()
            }
        })?;

        let (f, ..) = dat_writer.into_parts();
        f.sync_all().map_err(|e| Error::Io(dat_path, e))?;

        let (w, ..) = tlist_writer.into_parts();
        let (f, ..) = w.into_parts();
        f.sync_all().map_err(|e| Error::Io(tlist_path, e))?;

        Ok(())
    }

    fn output_paths(&self) -> Result<(PathBuf, PathBuf, PathBuf), Error> {
        let prefix = file_prefix(&self.file)?;

        let base_path = match &self.output {
            Some(path) => path.join(prefix),
            None => self
                .file
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(prefix),
        };

        let tlist = base_path.with_extension("transfer.list");
        let patch = base_path.with_extension("patch.dat");

        let mut dat = base_path;
        dat.set_extension(self.brotli.map_or("new.dat", |_| "new.dat.br"));

        Ok((dat, tlist, patch))
    }
}

#[allow(unused_assignments)]
fn img2sdat(
    input_reader: &mut impl Read,
    dat_writer: &mut impl Write,
    tlist_writer: &mut ListWriter<impl Write>,
    block_size: u32,
) -> Result<(), ProcessError> {
    let mut block_buf = vec![0u8; block_size as usize];
    let mut pos: u32 = 0;
    let mut range_from: u32 = 0;
    let mut data_blocks: u32 = 0;
    let mut zero_blocks: u32 = 0;

    macro_rules! flush_range {
        ($cmd:expr, $count:expr) => {
            if $count != 0 {
                let ranges = [(range_from, range_from + $count)];
                tlist_writer
                    .write_command(&$cmd(&ranges))
                    .map_err(ProcessError::TransferListWrite)?;
                range_from = pos;
                $count = 0;
            }
        };
    }

    while input_reader
        .read(&mut block_buf)
        .map_err(ProcessError::Read)?
        != 0
    {
        if block_buf.iter().all(|&b| b == 0) {
            flush_range!(tlist::Command::New, data_blocks);
            zero_blocks += 1;
        } else {
            flush_range!(tlist::Command::Zero, zero_blocks);
            dat_writer
                .write_all(&block_buf)
                .map_err(ProcessError::Write)?;
            data_blocks += 1;
        }
        pos += 1;
    }

    flush_range!(tlist::Command::New, data_blocks);
    flush_range!(tlist::Command::Zero, zero_blocks);

    dat_writer.flush().map_err(ProcessError::Write)?;
    tlist_writer
        .flush()
        .map_err(ProcessError::TransferListWrite)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const BLOCK_SIZE: u32 = 4;
    const BLOCK_USIZE: usize = BLOCK_SIZE as usize;

    fn run(input: Vec<u8>) -> (Vec<u8>, String) {
        let mut input = Cursor::new(input);
        let mut dat = Cursor::new(vec![]);
        let mut tlist_buf = Cursor::new(vec![]);
        let mut tlist_writer = ListWriter::new(&mut tlist_buf);
        img2sdat(&mut input, &mut dat, &mut tlist_writer, BLOCK_SIZE).expect("img2sdat failed");
        let dat = dat.into_inner();
        let tlist = String::from_utf8(tlist_buf.into_inner()).expect("tlist is not valid utf8");
        (dat, tlist)
    }

    #[test]
    fn basic() {
        let input = vec![1u8; BLOCK_USIZE * 2];
        let (dat, tlist) = run(input.clone());
        assert_eq!(dat, input);
        assert_eq!(tlist, "new 2,0,2\n");
    }

    #[test]
    fn zero_blocks() {
        let input = vec![0u8; BLOCK_USIZE * 2];
        let (dat, tlist) = run(input);
        assert!(dat.is_empty());
        assert_eq!(tlist, "zero 2,0,2\n");
    }

    #[test]
    fn mixed() {
        let mut input = vec![1u8; BLOCK_USIZE];
        input.extend(vec![0u8; BLOCK_USIZE]);
        input.extend(vec![1u8; BLOCK_USIZE]);
        let (dat, tlist) = run(input.clone());
        assert_eq!(dat.len(), BLOCK_USIZE * 2);
        assert_eq!(dat[0..BLOCK_USIZE], input[0..BLOCK_USIZE]);
        assert_eq!(
            dat[BLOCK_USIZE..BLOCK_USIZE * 2],
            input[BLOCK_USIZE * 2..BLOCK_USIZE * 3]
        );
        assert_eq!(tlist, "new 2,0,1\nzero 2,1,2\nnew 2,2,3\n");
    }

    #[test]
    fn empty() {
        let (dat, tlist) = run(vec![]);
        assert!(dat.is_empty());
        assert!(tlist.is_empty());
    }
}
