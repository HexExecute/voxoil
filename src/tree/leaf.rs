use bytemuck::{Pod, Zeroable};

/// A wrapper for leaves in the tree, this contains the primary "data" of the tree (i.e. material of a voxel).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct Leaf<L>(pub L);

impl<L: Pod + Copy + 'static> Leaf<L> {
    /// Returns a new leaf wrapping the given value.
    #[inline]
    pub fn new(value: L) -> Self {
        Leaf(value)
    }

    /// Returns a reference to the value inside the leaf.
    #[inline]
    pub fn get(&self) -> &L {
        &self.0
    }

    /// Returns a mutable reference to the value inside the leaf.
    #[inline]
    pub fn get_mut(&mut self) -> &mut L {
        &mut self.0
    }

    /// Returns the value inside the leaf and consumes the leaf.
    #[inline]
    pub fn into_inner(self) -> L {
        self.0
    }
}
