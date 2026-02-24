use BlockType;

use std::cmp::min;
use std::ptr;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct Inner<Block>(Option<Box<[Block]>>);
// Invariant: self.invariant()

#[cfg(feature = "serde")]
impl<'de, Block: BlockType + serde::Deserialize<'de>> serde::Deserialize<'de> for Inner<Block> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Unchecked<Block>(Option<Box<[Block]>>);
        let unchecked = Unchecked::deserialize(deserializer)?;
        let unchecked = Inner(unchecked.0);
        if !unchecked.invariant() {
            return Err(serde::de::Error::custom("Invalid Inner"));
        }
        Ok(unchecked)
    }
}

impl<Block: BlockType> Inner<Block> {
    #[allow(dead_code)]
    fn invariant(&self) -> bool {
        match self.0 {
            Some(ref b) => b.len() > 0,
            None        => true,
        }
    }

    pub fn new(init: Block, nblocks: usize) -> Self {
        Inner(if nblocks == 0 {
            None
        } else {
            Some(vec![init; nblocks].into_boxed_slice())
        })
    }

    // Precondition: The first `len` blocks of `self` are initialized an
    // in-bounds.
    pub unsafe fn clone_resize(&self, len: usize, new_cap: usize) -> Self {
        if new_cap == 0 {
            return Inner(None);
        }

        let mut result = vec![Block::zero(); new_cap].into_boxed_slice();

        for i in 0 .. min(len, new_cap) {
            result[i] = self.get_block(i);
        }

        Inner(Some(result))
    }

    pub fn len(&self) -> usize {
        match self.0 {
            Some(ref b) => b.len(),
            None        => 0,
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn into_boxed_slice(self) -> Box<[Block]> {
        self.0.unwrap_or_else(<Box<[Block]>>::default)
    }

    pub fn as_ptr(&self) -> *const Block {
        match self.0 {
            Some(ref b) => b.as_ptr(),
            None        => ptr::null(),
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut Block {
        match self.0 {
            Some(ref mut b) => b.as_mut_ptr(),
            None            => ptr::null_mut(),
        }
    }

    // Precondition: `index` is in bounds. This implies that `self.0.is_some()`.
    pub unsafe fn get_block(&self, index: usize) -> Block {
        // Weirdly, `.unwrap()` is consistently faster than
        // `.unwrap_or_else(ptr::null)`, or calls to `unreachable()`.
        let ptr = self.0.as_ref().unwrap().as_ptr();
        ptr::read(ptr.offset(index as isize))
    }

    // Precondition: `index` is in bounds. This implies that `self.0.is_some()`.
    pub unsafe fn set_block(&mut self, index: usize, value: Block) {
        let ptr = self.0.as_mut().unwrap().as_mut_ptr();
        ptr::write(ptr.offset(index as isize), value);
    }
}

impl<Block: BlockType> From<Box<[Block]>> for Inner<Block> {
    fn from(bb: Box<[Block]>) -> Self {
        Inner(Some(bb))
    }
}
