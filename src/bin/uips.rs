use clap::Parser;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::result::Result;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    input: OsString,
    ips: OsString,
    output: Option<OsString>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let output = args
        .output
        .or_else(|| {
            let mut ret = PathBuf::new();

            ret.push(Path::new(&args.input).parent()?);
            ret.push(Path::new(&args.ips).file_stem()?);
            ret.set_extension(Path::new(&args.input).extension()?);

            Some(ret.into_os_string())
        })
        .expect("Could not deduce output file name");

    let mut data = fs::read(args.input)?;
    let ips = fs::read(args.ips)?;
    umbralips::apply_ips(&mut data, &ips)?;
    fs::write(output, data)?;

    Ok(())
}
