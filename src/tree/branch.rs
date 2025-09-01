use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bytemuck::{Pod, Zeroable};
use glam::UVec3;

use crate::{
    tree::traversal::ChildSlot,
    util::{CoordinatePackError, coordinate_to_slot},
};

/// A 64-bit address type used in each Branch.
///
/// An absolute address, it identifies a leaf by its 62-bit index, plus flags indicating end/fork and solidity.
///
/// Bit layout (most significant to least significant):
/// - Bit 63: `is_end` flag - when set, this address points only to leaves, otherwise, to branches.
/// - Bit 62: `is_solid` flag - when set, indicates the branch is solid, meaning all bitmask 1s point to the same branch/leaf.
/// - Bits 0..=61: `index` - a 62-bit index to either memory arena.
///
/// Internally stored as a `u64`.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Zeroable, Pod)]
pub struct Address(pub u64);

/// A 64-bit bitmask representing active children of a Branch.
///
/// Each bit corresponds to one of 64 possible children.  
/// A set bit indicates the presence of a child at that position.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Zeroable, Pod)]
pub struct ChildMask(pub u64);

/// A branch in the sparse 64-tree structure.
///
/// Contains:
/// - `address`: an absolute address.
/// - `bitmask`: a mask indicating which of the 64 possible children exist.
///
/// A **fork** branch contains only other branches (non-leaf nodes).  
/// An **end** branch contains leaves (final nodes).
///
/// A **solid** branch contains only one child and all set bits in the bitmask correspond to the same child.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Zeroable, Pod)]
pub struct Branch {
    pub address: Address,
    pub bitmask: ChildMask,
}

const INDEX_MASK: u64 = 0x3FFFFFFFFFFFFFFF;
const END_FLAG_MASK: u64 = 1 << 63;
const SOLID_FLAG_MASK: u64 = 1 << 62;

impl Address {
    /// Returns the 62-bit index of the address value.
    #[inline]
    pub fn index(&self) -> usize {
        (self.0 & INDEX_MASK) as usize
    }

    /// Returns `true` if this address is an "end," meaning it points only to *leaves*.
    #[inline]
    pub fn is_end(&self) -> bool {
        self.0 & END_FLAG_MASK != 0
    }

    /// Returns `true` if this address is a "fork," meaning it points only to *branches*.
    #[inline]
    pub fn is_fork(&self) -> bool {
        !self.is_end()
    }

    /// Returns `true` if this address is "solid," meaning all set bits in the bitmask point to *one* distinct child.
    #[inline]
    pub fn is_solid(&self) -> bool {
        self.0 & SOLID_FLAG_MASK != 0
    }

    /// Returns `true` if this address is "composite," meaning set bits in the bitmask point to *many* distinct children.
    #[inline]
    pub fn is_composite(&self) -> bool {
        !self.is_solid()
    }
}

impl ChildMask {
    /// Gets the bit for the given child coordinate in the mask.
    pub fn get(&self, coordinate: UVec3) -> Result<bool, CoordinatePackError> {
        let slot = coordinate_to_slot(coordinate)?;
        Ok(self.0 & (1 << *slot) != 0)
    }

    /// Calculates the dense index of a child slot.
    ///
    /// This counts the number of set bits (children) before this slot
    /// to find the actual index into the child array.
    #[inline]
    pub fn dense_index_of(&self, slot: ChildSlot) -> usize {
        // Create a mask for all bits before the slot, then count them.
        let mask_before = (1u64 << *slot) - 1;
        (self.0 & mask_before).count_ones() as usize
    }

    /// Sets the bit for the given child coordinate in the mask.
    pub fn set(&mut self, coordinate: UVec3) -> Result<(), CoordinatePackError> {
        let slot = coordinate_to_slot(coordinate)?;
        self.0 |= 1 << *slot;
        Ok(())
    }

    /// Unsets the bit for the given child coordinate in the mask.
    pub fn unset(&mut self, coordinate: UVec3) -> Result<(), CoordinatePackError> {
        let slot = coordinate_to_slot(coordinate)?;
        self.0 &= !(1 << *slot);
        Ok(())
    }

    /// Returns `true` if no bits are set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns the number of set bits.
    #[inline]
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Returns the first child index (lowest set bit).
    pub fn first_child_index(&self) -> Option<u32> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.trailing_zeros())
        }
    }

    /// Returns the last child index (highest set bit).
    pub fn last_child_index(&self) -> Option<u32> {
        if self.is_empty() {
            None
        } else {
            Some(63 - self.0.leading_zeros())
        }
    }
}

impl Branch {
    /// Returns `true` if its address is an "end," meaning it points only to *leaves*.
    #[inline]
    pub fn is_end(&self) -> bool {
        self.address.is_end()
    }

    /// Returns `true` if its address is a "fork," meaning it points only to *branches*.
    #[inline]
    pub fn is_fork(&self) -> bool {
        self.address.is_fork()
    }

    /// Returns `true` if its address is "solid," meaning all set bits in the bitmask point to *one* distinct child.
    #[inline]
    pub fn is_solid(&self) -> bool {
        self.address.is_solid()
    }

    /// Returns `true` if its address is "composite," meaning set bits in the bitmask point to *many* distinct children.
    #[inline]
    pub fn is_composite(&self) -> bool {
        self.address.is_composite()
    }

    /// Returns `true` if this branch has no children.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bitmask.is_empty()
    }
}

impl Deref for ChildMask {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChildMask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Address")
            .field("raw", &format_args!("0x{:016X}", self.0))
            .field("is_end", &self.is_end())
            .field("is_solid", &self.is_solid())
            .field("index", &format_args!("0x{:016X}", self.index()))
            .finish()
    }
}

impl Debug for ChildMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw = self.0;

        f.debug_struct("ChildMask")
            .field("raw", &format_args!("0x{:016X}", raw))
            .field("count", &raw.count_ones())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_address_flags() {
        let end_address = Address(1 << 63);
        assert!(end_address.is_end());
        assert!(!end_address.is_fork());

        let fork_address = Address(0);
        assert!(fork_address.is_fork());
        assert!(!fork_address.is_end());

        let solid_address = Address(1 << 62);
        assert!(solid_address.is_solid());
        assert!(!solid_address.is_composite());

        let composite_address = Address(0);
        assert!(composite_address.is_composite());
        assert!(!composite_address.is_solid());
    }

    #[test]
    fn branch_childmask_operations() {
        let mut mask = ChildMask::default();
        assert!(mask.is_empty());
        assert_eq!(mask.count(), 0);
        assert_eq!(mask.first_child_index(), None);
        assert_eq!(mask.last_child_index(), None);

        let coord1 = UVec3::new(1, 1, 1);
        let coord2 = UVec3::new(2, 2, 2);
        let coord3 = UVec3::new(3, 3, 3);

        // Set a bit
        mask.set(coord1).unwrap();
        assert!(!mask.is_empty());
        assert_eq!(mask.count(), 1);
        assert!(mask.get(coord1).unwrap());
        assert!(!mask.get(coord2).unwrap());

        // Set another bit
        mask.set(coord2).unwrap();
        assert_eq!(mask.count(), 2);
        assert!(mask.get(coord2).unwrap());

        // Test dense index
        let slot1 = coordinate_to_slot(coord1).unwrap();
        let slot2 = coordinate_to_slot(coord2).unwrap();
        assert_eq!(mask.dense_index_of(slot1), 0);
        assert_eq!(mask.dense_index_of(slot2), 1);

        // Unset a bit
        mask.unset(coord1).unwrap();
        assert_eq!(mask.count(), 1);
        assert!(!mask.get(coord1).unwrap());
        assert_eq!(mask.dense_index_of(slot2), 0);

        // Test first/last child
        mask.set(coord3).unwrap();
        let slot3 = coordinate_to_slot(coord3).unwrap();
        assert_eq!(mask.first_child_index(), Some(slot2.get() as u32));
        assert_eq!(mask.last_child_index(), Some(slot3.get() as u32));
    }
}
