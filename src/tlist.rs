use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("{0}: failed to parse header field ({1})")]
    Parse(usize, #[source] std::num::ParseIntError),
    #[error("{0}: unsupported version ({1})")]
    UnsupportedVersion(usize, u8),
    #[error("{0}: {1}")]
    Range(usize, #[source] RangeError),
    #[error("{0}: expected header field, but got EOF")]
    UnexpectedEof(usize),
    #[error("{0}: unsupported command ({1})")]
    UnsupportedCommand(usize, String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error(transparent)]
    Range(#[from] RangeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum RangeError {
    #[error("odd length value ({0})")]
    OddLength(u32),
    #[error("missing value in range set")]
    MissingValue,
    #[error("failed to parse value in range set ({0}: {1})")]
    Parse(String, #[source] std::num::ParseIntError),
    #[error("invalid range ({0},{1})")]
    InvalidRange(u32, u32),
}

#[derive(Debug, PartialEq)]
pub enum Command<'a> {
    Erase(&'a [(u32, u32)]),
    New(&'a [(u32, u32)]),
    Zero(&'a [(u32, u32)]),
}

#[derive(Debug)]
pub struct Reader<R>
where
    R: std::io::BufRead,
{
    header: Header,
    inner: R,
    line_buf: String,
    line_num: usize,
}

#[derive(Debug)]
pub struct Writer<W>
where
    W: std::io::Write,
{
    total_blocks: u32,
    inner: W,
}

#[derive(Default, Debug, PartialEq)]
pub struct Header {
    pub version: u8,
    pub total_blocks: u32,
    pub stash_entries: u32,
    pub max_stash_blocks: u32,
}

impl<R: std::io::BufRead> Reader<R> {
    pub fn new(mut reader: R) -> Result<Self, ReadError> {
        let mut header = Header::default();
        let mut line_buf = String::new();
        let mut line_num = 0usize;

        macro_rules! read_header_field {
            () => {{
                if reader.read_line(&mut line_buf)? == 0 {
                    return Err(ReadError::UnexpectedEof(line_num));
                }
                line_num += 1;
                let res = line_buf
                    .trim()
                    .parse()
                    .map_err(|e| ReadError::Parse(line_num, e))?;
                line_buf.clear();
                res
            }};
        }

        header.version = read_header_field!();
        if header.version > 4 || header.version == 0 {
            return Err(ReadError::UnsupportedVersion(line_num, header.version));
        }

        header.total_blocks = read_header_field!();
        if header.version >= 2 {
            header.stash_entries = read_header_field!();
            header.max_stash_blocks = read_header_field!();
        }

        Ok(Self {
            header,
            inner: reader,
            line_buf,
            line_num,
        })
    }

    #[inline]
    pub fn next_command<'a>(
        &mut self,
        buf: &'a mut Vec<(u32, u32)>,
    ) -> Option<Result<Command<'a>, ReadError>> {
        self.line_buf.clear();

        match self.inner.read_line(&mut self.line_buf) {
            Ok(0) => return None,
            Err(e) => return Some(Err(e.into())),
            _ => {}
        }
        self.line_num += 1;

        let Some(space) = self.line_buf.find(' ') else {
            return Some(Err(ReadError::UnsupportedCommand(
                self.line_num,
                self.line_buf.clone(),
            )));
        };

        let cmd_str = &self.line_buf[..space];
        let rest = self.line_buf[space + 1..].trim_end();

        macro_rules! parse_command {
            ($command:expr) => {{
                if let Err(e) = parse_range_set(rest, buf) {
                    return Some(Err(ReadError::Range(self.line_num, e)));
                }
                Some(Ok($command))
            }};
        }

        match cmd_str {
            "new" => parse_command!(Command::New(buf)),
            "zero" => parse_command!(Command::Zero(buf)),
            "erase" => parse_command!(Command::Erase(buf)),
            _ => Some(Err(ReadError::UnsupportedCommand(
                self.line_num,
                cmd_str.to_string(),
            ))),
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_parts(self) -> (R, Header) {
        (self.inner, self.header)
    }
}

impl<W: std::io::Write> Writer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            total_blocks: 0,
            inner: writer,
        }
    }

    #[inline]
    pub fn write_command(&mut self, command: &Command) -> Result<(), WriteError> {
        macro_rules! write_command {
            ($cmd:expr, $ranges:expr,$update_total_blocks:expr) => {{
                write!(self.inner, "{} {}", $cmd, $ranges.len() * 2)?;
                for (start, end) in *$ranges {
                    check_range(*start, *end)?;
                    write!(self.inner, ",{start},{end}")?;
                    if $update_total_blocks {
                        self.total_blocks += end - start;
                    }
                }
                writeln!(self.inner).map_err(Into::into)
            }};
        }

        match command {
            Command::New(ranges) => write_command!("new", ranges, true),
            Command::Zero(ranges) => write_command!("zero", ranges, true),
            Command::Erase(ranges) => write_command!("erase", ranges, false),
        }
    }

    pub fn flush(&mut self) -> Result<(), WriteError> {
        self.inner.flush().map_err(Into::into)
    }

    pub fn inner(&self) -> &W {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn into_parts(self) -> (W, u32) {
        (self.inner, self.total_blocks)
    }
}

impl std::fmt::Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.version, self.total_blocks)?;
        if self.version >= 2 {
            write!(f, "\n{}\n{}", self.stash_entries, self.max_stash_blocks)?;
        }
        writeln!(f)
    }
}

#[inline]
fn parse_range_set(s: &str, v: &mut Vec<(u32, u32)>) -> Result<(), RangeError> {
    let mut split = s.split(',');

    macro_rules! parse_next {
        () => {{
            let num_str = split.next().ok_or(RangeError::MissingValue)?;
            match num_str.parse() {
                Ok(num) => num,
                Err(e) => return Err(RangeError::Parse(num_str.to_string(), e)),
            }
        }};
    }

    let len: u32 = parse_next!();
    if !len.is_multiple_of(2) {
        return Err(RangeError::OddLength(len));
    }

    for _ in 0..(len / 2) {
        let start: u32 = parse_next!();
        let end: u32 = parse_next!();
        check_range(start, end)?;
        v.push((start, end));
    }

    Ok(())
}

#[inline]
fn check_range(start: u32, end: u32) -> Result<(), RangeError> {
    if end <= start {
        return Err(RangeError::InvalidRange(start, end));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn parse_range_set_ok() {
        let cases = vec![
            ("2,43376,43888", vec![(43_376, 43_888)]),
            (
                "4,274569,274689,274752,275144",
                vec![(274_569, 274_689), (274_752, 275_144)],
            ),
            (
                "8,272812,273052,273053,273152,273167,273173,273236,273403",
                vec![
                    (272_812, 273_052),
                    (273_053, 273_152),
                    (273_167, 273_173),
                    (273_236, 273_403),
                ],
            ),
        ];

        let mut buf = Vec::new();

        for (input, expect) in cases {
            parse_range_set(input, &mut buf)
                .unwrap_or_else(|e| panic!("parse_range_set({input:?}) failed: {e}"));
            assert_eq!(buf, expect, "input: {input:?}");
            buf.clear();
        }
    }

    #[test]
    fn parse_range_set_errors() {
        type Case<'a> = (&'a str, fn(&RangeError) -> bool);

        let cases: &[Case] = &[
            ("1,1,2", |e| matches!(e, RangeError::OddLength(1))),
            ("2,43376", |e| matches!(e, RangeError::MissingValue)),
            ("abc", |e| matches!(e, RangeError::Parse(..))),
            ("2,abc,43888", |e| matches!(e, RangeError::Parse(..))),
            ("", |e| matches!(e, RangeError::Parse(..))),
            ("2,2,1", |e| matches!(e, RangeError::InvalidRange(2, 1))),
        ];

        let mut buf = Vec::new();

        for (input, check) in cases {
            let err = parse_range_set(input, &mut buf)
                .expect_err(&format!("expected error for input: {input:?}"));
            assert!(check(&err), "unexpected error {err:?} for input {input:?}");
            buf.clear();
        }
    }

    #[test]
    fn header_display() {
        let cases = [
            (
                Header {
                    version: 4,
                    total_blocks: 1024,
                    ..Default::default()
                },
                "4\n1024\n0\n0\n",
            ),
            (
                Header {
                    version: 1,
                    total_blocks: 1024,
                    ..Default::default()
                },
                "1\n1024\n",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(input.to_string(), expected, "version: {}", input.version);
        }
    }

    #[test]
    fn reader_ok() {
        let data = [
            "4",
            "290815",
            "0",
            "0",
            "new 2,43376,43888",
            "new 2,278216,278728",
            "new 2,42864,43376",
            "new 2,277704,278216",
            "new 2,42352,42864",
        ]
        .join("\n");

        let mut reader =
            Reader::new(Cursor::new(data)).unwrap_or_else(|e| panic!("Reader::new failed: {e}"));

        let expected = [
            Command::New(&[(43_376, 43_888)]),
            Command::New(&[(278_216, 278_728)]),
            Command::New(&[(42_864, 43_376)]),
            Command::New(&[(277_704, 278_216)]),
            Command::New(&[(42_352, 42_864)]),
        ];

        let mut buf = Vec::new();
        let mut count = 0;

        while let Some(command) = reader.next_command(&mut buf) {
            let command = command.unwrap_or_else(|e| panic!("reader failed: {e}"));
            assert_eq!(command, expected[count], "command {count}");
            buf.clear();
            count += 1;
        }
        assert_eq!(count, expected.len(), "command count mismatch");
        assert_eq!(
            *reader.header(),
            Header {
                version: 4,
                total_blocks: 290_815,
                stash_entries: 0,
                max_stash_blocks: 0
            },
        );
    }

    #[test]
    fn reader_errors() {
        type Case<'a> = (&'a str, fn(&ReadError) -> bool);

        let cases: &[Case] = &[
            ("", |e| matches!(e, ReadError::UnexpectedEof(0))),
            ("0\n", |e| matches!(e, ReadError::UnsupportedVersion(1, 0))),
            ("5\n", |e| matches!(e, ReadError::UnsupportedVersion(1, 5))),
            ("abc\n", |e| matches!(e, ReadError::Parse(1, _))),
            ("4\n", |e| matches!(e, ReadError::UnexpectedEof(1))),
            ("4\n290815\n", |e| matches!(e, ReadError::UnexpectedEof(2))),
        ];

        for (input, check) in cases {
            let err = Reader::new(Cursor::new(input))
                .expect_err(&format!("expected error for input: {input:?}"));
            assert!(check(&err), "unexpected error {err:?} for input {input:?}");
        }
    }

    #[test]
    fn reader_command_errors() {
        type Case<'a> = (&'a str, fn(&ReadError) -> bool);

        let cases: &[Case] = &[
            ("4\n0\n0\n0\nunknown 2,1,2\n", |e| {
                matches!(e, ReadError::UnsupportedCommand(5, _))
            }),
            ("4\n0\n0\n0\nnew 1,1,2\n", |e| {
                matches!(e, ReadError::Range(5, _))
            }),
        ];

        let mut buf = Vec::new();

        for (input, check) in cases {
            let mut reader = Reader::new(Cursor::new(input))
                .unwrap_or_else(|e| panic!("Reader::new failed: {e}"));

            let err = reader
                .next_command(&mut buf)
                .expect("expected Some")
                .expect_err(&format!("expected error for input: {input:?}"));
            assert!(check(&err), "unexpected error {err:?} for input {input:?}");
            buf.clear();
        }
    }

    #[test]
    fn writer_ok() {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_command(&Command::New(&[(43_376, 43_888)]))
            .unwrap_or_else(|e| panic!("write_command failed: {e}"));
        writer
            .write_command(&Command::Erase(&[(216_926, 217_950)]))
            .unwrap_or_else(|e| panic!("write_command failed: {e}"));
        writer
            .write_command(&Command::New(&[
                (273_979, 274_462),
                (274_477, 274_484),
                (274_547, 274_569),
            ]))
            .unwrap_or_else(|e| panic!("write_command failed: {e}"));
        writer
            .flush()
            .unwrap_or_else(|e| panic!("flush failed: {e}"));

        let (writer, total_blocks) = writer.into_parts();
        let result =
            String::from_utf8(writer.clone()).unwrap_or_else(|e| panic!("from_utf8 failed: {e}"));

        let expected = [
            "new 2,43376,43888",
            "erase 2,216926,217950",
            "new 6,273979,274462,274477,274484,274547,274569",
            "",
        ]
        .join("\n");

        assert_eq!(result, expected);
        assert_eq!(total_blocks, 1024);
    }
}
