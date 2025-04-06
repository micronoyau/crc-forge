# crc-forge

## Purpose

`crc-forge` is a tool meant to edit your data to forge any CRC checksum. Useful for CTF challenges.

## Installing

Simply clone this repo and run
```
cargo install --path .
```

This will install the `crc-forge` binary in your `$HOME/.cargo/bin` directory. Dont forget to add it to your `PATH` environment variable.

## Usage

### Options

```
➜  ~ crc-forge -h
Usage: crc-forge [OPTIONS] --input-file <INPUT_FILE> --target-crc <TARGET_CRC> <COMMAND>

Commands:
  append  Appends 4 bytes at end of file to match target CRC
  insert  Inserts 4 bytes at given offset to match target CRC
  help    Print this message or the help of the given subcommand(s)

Options:
  -i, --input-file <INPUT_FILE>    Input file to forge CRC on
  -o, --output-file <OUTPUT_FILE>  Output file (defaults to <INPUT_FILE>.patched)
  -t, --target-crc <TARGET_CRC>    Target crc
  -g, --generator <GENERATOR>      Generator polynomial [default: 79764919]
  -d, --debug                      Turn debugging information on
  -h, --help                       Print help
  -V, --version                    Print version
```

### Examples

In the following, let's take the following assumptions:

 - The input file is named `foo.txt`. Its content is
```
➜  ~ hexdump -C foo.txt 
00000000  6c 6f 72 65 6d 20 69 70  73 75 6d 0a              |lorem ipsum.|
```

 - The target CRC is `0x42424242`

#### Appending a pattern at the end of file

```
➜  ~ crc-forge -i foo.txt -t 0x42424242 append
➜  ~ hexdump -C foo.txt.patched
00000000  6c 6f 72 65 6d 20 69 70  73 75 6d 0a 32 9e 74 74  |lorem ipsum.2.tt|
```

#### Inserting a pattern at some offset

```
➜  ~ crc-forge -i foo.txt -t 0x42424242 insert 5
➜  ~ hexdump -C foo.txt.patched
00000000  6c 6f 72 65 6d 9e 0a 76  86 20 69 70 73 75 6d 0a  |lorem..v. ipsum.|
```
