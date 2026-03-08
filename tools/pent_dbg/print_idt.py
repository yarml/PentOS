import gdb
import struct

ENTRY_SIZE = 16
MAX_VECTORS = 256

EXCEPTION_NAMES = {
    0:  "#DE  Divide Error",
    1:  "#DB  Debug",
    2:  "     NMI",
    3:  "#BP  Breakpoint",
    4:  "#OF  Overflow",
    5:  "#BR  Bound Range Exceeded",
    6:  "#UD  Invalid Opcode",
    7:  "#NM  Device Not Available",
    8:  "#DF  Double Fault",
    9:  "     Coprocessor Segment Overrun",
    10: "#TS  Invalid TSS",
    11: "#NP  Segment Not Present",
    12: "#SS  Stack Fault",
    13: "#GP  General Protection Fault",
    14: "#PF  Page Fault",
    15: "     Reserved",
    16: "#MF  x87 FPU Error",
    17: "#AC  Alignment Check",
    18: "#MC  Machine Check",
    19: "#XM  SIMD FP Exception",
    20: "#VE  Virtualization Exception",
    21: "#CP  Control Protection Exception",
}

GATE_TYPES = {
    0xE: "Interrupt",
    0xF: "Trap    ",
}

def read_idtr():
    try:
        output = gdb.execute("info registers idtr", to_string=True)
        for line in output.splitlines():
            if "idtr" in line.lower():
                parts = line.split()
                for part in parts:
                    if part.startswith("0x") or part.startswith("0X"):
                        try:
                            return int(part, 16), MAX_VECTORS
                        except ValueError:
                            continue
    except gdb.error:
        pass

    try:
        result = gdb.parse_and_eval("$idtr_base")
        return int(result), MAX_VECTORS
    except gdb.error:
        pass

    return None, None


def read_memory(addr, size):
    inferior = gdb.selected_inferior()
    try:
        mem = inferior.read_memory(addr, size)
        return bytes(mem)
    except gdb.MemoryError as e:
        raise gdb.error(f"Cannot read memory at 0x{addr:016x}: {e}")


def parse_entry(data):
    (offset_low, selector, ist_byte, access,
     offset_middle, offset_high, _res0) = struct.unpack_from("<HHBBHII", data)

    handler = (offset_low
               | (offset_middle << 16)
               | (offset_high << 32))

    present   = (access >> 7) & 1
    dpl       = (access >> 5) & 0x3
    gate_type = access & 0xF
    ist       = ist_byte & 0x7

    return {
        "handler":    handler,
        "selector":   selector,
        "ist":        ist,
        "access":     access,
        "present":    present,
        "dpl":        dpl,
        "gate_type":  gate_type,
    }


def selector_str(sel):
    index = sel >> 3
    ti    = (sel >> 2) & 1   # 0 = GDT, 1 = LDT
    rpl   = sel & 0x3
    table = "LDT" if ti else "GDT"
    return f"0x{sel:04x} ({table}[{index}] RPL={rpl})"


def symbol_for(addr):
    if addr == 0:
        return ""
    try:
        result = gdb.execute(f"info symbol 0x{addr:016x}", to_string=True)
        result = result.strip()
        if "No symbol" in result or not result:
            return ""
        parts = result.split(" in section")
        return parts[0].strip()
    except gdb.error:
        return ""


def print_idt_at(base_addr, count):
    data = read_memory(base_addr, count * ENTRY_SIZE)

    print(f"\nIDT at 0x{base_addr:016x}  ({count} vectors)\n")
    print(f"{'Vec':>3}  {'Name':<38} {'P':1} {'DPL':3} {'Type':<10} {'IST':3}  {'Selector':<26} {'Handler':<18}  Symbol")
    print("─" * 130)

    for vec in range(count):
        entry_data = data[vec * ENTRY_SIZE:(vec + 1) * ENTRY_SIZE]
        e = parse_entry(entry_data)

        if not e["present"] and vec >= 32:
            continue

        name    = EXCEPTION_NAMES.get(vec, f"IRQ {vec - 32}" if vec >= 32 else f"Reserved({vec})")
        present = "Y" if e["present"] else "N"
        gtype   = GATE_TYPES.get(e["gate_type"], f"0x{e['gate_type']:x}    ")
        ist_str = str(e["ist"]) if e["ist"] != 0 else "-"
        sel_str = selector_str(e["selector"])
        sym     = symbol_for(e["handler"]) if e["present"] else ""
        handler = f"0x{e['handler']:016x}" if e["present"] else "               -"

        print(f"{vec:>3}  {name:<38} {present:1} {e['dpl']:>3} {gtype:<10} {ist_str:>3}  {sel_str:<26} {handler}  {sym}")

    print()


class PrintIDT(gdb.Command):
    def __init__(self):
        super().__init__("print_idt", gdb.COMMAND_USER)

    def invoke(self, arg, from_tty):
        argv = arg.split()

        if len(argv) == 0:
            base, count = read_idtr()
            if base is None:
                print("Could not read IDTR automatically.")
                print("Try: print_idt <idt_base_address>")
                print("  or set $idtr_base = <address> first")
                return
        elif len(argv) >= 1:
            try:
                base = int(argv[0], 0)
            except ValueError:
                print(f"Invalid address: {argv[0]}")
                return
            count = MAX_VECTORS
            if len(argv) >= 2:
                try:
                    count = int(argv[1], 0)
                    count = max(1, min(count, MAX_VECTORS))
                except ValueError:
                    print(f"Invalid count: {argv[1]}")
                    return
        else:
            print("Usage: print_idt [address [count]]")
            return

        try:
            print_idt_at(base, count)
        except gdb.error as e:
            print(f"Error: {e}")


PrintIDT()
print("print_idt loaded. Usage: print_idt [address [count]]")