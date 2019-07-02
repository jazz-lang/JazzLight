extern crate nabox;

use std::env::args;

fn main() {
    nabox::initialize(args().collect::<Vec<String>>());
}
