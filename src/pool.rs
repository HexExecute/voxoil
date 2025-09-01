//! A slab allocator for fixed-size blocks of memory.
use std::{fmt::Debug, mem::MaybeUninit};

pub type Block<T, const SIZE: usize> = [MaybeUninit<T>; SIZE];

/// A slab allocator for fixed-size blocks of memory.
#[repr(C)]
#[derive(Default)]
pub struct Slab<T, const SIZE: usize> {
    pub blocks: Vec<Block<T, SIZE>>,
    free_blocks: Vec<usize>,
}

/// A pool of slabs for different block sizes.
#[repr(C)]
#[derive(Default, Debug)]
pub struct SlabPool<T> {
    pub slabs: (
        Slab<T, 1>,
        Slab<T, 2>,
        Slab<T, 4>,
        Slab<T, 8>,
        Slab<T, 16>,
        Slab<T, 32>,
        Slab<T, 64>,
    ),
}

impl<T, const SIZE: usize> Slab<T, SIZE> {
    /// Returns a new, empty slab.
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            free_blocks: Vec::new(),
        }
    }

    /// Returns a new slab with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            blocks: Vec::with_capacity(capacity),
            free_blocks: Vec::new(),
        }
    }

    /// Allocates a new block in the slab and returns its index.
    pub fn alloc(&mut self) -> usize {
        if let Some(index) = self.free_blocks.pop() {
            index
        } else {
            let index = self.blocks.len();
            self.blocks.push([const { MaybeUninit::uninit() }; SIZE]);
            index
        }
    }

    /// Allocates a new block in the slab and returns a mutable reference to it.
    pub fn alloc_mut(&mut self) -> &mut Block<T, SIZE> {
        let index = self.alloc();
        &mut self.blocks[index]
    }

    /// Pushes a block into the slab and returns its index.
    pub fn push(&mut self, block: Block<T, SIZE>) -> usize {
        if let Some(index) = self.free_blocks.pop() {
            self.blocks[index] = block;
            index
        } else {
            let index = self.blocks.len();
            self.blocks.push(block);
            index
        }
    }

    /// Frees a block at the given index.
    pub fn free(&mut self, index: usize) {
        self.free_blocks.push(index);
    }

    /// Returns a reference to the block at the given index.
    pub fn get(&self, index: usize) -> Option<&Block<T, SIZE>> {
        self.blocks.get(index)
    }

    /// Returns a mutable reference to the block at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Block<T, SIZE>> {
        self.blocks.get_mut(index)
    }
}

impl<T> SlabPool<T> {
    /// Returns a new, empty slab pool.
    pub fn new() -> Self {
        Self {
            slabs: (
                Slab::new(),
                Slab::new(),
                Slab::new(),
                Slab::new(),
                Slab::new(),
                Slab::new(),
                Slab::new(),
            ),
        }
    }

    /// Returns a new slab pool with the given capacities.
    pub fn with_capacities(capacities: (usize, usize, usize, usize, usize, usize, usize)) -> Self {
        Self {
            slabs: (
                Slab::with_capacity(capacities.0),
                Slab::with_capacity(capacities.1),
                Slab::with_capacity(capacities.2),
                Slab::with_capacity(capacities.3),
                Slab::with_capacity(capacities.4),
                Slab::with_capacity(capacities.5),
                Slab::with_capacity(capacities.6),
            ),
        }
    }

    /// Returns a new slab pool with the given capacity for all slabs.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacities((
            capacity, capacity, capacity, capacity, capacity, capacity, capacity,
        ))
    }
}

impl<T, const SIZE: usize> Debug for Slab<T, SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Slab {{ capacity: {}, blocks: ", self.blocks.len())?;

        if self.blocks.is_empty() {
            return write!(f, "[] }}");
        }

        let mut block_states = vec!['A'; self.blocks.len()];
        for &free_index in &self.free_blocks {
            if free_index < block_states.len() {
                block_states[free_index] = 'F';
            }
        }

        for state in block_states {
            write!(f, "[{}]", state)?;
        }

        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_slab_debug_output() {
        let mut slab = Slab::<u8, 1>::new();

        // Allocate 4 blocks
        slab.alloc();
        slab.alloc();
        slab.alloc();
        slab.alloc();

        // Free blocks at index 1 and 3
        slab.free(1);
        slab.free(3);

        let debug_output = format!("{:?}", slab);
        assert_eq!(debug_output, "Slab { capacity: 4, blocks: [A][F][A][F] }");
    }
}
