//! This crate implements a simple 'first-fit-decreasing' strategy for 2D bin-packing.
//!
//! See <https://en.wikipedia.org/wiki/Bin_packing_problem> .
//!
//! Example usage:
//!
//! ```rust
//!
//! use bin_packing_2d::{Bin, Item};
//! let items = [
//!     Item {
//!         w: 10,
//!         h: 3,
//!         id: 'D'
//!     },
//!     Item {
//!         w: 10,
//!         h: 3,
//!         id: 'A'
//!     },
//!     Item {
//!         w: 10,
//!         h: 3,
//!         id: 'B'
//!     },
//!     Item {
//!         w: 1,
//!         h: 10,
//!         id: 'C'
//!     },
//! ];
//!
//! let mut bin = Bin::new(10, 10);   // Create 10x10 bin
//! let all_fit: bool = bin.place_all(items.into_iter()); // Pack all the items
//!
//! println!("All items placed: {:?}", all_fit);
//! println!("Solution: {:#?}", bin.solution());
//!
//! ```
//!
//! The above program prints:
//! ```text
//! All items placed: true
//! Solution: [
//!     PlacedItem {
//!         x0: 0,
//!         y0: 0,
//!         x1: 10,
//!         y1: 3,
//!         rotated: false,
//!         id: 'D',
//!     },
//!     PlacedItem {
//!         x0: 0,
//!         y0: 3,
//!         x1: 10,
//!         y1: 6,
//!         rotated: false,
//!         id: 'A',
//!     },
//!     PlacedItem {
//!         x0: 0,
//!         y0: 6,
//!         x1: 10,
//!         y1: 9,
//!         rotated: false,
//!         id: 'B',
//!     },
//!     PlacedItem {
//!         x0: 0,
//!         y0: 9,
//!         x1: 10,
//!         y1: 10,
//!         rotated: true,
//!         id: 'C',
//!     },
//! ]
//! ```
//!
//!
//!
#![deny(missing_docs)]
#![deny(warnings)]
use std::cmp::Reverse;
use bit_vec::BitVec;


struct Bitmap2d {
    width: usize,
    height: usize,
    bits: BitVec,
}

impl Bitmap2d {
    fn new(width: usize, height: usize) -> Bitmap2d {
        if width < 1 || height < 1 {
            panic!("Width and height must both be > 0");
        }
        Bitmap2d {
            width,
            height,
            bits: BitVec::from_elem(width*height, false)
        }
    }
    fn get(&self, x: usize, y: usize) -> bool {
        self.bits[y*self.width + x]
    }
    fn set(&mut self, x: usize, y: usize, value: bool) {
        self.bits.set(y*self.width + x, value)
    }
}

/// An item that is to be packed.
/// Note that the item might be rotated 90 degrees when placed
pub struct Item<I> {
    /// Width of item
    /// Note that the item might be rotated 90 degrees when placed
    pub w: usize,
    /// Height of item
    /// Note that the item might be rotated 90 degrees when placed
    pub h: usize,
    /// An id for the item.
    /// This is not interpreted by this library, but can be useful to keep
    /// track of items.
    pub id: I
}

impl<I> Item<I> {
    fn size(&self) -> usize {
        self.w.max(self.h)
    }
}

/// A placed item. This contains information on where an item was placed.
/// The item is placed at the coordinate 'x0,y0', and extends to (but not including)
/// 'x1,y1'.
///
/// The set of coordinates covered by the object is thus:
///
/// Horizontally: `x0..x1`
/// Vertically: `y0..y1`
#[derive(Debug)]
pub struct PlacedItem<I> {
    /// The horizontal coordinate for the leftmost edge of the item.
    pub x0: usize,
    /// The vertical coordinate for the top edge of the item.
    pub y0: usize,
    /// One past the rightmost edge of the item, horizontally.
    pub x1: usize,
    /// One past the bottom edge of the item, horizontally.
    pub y1: usize,
    /// True if the object was rotated 90 degrees to fit
    pub rotated: bool,
    /// The user-supplied id of the object.
    pub id: I
}

impl<I> PlacedItem<I> {
    /// Check if the item, when placed in the rotation chosen,
    /// contains the given point
    pub fn contains(&self, pos: (usize,usize)) -> bool {
        let (x,y) = pos;
        x >= self.x0 && x < self.x1 &&
            y >= self.y0 && y < self.y1
    }
}

/// A bin into which objects are to be packed.
pub struct Bin<I> {
    bitmap: Bitmap2d,
    items: Vec<PlacedItem<I>>
}

impl<I> Bin<I> {

    /// Return the set of placed objects.
    /// Note: If some objects couldn't fit, this slice will have less elements
    /// than the user attmpted to place.
    /// This library does not generate optimal solutions.
    pub fn solution(&self) -> &[PlacedItem<I>] {
        &self.items
    }

    /// Create a new bin width the given horizontal width and vertical height.
    pub fn new(width: usize, height: usize) -> Bin<I> {
        Bin {
            bitmap: Bitmap2d::new(width,height),
            items: vec![]
        }
    }
    /// Place all the items given by the iterator 'items'.
    /// Returns true if all items could be placed.
    /// The solution can be retrieved by calling the 'solution'-method.
    /// Note that this library does not in general produce optimal solutions.
    pub fn place_all(&mut self, items: impl Iterator<Item=Item<I>>) -> bool {
        let mut items : Vec<Item<I>> = items.collect();
        items.sort_by_key(|x|Reverse(x.size()));
        let mut all_fit = true;
        for item in items {
            if !self.add_to_best_fit(item) {
                all_fit = false;
            }
        }
        all_fit
    }

    fn place(&mut self, x0: usize, y0:usize, item: Item<I>, rotated: bool) {
        let w = if rotated {item.h} else {item.w};
        let h = if rotated {item.w} else {item.h};
        for y in y0..y0+h {
            for x in x0..x0+w {
                self.bitmap.set(x,y,true);

            }
        }
        self.items.push(PlacedItem{
            x0,
            y0,
            x1: x0+w,
            y1: y0+h,
            rotated,
            id: item.id,
        });
    }
    fn evaluate_fit(&self, x0: usize, y0: usize, w: usize, h: usize) -> Option<usize> {
        if x0 + w > self.bitmap.width || y0 + h > self.bitmap.height {
            return None;
        }
        for y in y0..y0+h {
            for x in x0..x0+w {
                if self.bitmap.get(x, y) {
                    return None; //No fit
                }
            }
        }

        let mut points = 0;
        for y in y0..y0+h {
            if x0 > 0 && !self.bitmap.get(x0-1,y) { points += 1}
            if x0+w < self.bitmap.width && !self.bitmap.get(x0+w,y) { points += 1}
        }

        for x in x0..x0+w {
            if y0 > 0 && !self.bitmap.get(x,y0-1) { points += 1}
            if y0 + h < self.bitmap.height && !self.bitmap.get(x, y0+h) { points += 1}
        }

        Some(points)
    }
    fn add_to_best_fit(&mut self, item: Item<I>) -> bool {
        if item.w > self.bitmap.width && item.h > self.bitmap.height {
            return false; //Impossible to fit.
        }
        let mut cur_best_fit = usize::MAX;
        let smallest_dim = item.h.min(item.w);
        let mut best_fit = None;
        for y in 0..=self.bitmap.height-smallest_dim {
            let mut had_busy = false;
            for x in 0..=self.bitmap.width-smallest_dim {
                if self.bitmap.get(x, y) {
                   had_busy = true;
                }
                if let Some(fit) = self.evaluate_fit(x,y,item.w,item.h) {
                    if fit < cur_best_fit {
                        cur_best_fit = fit;
                        best_fit = Some((x,y,false));
                    }
                }
                if let Some(fit) = self.evaluate_fit(x,y,item.h,item.w) { //Rotated
                    if fit < cur_best_fit {
                        cur_best_fit = fit;
                        best_fit = Some((x,y,true));
                    }
                }
            }
            if !had_busy && best_fit.is_some() {
                break;
            }
        }
        if let Some((fit_x,fit_y,rotated)) = best_fit {
            self.place(fit_x,fit_y, item, rotated);
            true
        } else {
            false
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

        let items = [
            Item {
                w: 10,
                h: 3,
                id: 'D'
            },
            Item {
                w: 10,
                h: 3,
                id: 'A'
            },
            Item {
                w: 10,
                h: 3,
                id: 'B'
            },
            Item {
                w: 1,
                h: 10,
                id: 'C'
            },
        ];
        let mut bin = Bin::new(10,10);
        let all_fit = bin.place_all(items.into_iter());
        println!("All items placed: {:?}", all_fit);
        println!("Solution: {:#?}", bin.solution());
        let places = bin.solution();
        for row in 0..10 {
            for col in 0..10 {
                let id = places.iter().find(|x|x.contains((col,row)));
                let c = id.map(|x|x.id).unwrap_or(' ');
                assert_ne!(c, ' ');
                print!("{}", c);
            }
            println!("|");
        }
    }
}
