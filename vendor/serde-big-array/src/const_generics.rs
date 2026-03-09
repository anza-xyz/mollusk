use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::result;
use serde::de::{Deserialize, Deserializer, Error, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeTuple, Serializer};

pub(crate) struct PartiallyInitialized<T, const N: usize>(
    pub(crate) Option<MaybeUninit<[T; N]>>,
    pub(crate) usize,
);

impl<T, const N: usize> PartiallyInitialized<T, N> {
    #[inline]
    pub(crate) fn new() -> Self {
        PartiallyInitialized(Some(MaybeUninit::uninit()), 0)
    }
}

impl<T, const N: usize> Drop for PartiallyInitialized<T, N> {
    fn drop(&mut self) {
        if !core::mem::needs_drop::<T>() {
            return;
        }
        if let Some(arr) = &mut self.0 {
            while self.1 > 0 {
                self.1 -= 1;
                let offs = self.1;
                let p = (arr.as_mut_ptr() as *mut T).wrapping_add(offs);
                unsafe {
                    core::ptr::drop_in_place::<T>(p);
                }
            }
        }
    }
}

/// The big array serialization helper trait
///
/// ```
/// # use serde_derive::{Serialize, Deserialize};
/// # use serde_big_array::BigArray;
/// #[derive(Serialize, Deserialize)]
/// struct S {
///     #[serde(with = "BigArray")]
///     arr: [u8; 64],
/// }
/// ```
pub trait BigArray<'de, T>: Sized {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize;
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>;
}

impl<'de, T, const N: usize> BigArray<'de, T> for [T; N] {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        let mut seq = serializer.serialize_tuple(self.len())?;
        for elem in &self[..] {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }

    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        struct ArrayVisitor<T> {
            element: PhantomData<T>,
        }

        impl<'de, T, const N: usize> Visitor<'de> for ArrayVisitor<[T; N]>
        where
            T: Deserialize<'de>,
        {
            type Value = [T; N];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an array of length {}", N)
            }

            fn visit_seq<A>(self, mut seq: A) -> result::Result<[T; N], A::Error>
            where
                A: SeqAccess<'de>,
            {
                unsafe {
                    let mut arr = PartiallyInitialized::<T, N>::new();
                    {
                        let p = arr.0.as_mut().unwrap();
                        for i in 0..N {
                            let p = (p.as_mut_ptr() as *mut T).wrapping_add(i);
                            let val = seq
                                .next_element()?
                                .ok_or_else(|| Error::invalid_length(i, &self))?;
                            core::ptr::write(p, val);
                            arr.1 += 1;
                        }
                    }
                    let initialized = arr.0.take().unwrap().assume_init();
                    Ok(initialized)
                }
            }
        }

        let visitor = ArrayVisitor {
            element: PhantomData,
        };
        // The allow is needed to support (32 + 33) like expressions
        #[allow(unused_parens)]
        deserializer.deserialize_tuple(N, visitor)
    }
}
