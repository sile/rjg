use clap::Parser;

#[derive(Parser)]
struct Args {}

fn main() -> orfail::Result<()> {
    let _args = Args::parse();
    Ok(())
}
