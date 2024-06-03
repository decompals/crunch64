use clap::{Parser, ValueEnum};
use crunch64::Crunch64Error;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
    process,
};

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Command {
    Compress,
    Decompress,
}

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum CompressionType {
    Yay0,
    Yaz0,
    Mio0,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    command: Command,
    #[arg(ignore_case = true)]
    format: CompressionType,
    #[arg()]
    in_path: String,
    #[arg()]
    out_path: String,
}

fn compress(format: CompressionType, bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    match format {
        CompressionType::Yay0 => crunch64::yay0::compress(bytes),
        CompressionType::Yaz0 => crunch64::yaz0::compress(bytes),
        CompressionType::Mio0 => crunch64::mio0::compress(bytes),
        // _ => Err(Crunch64Error::UnsupportedCompressionType),
    }
}

fn decompress(format: CompressionType, bytes: &[u8]) -> Result<Box<[u8]>, Crunch64Error> {
    match format {
        CompressionType::Yay0 => crunch64::yay0::decompress(bytes),
        CompressionType::Yaz0 => crunch64::yaz0::decompress(bytes),
        CompressionType::Mio0 => crunch64::mio0::decompress(bytes),
        //_ => Err(Crunch64Error::UnsupportedCompressionType),
    }
}

fn main() {
    let args = Args::parse();

    let file_bytes = read_file_bytes(args.in_path);

    let out_bytes = match args.command {
        Command::Compress => match compress(args.format, file_bytes.as_slice()) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("{:?}", error);
                process::exit(1);
            }
        },
        Command::Decompress => match decompress(args.format, file_bytes.as_slice()) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("{:?}", error);
                process::exit(1);
            }
        },
    };

    let mut buf_writer = match File::create(args.out_path) {
        Ok(file) => BufWriter::new(file),
        Err(_error) => {
            eprintln!("Failed to create file");
            process::exit(1);
        }
    };

    let _ = buf_writer.write_all(&out_bytes);
}

pub fn read_file_bytes<P: Into<PathBuf>>(path: P) -> Vec<u8> {
    let file = match File::open(path.into()) {
        Ok(file) => file,
        Err(_error) => {
            eprintln!("Failed to open file");
            process::exit(1);
        }
    };

    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();

    let _ = buf_reader.read_to_end(&mut buffer);

    buffer
}
