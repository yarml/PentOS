import gdb # type: ignore
import struct
from utils import bits

class GetPA(gdb.Command):
    def __init__(self):
        super(GetPA, self).__init__("getpa", gdb.COMMAND_USER)

    def invoke(self, arg, from_tty):
        argv = gdb.string_to_argv(arg)
        if len(argv) not in (1, 2):
            print("Usage: getpa <virt_addr> [pml4_phys]")
            return

        try:
            virt = int(gdb.parse_and_eval(argv[0]))
        except:
            print("Invalid address:", argv[0])
            return

        inf = gdb.selected_inferior()

        def read64_phys(paddr):
            try:
                return struct.unpack("<Q", inf.read_memory(paddr, 8).tobytes())[0]
            except gdb.MemoryError:
                raise RuntimeError(f"Cannot read physical memory {hex(paddr)}")
        
        print(f"VirtAddr = {hex(virt)}")

        pml4_i = bits(virt, 47, 39)
        pdpt_i = bits(virt, 38, 30)
        pd_i   = bits(virt, 29, 21)
        pt_i   = bits(virt, 20, 12)
        offset = bits(virt, 11, 0)

        if len(argv) >= 2:
            pml4_phys = int(gdb.parse_and_eval(argv[1])) & ~0xFFF
            print(f"PML4T (explicit) @ {hex(pml4_phys)}")
        else:
            cr3 = int(gdb.parse_and_eval("$cr3"))
            pml4_phys = cr3 & ~0xFFF
            print(f"CR3 = {hex(cr3)}, PML4T @ {hex(pml4_phys)}")
        
        print(f"Indices:\nPML4={pml4_i} PDPT={pdpt_i} PD={pd_i} PT={pt_i} off={offset}")

        pml4e_addr = pml4_phys + pml4_i * 8
        pml4e = read64_phys(pml4e_addr)
        print(f"PML4E @ {hex(pml4e_addr)} = {hex(pml4e)}")

        if not (pml4e & 1):
            print("=> Not present at PML4 level.")
            return

        pdpt_phys = pml4e & ~0xFFF

        pdpte_addr = pdpt_phys + pdpt_i * 8
        pdpte = read64_phys(pdpte_addr)
        print(f"PDPTE @ {hex(pdpte_addr)} = {hex(pdpte)}")

        if not (pdpte & 1):
            print("=> Not present at PDPT level.")
            return

        if (pdpte >> 7) & 1:  # PS: 1GB page
            phys = (pdpte & 0xFFFFFC0000000) + (virt & 0x3FFFFFFF)
            print("PA =", hex(phys))
            return

        pd_phys = pdpte & ~0xFFF

        pde_addr = pd_phys + pd_i * 8
        pde = read64_phys(pde_addr)
        print(f"PDE @ {hex(pde_addr)} = {hex(pde)}")

        if not (pde & 1):
            print("=> Not present at PD level.")
            return

        if (pde >> 7) & 1:  # PS: 2MB page
            phys = (pde & 0xFFFFFFE00000) + (virt & 0x1FFFFF)
            print("PA =", hex(phys))
            return

        pt_phys = pde & ~0xFFF

        pte_addr = pt_phys + pt_i * 8
        pte = read64_phys(pte_addr)
        print(f"PTE @ {hex(pte_addr)} = {hex(pte)}")

        if not (pte & 1):
            print("=> Not present at PT level.")
            return

        phys = (pte & ~0xFFF) + offset
        print("PA =", hex(phys))
