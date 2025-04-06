use core::{CRC32, CRC32Properties};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, Write},
    path::PathBuf,
};

mod core;
pub mod error;
mod math;

use error::CRCResult;

const BUF_SIZE: usize = 0x1000;

pub fn force_crc_append(
    input_file: &File,
    output_path: &PathBuf,
    target_crc: u32,
    generator: u32,
) -> CRCResult<()> {
    // First compute suffix
    let mut reader = BufReader::new(input_file);
    reader.seek(std::io::SeekFrom::Start(0))?;
    let crc = CRC32::new(CRC32Properties {
        g: generator,
        ..Default::default()
    })?;
    let suffix = crc.compute_suffix(
        reader.bytes().map(|res| res.map_err(std::io::Error::into)),
        target_crc,
    )?;

    // Then copy original file to output file and append suffix
    let output_file = File::create(output_path)?;
    let mut reader = BufReader::new(input_file);
    reader.seek(std::io::SeekFrom::Start(0))?;
    let mut writer = BufWriter::new(output_file);
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let read_bytes = reader.read(&mut buf)?;
        if read_bytes == 0 {
            break;
        }
        writer.write_all(&buf[..read_bytes])?;
    }
    writer.write_all(&suffix)?;

    Ok(())
}

pub fn force_crc_insert(
    input_file: &File,
    output_path: &PathBuf,
    offset: usize,
    target_crc: u32,
    generator: u32,
) -> CRCResult<()> {
    // First compute suffix
    let mut reader = BufReader::new(input_file);
    reader.seek(std::io::SeekFrom::Start(0))?;
    let crc = CRC32::new(CRC32Properties {
        g: generator,
        ..Default::default()
    })?;
    let inserted_bytes = crc.compute_inserted(
        reader.bytes().map(|res| res.map_err(std::io::Error::into)),
        offset,
        target_crc,
    )?;

    // Copy prefix
    let output_file = File::create(output_path)?;
    let mut reader = BufReader::new(input_file);
    reader.seek(std::io::SeekFrom::Start(0))?;
    let mut prefix_reader = reader.by_ref().take(offset as u64);
    let mut writer = BufWriter::new(output_file);
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let read_bytes = prefix_reader.read(&mut buf)?;
        if read_bytes == 0 {
            break;
        }
        writer.write_all(&buf[..read_bytes])?;
    }

    // Write inserted bytes
    writer.write_all(&inserted_bytes)?;

    // Copy suffix
    loop {
        let read_bytes = reader.read(&mut buf)?;
        if read_bytes == 0 {
            break;
        }
        writer.write_all(&buf[..read_bytes])?;
    }

    Ok(())
}
