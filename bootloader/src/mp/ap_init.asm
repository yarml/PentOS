[BITS 16]

global ap_bootstrap_begin
global ap_bootstrap_end

section .text
    ; ap_bootstrap_begin will be loaded at address 0 relative to the chosen real mode segment
    ap_bootstrap_begin:
        cli
        mov WORD [1024], 1
        mov al, 97
        mov dx, 0xE9
        mov cx, 6
    .loop.start:
        cmp cx, 0
        je .loop.end
        out dx, al
        dec cx
        jmp .loop.start
    .loop.end:
        hlt
    ap_bootstrap_end:

