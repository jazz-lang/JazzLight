
extern crate jazzvm;



use std::env::args;

fn main() {
    jazzvm::initialize(args().collect::<Vec<String>>());
}