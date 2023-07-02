//! Why write my own algorithm?
//!
//! Surprisingly few rect-packing crates support a grow operation. After pouring
//! through countless crates, I realized none quite did exactly what I wanted,
//! and I wasn't looking forward to actually implementing the texture management
//! code. So I decided to let myself get distracted with this problem, after
//! reading a summary of how [shelf-pack](https://github.com/mapbox/shelf-pack/)
//! was designed, which inspired [etagere](https://github.com/nical/etagere)
//! which has a PR open to add a grow operation when I'm writing this.
//!
//! Why not just use that branch? It's what I'll be falling back on if I don't
//! end up liking what I wrote here. The only downside of using etagere once
//! that PR is merged is needing euclid conversions, which is really not a good
//! reason to write your own packing algorithm.

use crate::math::{Point, Rect, Size, UPixels};

#[derive(Debug)]
pub struct TexturePacker {
    size: Size<UPixels>,
    allocated_width: u32,
    minimum_column_width: u16,
    columns: Vec<Column>,
}

impl TexturePacker {
    pub const fn new(size: Size<UPixels>, minimum_column_width: u16) -> Self {
        Self {
            size,
            allocated_width: 0,
            minimum_column_width,
            columns: Vec::new(),
        }
    }
    pub fn allocate(&mut self, area: Size<UPixels>) -> Option<TextureAllocation> {
        self.allocate_area(Size {
            width: area.width.0.try_into().expect("area allocated too large"),
            height: area.height.0.try_into().expect("area allocated too large"),
        })
        .map(|allocation| TextureAllocation {
            allocation,
            rect: Rect::new(self.allocation_origin(allocation), area),
        })
    }

    fn allocate_area(&mut self, area: Size<u16>) -> Option<AllocationId> {
        for (column_index, column) in self
            .columns
            .iter_mut()
            .enumerate()
            .filter(|(_, col)| area.width <= col.width)
        {
            if let Some(allocation) = column.allocate(area, column_index) {
                return Some(allocation);
            }
        }

        // No shelves found in any column. Allocate a shelf.
        for (column_index, column) in self
            .columns
            .iter_mut()
            .enumerate()
            .filter(|(_, col)| area.width <= col.width)
        {
            let remaining_height = self.size.height - column.allocated_height;
            if remaining_height.0 >= u32::from(area.height) {
                return Some(column.allocate_in_new_shelf(area, column_index));
            } /*else if let Some(last_shelf) = column.shelves.last_mut() {
                  if last_shelf.remaining_width >= u32::from(area.width) {
                      let growable_height = remaining_height + last_shelf.height;
                      if growable_height >= u32::from(area.height) {
                          // We can grow the existing shelf to occupy the remaining area.
                      }
                  }
              }*/
        }

        let width = self.minimum_column_width.max(area.width);
        let remaining_width = self.size.width - self.allocated_width;
        if u32::from(width) <= remaining_width.0 {
            let mut column = Column::new(self.allocated_width, width);
            self.allocated_width += u32::from(width);
            let allocation = column.allocate_in_new_shelf(area, self.columns.len());
            self.columns.push(column);
            return Some(allocation);
        }

        None
    }

    fn allocation_origin(&self, allocation: AllocationId) -> Point<UPixels> {
        let col = &self.columns[usize::from(allocation.column)];
        let shelf = &col.shelves[usize::from(allocation.shelf)];
        Point::new(col.x + u32::from(allocation.offset), shelf.y)
    }

    pub fn free(&mut self, id: AllocationId) {
        let column = &mut self.columns[usize::from(id.column)];
        let shelf = &mut column.shelves[usize::from(id.shelf)];
        let Some((index, allocation)) = shelf.allocations.iter_mut().enumerate().find(|(_, allocation)| allocation.offset() == id.offset) else { unreachable!("can't find allocation to free") };
        assert!(allocation.allocated());
        allocation.deallocate();
        shelf
            .free_list
            .push(index.try_into().expect("too many allocations"));
    }

    pub fn allocated(&self) -> UPixels {
        let mut allocated = 0;

        for col in &self.columns {
            for shelf in &col.shelves {
                for allocation in &shelf.allocations {
                    if allocation.allocated() {
                        allocated += u32::from(allocation.length) * u32::from(shelf.height);
                    }
                }
            }
        }

        allocated.into()
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TextureAllocation {
    pub allocation: AllocationId,
    pub rect: Rect<UPixels>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct AllocationId {
    shelf: u16,
    column: u16,
    offset: u16,
}

#[derive(Debug)]
struct Column {
    x: u32,
    allocated_height: u32,
    width: u16,
    shelves: Vec<Shelf>,
    shelves_by_height: Vec<u16>,
}

impl Column {
    pub const fn new(x: u32, width: u16) -> Self {
        Self {
            x,
            width,
            shelves: Vec::new(),
            shelves_by_height: Vec::new(),
            allocated_height: 0,
        }
    }

    pub fn new_shelf(&mut self, height: u16) -> &mut Shelf {
        let shelf = Shelf::new(self.allocated_height, height);
        self.allocated_height += u32::from(height);
        let index = self.shelves.len();
        self.shelves.push(shelf);
        &mut self.shelves[index]
    }

    pub fn allocate_in_new_shelf(&mut self, area: Size<u16>, my_index: usize) -> AllocationId {
        let column_width = self.width;
        let shelf_index = self.shelves.len().try_into().expect("too many shelves");
        let shelf = self.new_shelf(area.height);
        let offset = shelf
            .allocate(area.width, column_width)
            .expect("new shelf must have enough free space");
        let by_height_index = self
            .shelves_by_height
            .binary_search_by(|index| self.shelves[usize::from(*index)].height.cmp(&area.height))
            .map_or_else(|i| i, |i| i);
        self.shelves_by_height.insert(by_height_index, shelf_index);
        AllocationId {
            shelf: shelf_index,
            column: my_index.try_into().expect("too many columns"),
            offset,
        }
    }

    pub fn allocate(&mut self, area: Size<u16>, my_index: usize) -> Option<AllocationId> {
        // Iterate over the shelves in order of their height.
        for shelf_index in &self.shelves_by_height {
            let shelf = &mut self.shelves[usize::from(*shelf_index)];
            if shelf.height < area.height {
                continue;
            } else if shelf.height / 2 > area.height {
                // We want to avoid allocating into a shelf when we leave over
                // 50% of the space empty.
                break;
                // TODO we should keep track of a fallback allocation for when
                // the texture is packed so much that an unoptimal allocation is
                // better than failing to allocate.
            }

            if let Some(offset) = shelf.allocate(area.width, self.width) {
                return Some(AllocationId {
                    shelf: *shelf_index,
                    column: my_index.try_into().expect("too many columns"),
                    offset,
                });
            }
        }

        None
    }
}

#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u16,
    allocated_width: u16,
    allocations: Vec<Allocation>,
    free_list: Vec<u16>,
}

impl Shelf {
    pub const fn new(y: u32, height: u16) -> Self {
        Self {
            y,
            height,
            allocated_width: 0,
            allocations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn allocate(&mut self, width: u16, column_width: u16) -> Option<u16> {
        let remaining_width = column_width - self.allocated_width;
        if remaining_width >= width {
            let offset = self.allocated_width;
            self.allocations.push(Allocation::new(offset, width));
            self.allocated_width += width;

            Some(offset)
        } else {
            for (free_list_index, free_index) in self.free_list.iter().copied().enumerate().rev() {
                let allocation = &mut self.allocations[usize::from(free_index)];
                if let Some(extra_space) = allocation.length.checked_sub(width) {
                    let offset = allocation.offset();
                    allocation.allocate();
                    if extra_space > 0 {
                        // Add a new allocation to keep track of the freed space.
                        let new_offset = offset + allocation.length;
                        self.allocations.insert(
                            usize::from(free_index + 1),
                            Allocation::new_free(new_offset, extra_space),
                        );
                        self.free_list[free_list_index] += 1;
                    } else {
                        // The allocation was fully used, remove it from the free list.
                        self.free_list.remove(free_list_index);
                    }
                    return Some(offset);
                }
            }
            None
        }
    }
}

#[derive(Debug)]
struct Allocation {
    status: u16,
    length: u16,
}

impl Allocation {
    const ALLOCATED_BIT: u16 = 0x8000;
    const OFFSET_MASK: u16 = 0x7FFF;

    const fn allocated(&self) -> bool {
        self.status & Self::ALLOCATED_BIT != 0
    }

    const fn offset(&self) -> u16 {
        self.status & Self::OFFSET_MASK
    }

    pub fn new(offset: u16, length: u16) -> Self {
        let mut allocation = Allocation::new_free(offset, length);
        allocation.allocate();
        allocation
    }

    pub fn new_free(offset: u16, length: u16) -> Self {
        let allocation = Self {
            status: offset,
            length,
        };
        assert!(!allocation.allocated(), "length too large");
        allocation
    }

    fn allocate(&mut self) {
        self.status |= Self::ALLOCATED_BIT;
    }

    fn deallocate(&mut self) {
        self.status &= Self::OFFSET_MASK;
    }
}

#[test]
fn reallocation() {
    let mut packer = TexturePacker::new(Size::new(32, 2), 16);
    let first = dbg!(packer.allocate(Size::new(8, 1)).unwrap());
    let second = dbg!(packer.allocate(Size::new(8, 1)).unwrap());
    assert_eq!(first.allocation.column, second.allocation.column);
    assert_eq!(first.allocation.shelf, second.allocation.shelf);
    assert_ne!(first.allocation.offset, second.allocation.offset);
    assert!(!first.rect.intersects(&second.rect));
    packer.free(first.allocation);
    let reallocated = dbg!(packer.allocate(Size::new(8, 1)).unwrap());
    assert_eq!(first, reallocated);

    // New allocations will overflow into the next shelf
    let overflowed = dbg!(packer.allocate(Size::new(8, 1)).unwrap());
    assert_eq!(first.allocation.column, overflowed.allocation.column);
    assert_ne!(first.allocation.shelf, overflowed.allocation.shelf);

    // We are leaving an empty 8 pixels on this location.

    let next_column = packer.allocate(Size::new(8, 2)).unwrap();
    assert_ne!(first.allocation.column, next_column.allocation.column);

    let last_allocation = packer.allocate(Size::new(8, 2)).unwrap();
    assert_eq!(
        last_allocation.allocation.column,
        next_column.allocation.column
    );

    assert_eq!(packer.allocated().0, 56);
}
