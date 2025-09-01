use glam::UVec3;

use crate::tree::{Depth, traversal::ChildSlot};

#[derive(Debug)]
pub enum CoordinatePackError {
    OutOfRange { axis: char, value: u32 },
}

/// Packs a 3D coordinate into a 6-bit child slot (0..=63).
///
/// Each component (`x`, `y`, `z`) must be in the range `[0..=3]`
///
/// # Examples
///
/// ```rs
/// let coordinate = glam::UVec3::new(1, 2, 3);
/// assert_eq!(pack_coordinate(coordinate).unwrap(), 0b11_10_01);
/// ```
pub fn coordinate_to_slot(coordinate: UVec3) -> Result<ChildSlot, CoordinatePackError> {
    if coordinate.x > 0b11 {
        return Err(CoordinatePackError::OutOfRange {
            axis: 'x',
            value: coordinate.x,
        });
    }
    if coordinate.y > 0b11 {
        return Err(CoordinatePackError::OutOfRange {
            axis: 'y',
            value: coordinate.y,
        });
    }
    if coordinate.z > 0b11 {
        return Err(CoordinatePackError::OutOfRange {
            axis: 'z',
            value: coordinate.z,
        });
    }

    Ok(ChildSlot::new(((coordinate.z << 4) | (coordinate.y << 2) | coordinate.x) as u8).unwrap())
}

pub fn localize_coordinate(coordinate: UVec3, depth: Depth, max_depth: Depth) -> UVec3 {
    coordinate.map(|x| {
        let index = (*max_depth << 1) - (*depth << 1);
        let mask = 0b11 << index;
        (x & mask) >> index
    })
}

/// A macro to dynamically select and operate on a slab from a SlabPool by the number of children.
///
/// This macro takes a SlabPool, the count of children (e.g., from a bitmask), and a closure.
/// It uses match ranges to find the correct power-of-two-sized slab that can hold the children
/// and executes the closure with the correctly-typed slab.
///
/// # Panics
/// This macro will panic if the count is greater than 64.
/// The caller is responsible for handling the `count == 0` case, which represents an empty
/// branch and does not correspond to any slab.
macro_rules! with_slab_by_count {
    ($pool:expr, $count:expr, |$slab:ident| $body:expr) => {
        match $count {
            1 => {
                let $slab = &$pool.slabs.0;
                $body
            }
            2 => {
                let $slab = &$pool.slabs.1;
                $body
            }
            3..=4 => {
                let $slab = &$pool.slabs.2;
                $body
            }
            5..=8 => {
                let $slab = &$pool.slabs.3;
                $body
            }
            9..=16 => {
                let $slab = &$pool.slabs.4;
                $body
            }
            17..=32 => {
                let $slab = &$pool.slabs.5;
                $body
            }
            33..=64 => {
                let $slab = &$pool.slabs.6;
                $body
            }
            _ => panic!("Invalid slab count provided. Must be between 1 and 64."),
        }
    };
}

/// A macro to dynamically select and mutably operate on a slab from a SlabPool by the number of children.
///
/// This is the mutable version of `with_slab_by_count`.
///
/// # Panics
/// This macro will panic if the count is greater than 64.
/// The caller is responsible for handling the `count == 0` case.
macro_rules! with_slab_by_count_mut {
    ($pool:expr, $count:expr, |$slab:ident| $body:expr) => {
        match $count {
            1 => {
                let $slab = &mut $pool.slabs.0;
                $body
            }
            2 => {
                let $slab = &mut $pool.slabs.1;
                $body
            }
            3..=4 => {
                let $slab = &mut $pool.slabs.2;
                $body
            }
            5..=8 => {
                let $slab = &mut $pool.slabs.3;
                $body
            }
            9..=16 => {
                let $slab = &mut $pool.slabs.4;
                $body
            }
            17..=32 => {
                let $slab = &mut $pool.slabs.5;
                $body
            }
            33..=64 => {
                let $slab = &mut $pool.slabs.6;
                $body
            }
            _ => panic!("Invalid slab count provided. Must be between 1 and 64."),
        }
    };
}

#[doc(hidden)]
macro_rules! _push_to_slab {
    ($slab:expr, $size:expr, $data:expr) => {{
        let mut block = [const { std::mem::MaybeUninit::uninit() }; $size];
        for (i, item) in $data.into_iter().enumerate() {
            block[i] = std::mem::MaybeUninit::new(item);
        }
        $slab.push(block)
    }};
}

/// A macro to push dynamically sized data to a [`SlabPool`].
///
/// This macro selects the appropriate slab from a pool based on the data count,
/// creates a fixed-size block, fills it with the provided data, and pushes it to the slab.
/// It returns the index of the new block in the slab.
///
/// # Panics
/// This macro will panic if the count is 0 or greater than 64.
macro_rules! push_to_pool {
    ($pool:expr, $data:expr) => {
        match $data.len() {
            1 => _push_to_slab!(&mut $pool.slabs.0, 1, $data),
            2 => _push_to_slab!(&mut $pool.slabs.1, 2, $data),
            3..=4 => _push_to_slab!(&mut $pool.slabs.2, 4, $data),
            5..=8 => _push_to_slab!(&mut $pool.slabs.3, 8, $data),
            9..=16 => _push_to_slab!(&mut $pool.slabs.4, 16, $data),
            17..=32 => _push_to_slab!(&mut $pool.slabs.5, 32, $data),
            33..=64 => _push_to_slab!(&mut $pool.slabs.6, 64, $data),
            _ => panic!("Invalid slab count provided. Must be between 1 and 64."),
        }
    };
}

pub(crate) use _push_to_slab;
pub(crate) use push_to_pool;
pub(crate) use with_slab_by_count;
pub(crate) use with_slab_by_count_mut;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::SlabPool;

    #[test]
    fn util_coordinate_to_slot() {
        let coord = UVec3::new(1, 2, 3);
        assert_eq!(*coordinate_to_slot(coord).unwrap(), 0b11_10_01);

        let oob_x = UVec3::new(4, 0, 0);
        assert!(coordinate_to_slot(oob_x).is_err());

        let oob_y = UVec3::new(0, 4, 0);
        assert!(coordinate_to_slot(oob_y).is_err());

        let oob_z = UVec3::new(0, 0, 4);
        assert!(coordinate_to_slot(oob_z).is_err());
    }

    #[test]
    fn util_localize_coordinate() {
        let coordinate = UVec3::new(13, 1, 0);
        let max_depth = Depth(1);

        let l0 = localize_coordinate(coordinate, Depth(0), max_depth);
        assert_eq!(l0, UVec3::new(3, 0, 0));

        let l1 = localize_coordinate(coordinate, Depth(1), max_depth);
        assert_eq!(l1, UVec3::new(1, 1, 0));
    }

    #[test]
    fn util_push_to_slab() {
        let mut pool = SlabPool::<u8>::new();
        let data = vec![10, 20, 30];
        let data_len = data.len();

        let slab_index = push_to_pool!(&mut pool, data);

        // Data with length 3 should go into the slab for blocks of size 4.
        let slab = &pool.slabs.2;
        assert_eq!(slab_index, 0);

        let block = slab.get(slab_index).unwrap();

        // Verify the data was written correctly
        for i in 0..data_len {
            assert_eq!(unsafe { *block[i].as_ptr() }, (i + 1) as u8 * 10);
        }
    }
}
