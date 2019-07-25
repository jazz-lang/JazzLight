# Codegen module

This module should generate x86-64 assembly and then execute it.

What we should do for codegeneration: 
Generate basic blocks from VM opcodes -> Translate these basic blocks to LIR (low level intermediate representation) -> 
Perform basic optimizations -> Allocate registers -> Emit machine code