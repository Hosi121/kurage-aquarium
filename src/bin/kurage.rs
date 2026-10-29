use clap::Parser;
fn main() {
    if let Err(error) = kurage_aquarium::cli::view(kurage_aquarium::cli::Viewer::parse()) {
        eprintln!("kurage: {error}");
        std::process::exit(1);
    }
}
