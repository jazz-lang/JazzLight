use jazzvm::hash::*;

fn main() {
    println!("{} = {}", "Hello,world!", hash_bytes(b"Hello,world!"));
}
