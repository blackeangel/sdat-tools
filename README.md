# sdat-tools

Android block-based OTA tools for converting between raw images and `.new.dat`/`.new.dat.br` format.

## Commands

### `sdat2img`

```
Convert .new.dat or .new.dat.br file to a raw image

Usage: sdat-tools sdat2img [OPTIONS] <FILE>

Arguments:
  <FILE>  Input .new.dat(.br) file

Options:
  -t, --transfer-list <PATH>  Transfer list file [default: <FILE stem>.transfer.list]
  -o, --output <PATH>         Output raw image file [default: <FILE stem>.img]
  -b, --brotli                Enable brotli decompression [default: auto-detect from FILE extension]
  -f, --force                 Force overwrite output file
      --block-size <N>        Block size in bytes [default: 4096]
      --buffer-size <N>       Read/write buffer size in KiB [default: 256]
  -h, --help                  Print help
```

### `img2sdat`

```
Convert raw image file to .new.dat or .new.dat.br

Usage: sdat-tools img2sdat [OPTIONS] <FILE>

Arguments:
  <FILE>  Input raw image file

Options:
  -o, --output <PATH>     Directory for output files [default: <FILE directory>]
  -b, --brotli [<LEVEL>]  Enable brotli compression at specified level [default: 5]
  -f, --force             Force overwrite output files
      --format <FMT>      Transfer list format version [default: 4]
      --block-size <N>    Block size in bytes [default: 4096]
      --buffer-size <N>   Read/write buffer size in KiB [default: 256]
  -h, --help              Print help
```

### `install`

```
Install hardlinks to bundled commands

Usage: sdat-tools install [OPTIONS] <DIR>

Arguments:
  <DIR>  Directory to install hardlinks into

Options:
  -f, --force  Force overwrite existing hardlinks
  -h, --help   Print help
```

## Installation

Download a pre-built binary from the [releases page](../../releases), or build from source:

```sh
cargo install --path .
```

To install `sdat2img` and `img2sdat` as standalone commands:

```sh
sdat-tools install ~/.local/bin
```

## Usage

```sh
# Convert .new.dat.br to raw image
sdat2img system.new.dat.br

# Convert raw image to .new.dat with brotli compression
img2sdat system.img -b

# Or using sdat-tools directly
sdat-tools sdat2img system.new.dat.br
sdat-tools img2sdat system.img -b
```

## Acknowledgements

Inspired by [sdat2img](https://github.com/xpirt/sdat2img) and
[img2sdat](https://github.com/xpirt/img2sdat) by xpirt.
This project aims to be a more portable, reliable, and ergonomic alternative.

## License

MIT License - See [LICENSE](LICENSE) for details.
