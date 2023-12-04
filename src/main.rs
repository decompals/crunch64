use clap::Parser;
use crunch64::Crunch64Error;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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

    let file_magic: &[u8] = &file_bytes[0..4];

    let out_bytes = match file_magic {
        b"Yay0" => crunch64::CompressionType::Yay0.decompress(file_bytes.as_slice()),
        b"Yaz0" => crunch64::CompressionType::Yaz0.decompress(file_bytes.as_slice()),
        _ => {
            panic!("File format not recognized - magic: {:?}", file_magic)
        }
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
