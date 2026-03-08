pub const DF_IST: usize = 1;
pub const NMI_IST: usize = 2;

pub const fn ist_index(ist: usize) -> usize {
    ist - 1
}
