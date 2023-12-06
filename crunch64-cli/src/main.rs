use clap::{error::ErrorKind, CommandFactory, Parser, ValueEnum};
use crunch64::{CompressionType, Crunch64Error};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

#[derive(ValueEnum, Copy, Clone, Debug, PartialEq, Eq)]
enum Command {
    Compress,
    Decompress,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg()]
    command: Command,
    #[arg()]
    format: String,
    #[arg()]
    in_path: String,
    #[arg()]
    out_path: String,
}

fn main() {
    let args = Args::parse();

    let file_bytes = match read_file_bytes(args.in_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("{:?}", error);
            return;
        }
    };

    let compression_format = match args.format.as_str() {
        "Yay0" | "yay0" => CompressionType::Yay0,
        "Yaz0" | "yaz0" => CompressionType::Yaz0,
        "Mio0" | "mio0" => CompressionType::Mio0,
        _ => {
            let mut cmd = Args::command();
            cmd.error(
                ErrorKind::InvalidValue,
                format!("File format {} not supported", args.format),
            )
            .exit()
        }
    };

    let out_bytes = match args.command {
        Command::Compress => match compression_format.compress(file_bytes.as_slice()) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{:?}", error);
                return;
            }
        },
        Command::Decompress => match compression_format.decompress(file_bytes.as_slice()) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{:?}", error);
                return;
            }
        },
    };

    let mut buf_writer = match File::create(args.out_path) {
        Ok(file) => BufWriter::new(file),
        Err(_error) => {
            println!("Failed to create file");
            return;
        }
    };

    let _ = buf_writer
        .write_all(&out_bytes)
        .or(Err(Crunch64Error::WriteFile));
}

pub fn read_file_bytes<P: Into<PathBuf>>(path: P) -> Result<Vec<u8>, Crunch64Error> {
    let file = match File::open(path.into()) {
        Ok(file) => file,
        Err(_error) => {
            return Err(Crunch64Error::OpenFile);
        }
    };

    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();

    let _ = buf_reader
        .read_to_end(&mut buffer)
        .or(Err(Crunch64Error::ReadFile));

    Ok(buffer)
}
