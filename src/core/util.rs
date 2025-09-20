use num_traits::PrimInt;

pub fn extract<T: PrimInt>(val: T, mask: T) -> T {
    (val & mask) >> mask.trailing_zeros() as usize
}
