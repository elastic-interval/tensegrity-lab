use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long)]
    brick_name: Option<String>,
}

fn main() {
    let Args { brick_name } = Args::parse();

    tensegrity_lab::application::run_with(brick_name);
}