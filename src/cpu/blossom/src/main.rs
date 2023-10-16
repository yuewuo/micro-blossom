use clap::Parser;
use micro_blossom::cli;

pub fn main() {
    cli::Cli::parse().run();
}
