use num_traits::PrimInt;

#[inline]
pub fn extract<T: PrimInt>(val: T, mask: T) -> T {
    (val & mask) >> mask.trailing_zeros() as usize
}

#[inline]
pub const fn bit(x: u8, i: u8) -> u8 {
    (x & (1 << i)) >> i
}
