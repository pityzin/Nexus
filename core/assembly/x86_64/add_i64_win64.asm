; Windows x64 ABI: left in RCX, right in RDX, result in RAX.
; Leaf routine: does not modify nonvolatile registers or touch the stack.
bits 64
default rel
section .text
global nexus_asm_add_i64
nexus_asm_add_i64:
    mov rax, rcx
    add rax, rdx
    ret
