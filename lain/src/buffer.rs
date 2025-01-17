use crate::traits::*;
use crate::types::UnsafeEnum;
use byteorder::{ByteOrder, WriteBytesExt};
use std::io::Write;

/// Default implementation of SerializedSize for slices of items. This runs in O(n) complexity since
/// not all items in the slice are guaranteed to be the same size (e.g. strings)
impl<T> SerializedSize for [T]
where
    T: SerializedSize,
{
    default fn serialized_size(&self) -> usize {
        trace!("using default serialized_size for array");
        if self.is_empty() {
            return 0;
        }

        let size = self
            .iter()
            .map(SerializedSize::serialized_size)
            .fold(0, |sum, i| sum + i);

        size
    }

    fn min_nonzero_elements_size() -> usize {
        T::min_nonzero_elements_size()
    }
}

/// Returns the size of an UnsafeEnum's primitive type
impl<T, P> SerializedSize for UnsafeEnum<T, P> {
    fn serialized_size(&self) -> usize {
        trace!("using serialized size of unsafe enum");
        std::mem::size_of::<P>()
    }

    fn min_nonzero_elements_size() -> usize {
        std::mem::size_of::<P>()
    }
}

impl<T> SerializedSize for Vec<T>
where
    T: SerializedSize,
{
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size for Vec");
        if self.is_empty() {
            trace!("returning 0 since there's no elements");
            return 0;
        }

        let size = self.iter().map(SerializedSize::serialized_size).sum();

        trace!("size is 0x{:02X}", size);

        size
    }

    fn min_nonzero_elements_size() -> usize {
        T::min_nonzero_elements_size()
    }
}

impl SerializedSize for str {
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size of str");
        self.as_bytes().len()
    }

    fn min_nonzero_elements_size() -> usize {
        std::mem::size_of::<char>()
    }
}

impl SerializedSize for String {
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size of String");
        self.as_bytes().len()
    }

    fn min_nonzero_elements_size() -> usize {
        std::mem::size_of::<char>()
    }
}

impl<T> BinarySerialize for Vec<T>
where
    T: BinarySerialize,
{
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        let inner_ref: &[T] = self.as_ref();
        inner_ref.binary_serialize::<_, E>(buffer);
    }
}

impl BinarySerialize for bool {
    #[inline(always)]
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        // unsafe code here for non-binary booleans. i.e. when we do unsafe mutations
        // sometimes a bool is represented as 3 or some other non-0/1 number
        let value = unsafe { *((self as *const bool) as *const u8) };

        buffer.write_u8(value).ok();
    }
}

impl BinarySerialize for i8 {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        buffer.write_i8(*self as i8).ok();
    }
}

impl BinarySerialize for u8 {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        buffer.write_u8(*self as u8).ok();
    }
}

impl BinarySerialize for [u8] {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        buffer.write(&self).ok();
    }
}

impl<T> BinarySerialize for [T]
where
    T: BinarySerialize,
{
    #[inline(always)]
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        for item in self.iter() {
            item.binary_serialize::<W, E>(buffer);
        }
    }
}

impl<T, I> BinarySerialize for UnsafeEnum<T, I>
where
    T: BinarySerialize,
    I: BinarySerialize + Clone,
{
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        match *self {
            UnsafeEnum::Invalid(ref value) => {
                value.binary_serialize::<_, E>(buffer);
            }
            UnsafeEnum::Valid(ref value) => {
                value.binary_serialize::<_, E>(buffer);
            }
        }
    }
}

impl BinarySerialize for String {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        self.as_bytes().binary_serialize::<_, E>(buffer);
    }
}

/// This probably could and should be on a generic impl where T: Deref, but currently
/// this causes a specialization issue since other crates could impl Deref<Target=T> for
/// bool (specifically) in the future. See: https://github.com/rust-lang/rust/issues/45542
impl BinarySerialize for &str {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
        self.as_bytes().binary_serialize::<_, E>(buffer);
    }
}

macro_rules! impl_buffer_pushable {
    ( $($name:ident),* ) => {
        $(
            impl BinarySerialize for $name {
                #[inline(always)]
                fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) {
                    // need to use mashup here to do write_(u8|u16|...) since you can't concat
                    // idents otherwise
                    mashup! {
                        m["method_name"] = write_ $name;
                    }

                    m! {
                        buffer."method_name"::<E>(*self as $name).ok();
                    }
                }
            }
        )*
    }
}

impl_buffer_pushable!(i64, u64, i32, u32, i16, u16, f32, f64);

macro_rules! impl_serialized_size {
    ( $($name:ident),* ) => {
        $(
            impl SerializedSize for $name {
                #[inline(always)]
                fn serialized_size(&self) -> usize {
                    std::mem::size_of::<$name>()
                }

                fn min_nonzero_elements_size() -> usize {
                    std::mem::size_of::<$name>()
                }
            }
        )*
    }
}

impl_serialized_size!(i64, u64, i32, u32, i16, u16, f32, f64, u8, i8, bool);
