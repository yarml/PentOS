[BITS 16]

; sync with bootloader/src/hart.rs
%define STATUS_FLAG_OFFSET 1024
%define BASE_OFFSET 1028
%define CR3_OFFSET 1032
%define ENTRYPOINT_OFFSET 1040
%define STACK_OFFSET 1048

%define STATUS_WAIT 0
%define STATUS_ALIVE 1
%define STATUS_DONE 2
%define STATUS_ERROR 3
; end sync

%define CHUNK_SIZE 0x10000

%define CODE32_SEG 0x08
%define DATA_SEG 0x10
%define CODE64_SEG 0x18

%define CR0_B32_SWITCH 0x00050033
%define CR0_B64_SWITCH 0x80050033

%define CR4_B64_SWITCH 0x00000028 ; only PAE & DE for now, other flags will be setup in rust

%define EFER_LO_B64_SWITCH 0x00000900 ; NE & LME
%define EFER_HI_B64_SWITCH 0x00000000

%define EFER_MSR 0xC000_0080

; code "section"
ORG 0
ap_init:
.start_16:
    cli
    wbinvd ; Honestly just following linux here, ig this is just for caution

    mov ax, cs
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    mov BYTE [STATUS_FLAG_OFFSET], STATUS_ALIVE

    ; "parameters" for .start_32
    mov ebx, DWORD [BASE_OFFSET]

    ; add BASE_OFFSET to structures
    add DWORD [far_jump_32], ebx
    add DWORD [far_jump_64], ebx
    add DWORD [gdtr+2], ebx

    lidt [idtr]
    lgdt [gdtr]

    mov eax, CR0_B32_SWITCH
    mov cr0, eax
    jmp FAR DWORD [far_jump_32]
.halt_16:
    hlt
    jmp .halt_16
[BITS 32]
ALIGN 4
.start_32:
    mov edx, DATA_SEG
    mov ss, edx
    mov ds, edx
    mov es, edx
    mov fs, edx
    mov gs, edx

    lea esp, [ebx + CHUNK_SIZE]

    mov eax, CR4_B64_SWITCH
    mov cr4, eax

    mov edx, DWORD [ebx + CR3_OFFSET]
    mov cr3, edx

    mov eax, EFER_LO_B64_SWITCH
    mov edx, EFER_HI_B64_SWITCH
    mov ecx, EFER_MSR
    wrmsr

    mov eax, CR0_B64_SWITCH
    mov cr0, eax
    jmp FAR DWORD [ebx+far_jump_64]
.halt_32:
    hlt
    jmp .halt_32

[BITS 64]
ALIGN 8
.start_64:
    mov rdi, rbx
    mov rsp, QWORD [ebx + STACK_OFFSET]
    mov rax, QWORD [ebx + ENTRYPOINT_OFFSET]
    jmp rax ; goto rust
.halt_64:
    hlt
    jmp .halt_64

; data "section"
idtr:
    dw 0
    dq 0

gdtr:
    dw gdt.end - gdt.start - 1
    dq gdt.start

gdt:
.start:
    dq 0 ; NULL segment
    dq 0x00CF9B000000FFFF ; Code32
    dq 0x00CF93000000FFFF ; Data
    dq 0x00AF9B000000FFFF ; Code64
.end:

far_jump_32:
    dd ap_init.start_32
    dw CODE32_SEG

far_jump_64:
    dd ap_init.start_64
    dw CODE64_SEG
