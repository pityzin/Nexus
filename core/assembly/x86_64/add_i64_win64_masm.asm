; MASM fallback for Windows machines without NASM. Windows x64 ABI: RCX, RDX -> RAX.
; Leaf routine: no stack use and no nonvolatile register changes.
PUBLIC nexus_asm_add_i64
_TEXT SEGMENT
nexus_asm_add_i64 PROC
    mov rax, rcx
    add rax, rdx
    ret
nexus_asm_add_i64 ENDP
_TEXT ENDS
END
