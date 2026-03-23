use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use brotlic::DecompressorReader;
use clap::Args;

use crate::error::{Error, ErrorExt, ProcessError, check_file_alignment, file_prefix};
use crate::tlist::{self, Reader as ListReader};
use crate::ui::progress_bar;

/// Convert .new.dat or .new.dat.br file to a raw image
#[derive(Args, Debug)]
pub struct Cmd {
    /// Input .new.dat(.br) file
    file: PathBuf,
    /// Transfer list file [default: <FILE stem>.transfer.list]
    #[arg(short, long, value_name = "PATH")]
    transfer_list: Option<PathBuf>,
    /// Output raw image file [default: <FILE stem>.img]
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
    /// Enable brotli decompression [default: auto-detect from FILE extension]
    #[arg(short, long)]
    brotli: bool,
    /// Force overwrite output file
    #[arg(short, long)]
    force: bool,
    /// Block size in bytes
    #[arg(long, value_name = "N", default_value_t = 4096)]
    block_size: u32,
    /// Read/write buffer size in KiB
    #[arg(long, value_name = "N", default_value_t = 256)]
    buffer_size: usize,
}

impl Cmd {
    pub fn run(&mut self) -> Result<(), Error> {
        let use_brotli = self.brotli || self.file.extension().is_some_and(|e| e == "br");

        let input_len = if use_brotli {
            self.file.metadata().path_err(&self.file)?.len()
        } else {
            check_file_alignment(&self.file, self.block_size)?
        };

        let tlist_path = match self.transfer_list.take() {
            Some(path) => path,
            None => self.file_with_extension("transfer.list")?,
        };
        let output_path = match self.output.take() {
            Some(path) => path,
            None => self.file_with_extension("img")?,
        };

        let mut tlist_reader = {
            let f = File::open(&tlist_path).map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => Error::TransferListNotFound(tlist_path.clone()),
                _ => Error::Io(tlist_path.clone(), e),
            })?;
            let reader = BufReader::new(f);
            ListReader::new(reader).path_err(&tlist_path)
        }?;

        let input_reader = {
            let f = File::open(&self.file).path_err(&self.file)?;
            BufReader::with_capacity(self.buffer_size * 1024, f)
        };

        let mut output_writer = {
            let func = if self.force {
                File::create
            } else {
                File::create_new
            };
            let f = func(&output_path).path_err(&output_path)?;
            BufWriter::with_capacity(self.buffer_size * 1024, f)
        };

        let progress_bar = progress_bar(input_len);
        let mut input_reader = progress_bar.wrap_read(input_reader);

        let result = if use_brotli {
            let mut input_reader = DecompressorReader::new(input_reader);
            sdat2img(
                &mut input_reader,
                &mut tlist_reader,
                &mut output_writer,
                self.block_size,
            )
        } else {
            sdat2img(
                &mut input_reader,
                &mut tlist_reader,
                &mut output_writer,
                self.block_size,
            )
        };

        progress_bar.finish_and_clear();

        let (max_offset, total_blocks) = result.map_err(|e| match e {
            ProcessError::Read(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => Error::UnexpectedEof(self.file.clone()),
                _ => Error::Io(self.file.clone(), e),
            },
            ProcessError::Write(e) => Error::Io(output_path.clone(), e),
            ProcessError::TransferListRead(tlist::ReadError::Io(e)) => {
                Error::Io(tlist_path.clone(), e)
            }
            ProcessError::TransferListRead(e) => Error::TransferList(tlist_path.clone(), e),
            ProcessError::TransferListWrite(_) => unreachable!(),
        })?;

        let header = tlist_reader.header();

        if total_blocks != header.total_blocks {
            return Err(Error::TotalBlocksMismatch(
                tlist_path.clone(),
                header.total_blocks,
                total_blocks,
            ));
        }

        let f = output_writer.get_ref();

        f.set_len(u64::from(max_offset * self.block_size))
            .path_err(&output_path)?;
        f.sync_all().path_err(&output_path)
    }

    fn file_with_extension(&self, ext: &str) -> Result<PathBuf, Error> {
        let prefix = file_prefix(&self.file)?;
        let mut path = self
            .file
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(prefix);
        path.set_extension(ext);
        Ok(path)
    }
}

fn sdat2img(
    input_reader: &mut impl Read,
    tlist_reader: &mut ListReader<impl BufRead>,
    output_writer: &mut (impl Write + Seek),
    block_size: u32,
) -> Result<(u32, u32), ProcessError> {
    let mut command_buf = vec![];
    let mut data_buf = vec![0u8; block_size as usize];

    let (mut max_offset, mut pos, mut total_blocks) = (0, 0, 0);

    while let Some(command) = tlist_reader.next_command(&mut command_buf) {
        let command = command.map_err(ProcessError::TransferListRead)?;

        match command {
            tlist::Command::New(ranges) => {
                for (start, end) in ranges {
                    if pos != *start {
                        pos = *start;
                        output_writer
                            .seek(SeekFrom::Start(u64::from(start * block_size)))
                            .map_err(ProcessError::Write)?;
                    }

                    let blocks = end - start;

                    for _ in 0..blocks {
                        input_reader
                            .read_exact(&mut data_buf)
                            .map_err(ProcessError::Read)?;
                        output_writer
                            .write_all(&data_buf)
                            .map_err(ProcessError::Write)?;
                        pos += 1;
                    }
                    max_offset = max_offset.max(*end);
                    total_blocks += blocks;
                }
            }
            tlist::Command::Zero(ranges) => ranges.iter().for_each(|(start, end)| {
                total_blocks += end - start;
                max_offset = max_offset.max(*end);
            }),
            tlist::Command::Erase(ranges) => ranges
                .iter()
                .for_each(|(_, end)| max_offset = max_offset.max(*end)),
        }
        command_buf.clear();
    }

    output_writer.flush().map_err(ProcessError::Write)?;

    Ok((max_offset, total_blocks))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    const BLOCK_SIZE: u32 = 4;
    const BLOCK_USIZE: usize = BLOCK_SIZE as usize;

    fn make_tlist(commands: &str) -> tlist::Reader<Cursor<String>> {
        let data = format!("4\n3\n0\n0\n{commands}");
        tlist::Reader::new(Cursor::new(data)).expect("failed to create tlist reader")
    }

    fn run(tlist_data: &str, input: Vec<u8>, output_blocks: u32) -> (u32, u32, Vec<u8>) {
        let mut tlist_reader = make_tlist(tlist_data);
        let mut input = Cursor::new(input);
        let mut output = Cursor::new(vec![0u8; (output_blocks * BLOCK_SIZE) as usize]);
        let (max_offset, total_blocks) =
            sdat2img(&mut input, &mut tlist_reader, &mut output, BLOCK_SIZE)
                .expect("sdat2img failed");
        (max_offset, total_blocks, output.into_inner())
    }

    #[test]
    fn basic() {
        let data = vec![1u8; BLOCK_USIZE * 3];
        let (max_offset, total_blocks, out) = run("new 4,0,1,1,3", data.clone(), 3);
        assert_eq!(max_offset, 3);
        assert_eq!(total_blocks, 3);
        assert_eq!(out, data);
    }

    #[test]
    fn mixed() {
        let data = vec![1u8; BLOCK_USIZE * 3];
        let (max_offset, total_blocks, out) = run("new 4,0,1,1,3\nzero 2,3,5", data.clone(), 5);
        assert_eq!(max_offset, 5);
        assert_eq!(total_blocks, 5);
        assert_eq!(&out[..BLOCK_USIZE * 3], data);
        assert_eq!(&out[BLOCK_USIZE * 3..], vec![0u8; BLOCK_USIZE * 2]);
    }

    #[test]
    fn seek() {
        let data = vec![1u8; BLOCK_USIZE];
        let (.., out) = run("new 2,2,3", data.clone(), 3);
        assert_eq!(&out[..BLOCK_USIZE * 2], vec![0u8; BLOCK_USIZE * 2]);
        assert_eq!(&out[BLOCK_USIZE * 2..], data);
    }

    #[test]
    fn zero_erase_no_data() {
        let (max_offset, total_blocks, _) = run("zero 2,0,2\nerase 2,2,3", vec![], 3);
        assert_eq!(max_offset, 3);
        assert_eq!(total_blocks, 2);
    }
}
