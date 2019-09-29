lazy_static::lazy_static! {
    static ref STACK_BOTTOM: usize = {
        let dummy = 0u8 as *const u8;
        (&dummy) as *const *const u8 as *const u8 as usize
    };
}

extern crate vmm;

fn main() {}
