use clap::Parser;

use tensegrity_lab::build::brick::BrickName;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long, value_enum)]
    prototype: Option<BrickName>,
}

fn main() {
    let args = Args::parse();
    let brick_name = args.prototype;
    tensegrity_lab::application::run_with(brick_name);
}