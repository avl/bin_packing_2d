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

use std::cell::RefCell;
use std::cmp::Reverse;
use bit_vec::BitVec;


struct Bitmap2d {
    width: usize,
    height: usize,
    bits: BitVec,
}

impl Bitmap2d {
    fn clear(&mut self) {
        self.bits.clear();
    }
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
#[derive(PartialEq,Eq,Debug,Hash,Clone)]
pub struct Item<I> {
    /// Width of item
    /// Note that the item might be rotated 90 degrees when placed
    pub w: usize,
    /// Height of item
    /// Note that the item might be rotated 90 degrees when placed
    pub h: usize,
    /// Item can be rotated
    pub allow_rotate: bool,
    /// An id for the item.
    /// This is not interpreted by this library, but can be useful to keep
    /// track of items.
    pub id: I,
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
#[derive(Debug,Clone)]
pub struct PlacedItem<I:Clone> {
    /// The horizontal coordinate for the leftmost edge of the item.
    pub x0: usize,
    /// The vertical coordinate for the top edge of the item.
    pub y0: usize,
    /// One past the rightmost edge of the item, horizontally.
    /// This takes 'rotation' into account.
    pub x1: usize,
    /// One past the bottom edge of the item, horizontally.
    /// This takes 'rotation' into account.
    pub y1: usize,
    /// True if the object was rotated 90 degrees to fit
    pub rotated: bool,
    /// The user-supplied id of the object.
    pub id: I
}

impl<I:Clone> PlacedItem<I> {
    /// Check if the item, when placed in the rotation chosen,
    /// contains the given point
    pub fn contains(&self, pos: (usize,usize)) -> bool {
        let (x,y) = pos;
        x >= self.x0 && x < self.x1 &&
            y >= self.y0 && y < self.y1
    }
}
///A free, unused area
#[derive(Debug,Clone,Copy)]
pub struct Hole {
    /// Width of the area
    pub width: usize,
    /// Height of the area
    pub height: usize,
}

/// A bin into which objects are to be packed.
pub struct Bin<I:Clone> {
    bitmap: Bitmap2d,
    items: Vec<PlacedItem<I>>,
    largest_hole: Hole,
    metric: fn(Hole)->usize,
}

/// Constraints on placing
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Strategy {
    /// Rotate the item 90 degrees
    Rotate,
    /// Do not rotate 90 degrees
    DoNotRotate,
    /// Rotate if this gives a better fit
    RotateIfSuitable
}

#[derive(Clone,Copy)]
struct Rect {
    x0: usize,
    y0: usize,
    x1: usize, //Inclusive
    y1: usize, //Inclusive
}

impl Hole {
    fn default_area(&self) -> usize {
        self.width * self.height
    }
}

impl Rect {

    fn hole(&self) -> Hole {
        Hole {
            width: (self.x1-self.x0+1),
            height: (self.y1-self.y0 +1 )
        }
    }
    fn is_obstructed(&self, bitmap: &Bitmap2d) -> bool {
        for y in self.y0..=self.y1 {
            for x in self.x0..=self.x1 {
                if bitmap.get(x,y) {
                    return true;
                }
            }
        }
        false
    }
    fn top_neighbors(&self) -> Option<Rect> {
        if self.y0 == 0 {
            return None;
        }
        Some(Rect {
            x0: self.x0,
            y0: self.y0-1,
            x1: self.x1,
            y1: self.y0-1,
        })
    }
    fn bottom_neighbors(&self, bin_height: usize) -> Option<Rect> {
        if self.y1 + 1 == bin_height {
            return None;
        }
        Some(Rect {
            x0: self.x0,
            y0: self.y1+1,
            x1: self.x1,
            y1: self.y1+1,
        })
    }
    fn right_neighbors(&self, bin_width: usize) -> Option<Rect> {
        if self.x1 + 1 == bin_width {
            return None;
        }
        Some(Rect {
            x0: self.x1+1,
            y0: self.y0,
            x1: self.x1+1,
            y1: self.y1,
        })
    }
    fn left_neighbors(&self) -> Option<Rect> {
        if self.x0 == 0 {
            return None;
        }
        Some(Rect {
            x0: self.x0-1,
            y0: self.y0,
            x1: self.x0-1,
            y1: self.y1,
        })
    }
    fn grow_left(mut self) -> Self {
        self.x0 -= 1;
        self
    }
    fn grow_right(mut self) -> Self {
        self.x1 += 1;
        self
    }
    fn grow_up(mut self) -> Self {
        self.y0 -= 1;
        self
    }
    fn grow_down(mut self) -> Self {
        self.y1 += 1;
        self
    }
}


impl<I:Clone> Bin<I> {

    /// The width of the bin. This is always just the value
    /// that was provided in the 'new' call.
    pub fn width(&self) -> usize {
        self.bitmap.width
    }

    /// The height of the bin. This is always just the value
    /// that was provided in the 'new' call.
    pub fn height(&self) -> usize {
        self.bitmap.height
    }

    /// Return the set of placed objects.
    /// Note: If some objects couldn't fit, this slice will have less elements
    /// than the user attmpted to place.
    /// This library does not generate optimal solutions.
    pub fn solution(&self) -> &[PlacedItem<I>] {
        &self.items
    }
    /// Return the set of placed objects.
    /// Note: If some objects couldn't fit, this slice will have less elements
    /// than the user attmpted to place.
    /// This library does not generate optimal solutions.
    pub fn take_solution(self) -> Vec<PlacedItem<I>> {
        self.items
    }

    /// Create a new bin width the given horizontal width and vertical height.
    pub fn new(width: usize, height: usize) -> Bin<I> {
        Bin {
            bitmap: Bitmap2d::new(width,height),
            items: vec![],
            largest_hole: Hole {
                width, height
            },
            metric: |hole|hole.default_area(),
        }
    }

    fn calculate_largest_hole(&self) -> Hole {
        let offshore_map = RefCell::new(vec![]);
        for bit in self.bitmap.bits.iter() {
            offshore_map.borrow_mut().push(if bit {0} else {u32::MAX});
        }

        let get = |x:isize,y:isize| -> Option<u32>{
            let cx = x as usize;
            let cy = y as usize;
            if cx < self.bitmap.width && cy< self.bitmap.height {
                return Some(offshore_map.borrow()[cy*self.bitmap.width+cx]);
            }
            None
        };
        let set = |x:usize,y:usize,val:u32| {
            debug_assert!(x < self.bitmap.width && y< self.bitmap.height);
            offshore_map.borrow_mut()[y*self.bitmap.width+x] = val;
        };


        let mut dist = 0;
        loop {
            let mut no_progress = true;
            let nextdist= dist + 1 ;
            for y in 0..self.bitmap.height {
                for x in 0..self.bitmap.width {
                    if get(x as isize,y as isize).unwrap() != u32::MAX {
                        continue;
                    }
                    'inner: for dy in -1..=1 {
                        for dx in -1..=1 {
                            let cy = y as isize + dy;
                            let cx = x as isize + dx;
                            let mapdist = get(cx,cy).unwrap_or(0);
                            if mapdist == dist {
                                set(x,y, nextdist);
                                no_progress = false;
                                break 'inner;
                            }
                        }
                    }
                }
            }

            if no_progress {
                break;
            }
            dist = nextdist;
        }
        if dist == 0 {
            return Hole{width:0,height:0};
        }
        let mut candidates = vec![];
        for y in 0..self.bitmap.height {
            for x in 0..self.bitmap.width {
                if get(x as isize,y as isize).unwrap() != dist {
                    continue;
                }
                candidates.push(Rect{x0:x,y0:y,x1:x,y1:y});
            }
        }
        let mut biggest_hole = Hole {
            width: 0,
            height: 0,
        };
        let mut biggest_area = 0;


        for mut rect in candidates {
            loop {
                let mut progress = false;
                let dirs ;
                if self.measure(rect.grow_right().hole()) > self.measure(rect.grow_down().hole()) {
                    dirs = [true, false];
                } else {
                    dirs = [false, true];
                }
                for horiz in dirs {
                    if horiz {
                        if rect.left_neighbors().map(|x|x.is_obstructed(&self.bitmap)).unwrap_or(true) == false {
                            progress = true;
                            rect = rect.grow_left();
                            break;
                        }
                        if rect.right_neighbors(self.bitmap.width).map(|x|x.is_obstructed(&self.bitmap)).unwrap_or(true) == false {
                            progress = true;
                            rect = rect.grow_right();
                            break;
                        }
                    } else {
                        if rect.top_neighbors().map(|x|x.is_obstructed(&self.bitmap)).unwrap_or(true) == false {
                            progress = true;
                            rect = rect.grow_up();
                            break;
                        }
                        if rect.bottom_neighbors(self.bitmap.height).map(|x|x.is_obstructed(&self.bitmap)).unwrap_or(true) == false {
                            progress = true;
                            rect = rect.grow_down();
                            break;
                        }
                    }
                }

                if !progress {
                    break;
                }
            }
            let metric = self.measure(rect.hole());
            if metric > biggest_area {
                biggest_area = metric;
                biggest_hole = rect.hole();
            }
        }
        biggest_hole
    }

    fn measure(&self, hole: Hole) -> usize {
        (self.metric)(hole)
    }

    /// Determine how sizes of holes are measured, for the feature
    /// 'get_largest_hole' that returns the largest non-occupied area in a bin.
    /// Default is area (width * height).
    ///
    /// Must be called _before_ 'place_all', to have any effect
    pub fn set_metric(&mut self, metric: fn(Hole)->usize) {
        self.metric = metric;
    }

    /// Return the largest free area available after the most recent successful or unsuccessful
    /// 'place_all'.
    pub fn get_largest_hole(&self) -> Hole {
        self.largest_hole
    }

    /// Place all the items given by the iterator 'items'.
    /// Returns true if all items could be placed.
    /// The solution can be retrieved by calling the 'solution'-method.
    /// Note that this library does not in general produce optimal solutions.
    pub fn place_all(&mut self, input: impl Iterator<Item=Item<I>>, mut cancel: impl FnMut() -> bool) -> bool {
        let mut input_items:Vec<Item<I>> = input.collect();
        input_items.sort_by_key(|x|Reverse(x.size()));
        let any_rotatable = input_items.iter().any(|x|x.allow_rotate);
        if self.place_all_impl(&input_items, Strategy::DoNotRotate, &mut cancel) {
            self.largest_hole = self.calculate_largest_hole();
            return true;
        }
        self.largest_hole = self.calculate_largest_hole();
        if !any_rotatable {
            return false; //No point in trying passes where rotation is allowed, since none of the items allow rotation
        }
        if cancel() {
            return false;
        }
        self.items.clear();
        self.bitmap.clear();
        if self.place_all_impl(&input_items, Strategy::Rotate, &mut cancel) {
            self.largest_hole = self.calculate_largest_hole();
            return true;
        }
        let new_largest_hole = self.calculate_largest_hole();
        if self.measure(new_largest_hole) > self.measure(self.largest_hole) {
            self.largest_hole = new_largest_hole;
        }
        if cancel() {
            return false;
        }
        self.items.clear();
        self.bitmap.clear();
        let placed = self.place_all_impl(&input_items, Strategy::RotateIfSuitable, &mut cancel);
        let new_largest_hole = self.calculate_largest_hole();
        if self.measure(new_largest_hole) > self.measure(self.largest_hole) {
            self.largest_hole = new_largest_hole;
        }
        placed
    }
    fn place_all_impl(&mut self, items: &[Item<I>], strategy: Strategy, mut cancel: impl FnMut() -> bool) -> bool {
        let mut all_fit = true;
        for item in items {
            if !self.add_to_best_fit(item, strategy, &mut cancel) {
                all_fit = false;
            }
            if cancel() {
                return false;
            }
        }
        all_fit
    }

    fn place(&mut self, x0: usize, y0:usize, item: &Item<I>, rotated: bool) {
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
            id: item.id.clone(),
        });
    }
    fn evaluate_fit(&self, x0: usize, y0: usize, w: usize, h: usize) -> Option<usize> {
        if x0 >= self.bitmap.width || y0 >= self.bitmap.height || x0 + w > self.bitmap.width || y0 + h > self.bitmap.height {
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
    fn add_to_best_fit(&mut self, item: &Item<I>, strategy: Strategy, mut cancel: impl FnMut() -> bool) -> bool {
        if item.w == 0 || item.h == 0 {
            panic!("Item size must not be 0 in any dimension");
        }
        if item.w > self.bitmap.width && item.h > self.bitmap.height {
            return false; //Impossible to fit.
        }
        let mut cur_best_fit = usize::MAX;
        let smallest_dim = item.h.min(item.w);
        let mut best_fit = None;
        for y in 0..self.bitmap.height.saturating_sub(smallest_dim - 1) {
            let mut had_busy = false;
            if cancel() {
                return false;
            }
            for x in 0..self.bitmap.width.saturating_sub(smallest_dim - 1) {
                if self.bitmap.get(x, y) {
                   had_busy = true;
                }
                if strategy == Strategy::DoNotRotate || strategy == Strategy::RotateIfSuitable {
                    if let Some(fit) = self.evaluate_fit(x,y,item.w,item.h) {
                        if fit < cur_best_fit {
                            cur_best_fit = fit;
                            best_fit = Some((x,y,false));
                        }
                    }
                }
                if item.allow_rotate && (strategy == Strategy::Rotate || strategy == Strategy::RotateIfSuitable) {
                    if let Some(fit) = self.evaluate_fit(x, y, item.h, item.w) { //Rotated
                        if fit < cur_best_fit {
                            cur_best_fit = fit;
                            best_fit = Some((x, y, true));
                        }
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
    fn test_hole() {
        let items = [
            Item {
                w: 10,
                h: 3,
                allow_rotate: true,
                id: 'A'
            },
            Item {
                w: 5,
                h: 3,
                allow_rotate: true,
                id: 'B'
            },
            Item {
                w: 10,
                h: 5,
                allow_rotate: true,
                id: 'C'
            },
            ];
        let mut bin = Bin::new(10,10);
        bin.set_metric(|hole|hole.width);
        let all_fit = bin.place_all(items.into_iter(),||false);
        print_solution(&bin, true);
        println!("Hole: {:?} (all_fit: {:?})", bin.largest_hole, all_fit)

    }

    #[test]
    fn it_works() {

        let items = [
            Item {
                w: 10,
                h: 3,
                allow_rotate: true,
                id: 'D'
            },
            Item {
                w: 10,
                h: 3,
                allow_rotate: true,
                id: 'A'
            },
            Item {
                w: 10,
                h: 3,
                allow_rotate: true,
                id: 'B'
            },
            Item {
                w: 1,
                h: 10,
                allow_rotate: true,
                id: 'C'
            },
        ];
        let mut bin = Bin::new(10,10);
        let all_fit = bin.place_all(items.into_iter(),||false);
        println!("All items placed: {:?}", all_fit);
        print_solution(&bin, false);
    }
    fn print_solution(bin: &Bin<char>, expect_hole: bool) {
        println!("Solution: {:#?}", bin.solution());
        let places = bin.solution();
        for row in 0..10 {
            for col in 0..10 {
                let id = places.iter().find(|x|x.contains((col,row)));
                let c = id.map(|x|x.id).unwrap_or(' ');
                if !expect_hole {
                    assert_ne!(c, ' ');
                }
                print!("{}", c);
            }
            println!("|");
        }
    }
}
