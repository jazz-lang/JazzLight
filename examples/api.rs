extern crate jazzlight;

use jazzlight::State;

fn main() {
    let mut state = State::new();

    state.set_var("x", vec![0, 1, 2, 3]);

    state.eval("x[2] = 4* 2").unwrap();

    println!("{}", state.get_var("x").borrow());
}
