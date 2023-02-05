use clap::Parser;

use tensegrity_lab::build::brick::BrickName::LeftTwist;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the prototype to settle and capture
    #[arg(long)]
    prototype: Option<String>,
}

fn main() {
    let args = Args::parse();
    let brick_name = args.prototype.map(|name|
        match name.as_str() {
            "LeftTwist" => LeftTwist,
            _ => panic!("no such prototype: {name}")
        }
    );
    tensegrity_lab::application::run_with(brick_name);
}