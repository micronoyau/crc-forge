use clap::{Parser, Subcommand};
use crc_forge::error::{CRCResult, Error};
use std::{fs::File, path::PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input file to forge CRC on
    #[arg(short, long)]
    input_file: PathBuf,

    /// Output file (defaults to <INPUT_FILE>.patched)
    #[arg(short, long)]
    output_file: Option<PathBuf>,

    /// Target crc
    #[arg(short, long, value_parser = hex_arg_parser)]
    target_crc: u32,

    /// Generator polynomial
    #[arg(short, long, default_value_t = 0x04c11db7u32, value_parser = hex_arg_parser)]
    generator: u32,

    /// Turn debugging information on
    #[arg(short, long)]
    debug: bool,

    #[command(subcommand)]
    command: Command,
}

fn hex_arg_parser(arg: &str) -> Result<u32, clap::error::Error> {
    let parsed = match arg.strip_prefix("0x") {
        Some(arg) => u32::from_str_radix(arg, 0x10),
        None => u32::from_str_radix(arg, 10),
    };
    parsed.map_err(|_| clap::error::Error::new(clap::error::ErrorKind::InvalidValue))
}

#[derive(Subcommand)]
enum Command {
    /// Appends 4 bytes at end of file to match target CRC
    Append,
    /// Inserts 4 bytes at given offset to match target CRC
    Insert { offset: usize },
}

fn main() -> CRCResult<()> {
    let cli = Cli::parse();

    let output_path = match cli.output_file {
        Some(output_file) => output_file,
        None => PathBuf::from(format!(
            "{}.patched",
            cli.input_file
                .as_os_str()
                .to_str()
                .ok_or(Error::EncodingError)?
        )),
    };

    let input_file = File::open(cli.input_file)?;

    println!("Output file: {:?}", output_path);
    println!("Target crc: 0x{:08x}", cli.target_crc);

    match cli.command {
        Command::Append => {
            crc_forge::force_crc_append(&input_file, &output_path, cli.target_crc, cli.generator)?;
        }
        Command::Insert { offset } => {
            crc_forge::force_crc_insert(
                &input_file,
                &output_path,
                offset,
                cli.target_crc,
                cli.generator,
            )?;
        }
    };

    Ok(())
}
