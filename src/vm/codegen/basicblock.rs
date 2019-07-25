use crate::vm;
use vm::opcodes::*;

#[derive(Clone, PartialEq,Debug)]
pub struct BBlock {
    pub opcodes: Vec<Opcode>,
}
impl BBlock {
    pub fn empty() -> BBlock {
        BBlock { opcodes: vec![] }
    }
    pub fn push(&mut self, o: Opcode) {
        self.opcodes.push(o);
    }
}


pub fn translate_to_blocks(code: Vec<Opcode>) -> Vec<BBlock> {
    let mut pc = 0;
    let mut blocks = vec![];
    use hashlink::LinkedHashMap;
    let mut targets = LinkedHashMap::new();
    let mut bb = BBlock::empty();
    let mut block_id = 0;
    for (i,op) in code.iter().enumerate() {
        targets.insert(i, block_id);
        match op {
            Opcode::BlockEnd => {
                blocks.push(bb.clone());
                bb = BBlock::empty();
                

                continue;
            }
            Opcode::BlockStart => {
                block_id += 1;
            }
            Opcode::Return => {
                blocks.push(bb.clone());
                bb = BBlock::empty();
                
            }
            _ => 
            {
                bb.push(op.clone());
            }
        }
    }
    for bb in blocks.iter_mut() {
        for op in bb.opcodes.iter_mut() {
            let op: &mut Opcode = op;
            match op {
                Opcode::Jump(target) | Opcode::JumpIf(target) | Opcode::JumpIfFalse(target) => {
                    let new_target = *targets.get(&(*target as usize)).unwrap();
                    *target = new_target + 1;
                }
                _ => ()
            }
        }   
    }
    for bb in blocks.iter_mut() {
        bb.opcodes.retain(|op| *op != Opcode::Label);
    }


    blocks
}



/*
pub fn translate_to_blocks(code: Vec<Opcode>) -> Vec<BBlock> {
    let mut pc = 0;
    let mut blocks = vec![];
    let iter = code.split(|op| match op {
        Opcode::Jump(_) | Opcode::JumpIf(_) | Opcode::JumpIfFalse(_) => true,
        Opcode::Return => true,
        Opcode::Yield => true,
        Opcode::Label => true,
        _ => false
    });
    for x in iter {
        let mut bb = BBlock::empty();
        for x in x.iter() {
            bb.opcodes.push(x.clone());
        }
        blocks.push(bb);
    }
    blocks
}*/

#[cfg(test)]
mod tests {
    use super::{translate_to_blocks, Opcode};

    #[test]
    fn simple() {
        let code = vec![Opcode::LoadInt(0), Opcode::LoadInt(0), Opcode::Return];

        let bbs = translate_to_blocks(code);
        assert!(bbs.len() == 1);
    }

    #[test]
    fn jump() {
        let code = vec![
            Opcode::Jump(4),
            Opcode::LoadInt(0),
            Opcode::LoadInt(2),
            Opcode::Mul,
            Opcode::LoadInt(42),
            Opcode::Return,
        ];
        let bbs = translate_to_blocks(code);

        assert!(bbs.len() == 2);
    }
}
