use core::num::NonZeroU8;

pub const DF_IST: NonZeroU8 = NonZeroU8::new(1).unwrap();
pub const NMI_IST: NonZeroU8 = NonZeroU8::new(2).unwrap();

pub const fn ist_index(ist: NonZeroU8) -> usize {
    (ist.get() - 1) as usize
}
