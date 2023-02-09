use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long)]
    prototype: Option<String>,
}

fn main() {
    let Args { prototype } = Args::parse();

    tensegrity_lab::application::run_with(prototype);
}