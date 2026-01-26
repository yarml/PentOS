[BITS 16]

; Sync with bootloader/src/hart.rs
%define BASE 1028
%define STATUS_FLAG 1024

%define STATUS_WAIT 0
%define STATUS_ALIVE 1
%define STATUS_DONE 2
%define STATUS_ERROR 3

%define DATA_SEG 0x10
%define CODE_SEG 0x08

%define CR0_B32_SWITCH 0x00050033

; code "section"
ORG 0
    cli
    wbinvd ; Honestly just following linux here, ig this is just for caution

    mov ax, cs
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov BYTE [STATUS_FLAG], STATUS_ALIVE

    ; "parameters" for .start_32
    mov ecx, DWORD [BASE]

    ; add base offset to structures
    add DWORD [far_jump_32], ecx
    add DWORD [b32_gdtr+2], ecx

    lidt [idtr]
    lgdt [b32_gdtr]

    mov eax, CR0_B32_SWITCH
    mov cr0, eax
    jmp FAR DWORD [far_jump_32]
[BITS 32]
.start_32:
    mov edx, DATA_SEG
    mov ss, edx
    mov ds, edx
    mov es, edx
    mov fs, edx
    mov gs, edx

.halt:
    hlt
    jmp .halt

; data "section"
idtr:
    dw 0
    dq 0

b32_gdtr:
    dw b32_gdt.end - b32_gdt.start - 1
    dd b32_gdt.start

b32_gdt.start:
    dq 0 ; NULL segment
    dq 0x00CF9B000000FFFF ; Code32
    dq 0x00CF93000000FFFF ; Data
b32_gdt.end:

far_jump_32:
    dd .start_32
    dw CODE_SEG
