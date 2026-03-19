[BITS 64]

section .text

global _start

_start:
    xor rax, rax
    xor rbx, rbx
    xor rcx, rcx
    xor rdx, rdx
.loop:
    hlt
    jmp .loop
