def bits(x, hi, lo):
    return (x >> lo) & ((1 << (hi - lo + 1)) - 1)