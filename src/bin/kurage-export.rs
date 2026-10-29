use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Render a seamless, 30 fps terminal aquarium background."
)]
struct Args {
    output: PathBuf,
    #[arg(long)]
    font: Option<PathBuf>,
    #[arg(long)]
    single: bool,
    #[arg(long,default_value_t=kurage_aquarium::export::default_workers())]
    workers: usize,
}
fn main() {
    let args = Args::parse();
    if let Err(error) =
        kurage_aquarium::export::export(args.output, args.single, args.workers, args.font)
    {
        eprintln!("kurage-export: {error}");
        std::process::exit(1);
    }
}
