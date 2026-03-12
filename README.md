# sdat-tools

Android block-based OTA tools for converting between raw images and `.new.dat`/`.new.dat.br` format.

## Commands

### `sdat2img`

```
Convert .new.dat or .new.dat.br file to a raw image

Usage: sdat-tools sdat2img [OPTIONS] <FILE> [TRANSFER_LIST] [OUTPUT]

Arguments:
  <FILE>           Input .new.dat(.br) file
  [TRANSFER_LIST]  Transfer list file [default: <FILE stem>.transfer.list]
  [OUTPUT]         Output raw image file [default: <FILE stem>.img]

Options:
      --buffer-size <N>  Read/write buffer size in KiB [default: 256]
      --block-size <N>   Block size in bytes [default: 4096]
  -f, --force            Force overwrite output file
  -b, --brotli           Enable brotli decompression [default: auto-detect from FILE extension]
  -h, --help             Print help
```

### `img2sdat`

```
Convert raw image file to .new.dat or .new.dat.br

Usage: sdat-tools img2sdat [OPTIONS] <FILE> [OUTPUT]

Arguments:
  <FILE>    Input raw image file
  [OUTPUT]  Directory for output files [default: <FILE directory>]

Options:
      --buffer-size <N>    Read/write buffer size in KiB [default: 256]
      --block-size <N>     Block size in bytes [default: 4096]
  -f, --force              Force overwrite output files
  -b, --brotli [<LEVEL>]   Enable brotli compression at specified level [default: 5]
  -v, --version <VERSION>  Transfer list format version [default: 4]
  -h, --help               Print help
```

### `install`

```
Install hardlinks to bundled commands

Usage: sdat-tools install <DIR>

Arguments:
  <DIR>  Directory to install hardlinks into

Options:
  -h, --help  Print help
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
