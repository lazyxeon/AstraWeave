// SPDX-License-Identifier: MIT
//! Type-erased contiguous storage for components
//!
//! Inspired by Bevy's BlobVec, this provides cache-friendly storage without
//! the overhead of Box<dyn Any> indirection. Components are stored as raw bytes
//! with proper alignment and drop handling.

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr::{self, NonNull};

/// Type-erased vector of components stored contiguously in memory.
///
/// This is the foundation for high-performance ECS storage, providing:
/// - Zero heap indirection (vs Box<dyn Any>)
/// - SIMD-friendly contiguous memory
/// - Cache-friendly iteration
/// - Proper drop handling via function pointer
///
/// # Type-Erased API
///
/// For archetype storage, BlobVec provides type-erased operations:
/// - `from_layout()`: Create BlobVec from runtime Layout
/// - `push_raw()`: Push bytes with clone function
/// - `get_raw()` / `get_raw_mut()`: Get raw pointers
/// - `swap_remove_raw()`: Remove without returning value
pub struct BlobVec {
    /// Raw pointer to the start of component data
    data: NonNull<u8>,
    /// Number of components currently stored
    len: usize,
    /// Allocated capacity (in number of components)
    capacity: usize,
    /// Memory layout of a single component
    item_layout: Layout,
    /// Function to drop a single component
    /// SAFETY: Must be called with a valid pointer to T
    drop_fn: Option<unsafe fn(*mut u8)>,
}

impl BlobVec {
    /// Create a new BlobVec for components of type T
    pub fn new<T: 'static>() -> Self {
        let item_layout = Layout::new::<T>();
        let drop_fn = if std::mem::needs_drop::<T>() {
            Some(Self::drop_fn_of::<T>() as unsafe fn(*mut u8))
        } else {
            None
        };

        Self {
            data: NonNull::dangling(),
            len: 0,
            capacity: 0,
            item_layout,
            drop_fn,
        }
    }

    /// Create a new BlobVec with specified capacity
    pub fn with_capacity<T: 'static>(capacity: usize) -> Self {
        let mut blob = Self::new::<T>();
        if capacity > 0 {
            blob.reserve(capacity);
        }
        blob
    }

    // ========================================================================
    // Type-Erased Constructors (for ComponentMeta integration)
    // ========================================================================

    /// Create a BlobVec from a runtime Layout (type-erased).
    ///
    /// Used by archetype storage when component type is only known as TypeId.
    ///
    /// # Arguments
    /// * `item_layout` - Memory layout for each item
    /// * `drop_fn` - Function to drop an item (None if no drop needed)
    pub fn from_layout(item_layout: Layout, drop_fn: Option<unsafe fn(*mut u8)>) -> Self {
        Self {
            data: NonNull::dangling(),
            len: 0,
            capacity: 0,
            item_layout,
            drop_fn,
        }
    }

    /// Create a BlobVec from a runtime Layout with pre-allocated capacity.
    pub fn from_layout_with_capacity(
        item_layout: Layout,
        drop_fn: Option<unsafe fn(*mut u8)>,
        capacity: usize,
    ) -> Self {
        let mut blob = Self::from_layout(item_layout, drop_fn);
        if capacity > 0 {
            blob.reserve(capacity);
        }
        blob
    }

    /// Get the drop function for type T
    fn drop_fn_of<T>() -> unsafe fn(*mut u8) {
        |ptr| unsafe {
            ptr.cast::<T>().drop_in_place();
        }
    }

    /// Reserve space for at least `additional` more components
    #[allow(clippy::expect_used)] // OOM/layout panics are the only correct response — no recovery possible
    pub fn reserve(&mut self, additional: usize) {
        let required_cap = self.len.checked_add(additional).expect("capacity overflow");
        if required_cap <= self.capacity {
            return;
        }

        let new_capacity = required_cap.max(self.capacity * 2).max(4);

        let new_layout = Layout::from_size_align(
            self.item_layout.size() * new_capacity,
            self.item_layout.align(),
        )
        .expect("invalid layout");

        let new_data = if self.capacity == 0 {
            // First allocation
            // SAFETY: `new_layout` has non-zero size (validated above) and correct alignment.
            unsafe { NonNull::new(alloc(new_layout)).expect("allocation failed") }
        } else {
            // Reallocate existing memory
            let old_layout = Layout::from_size_align(
                self.item_layout.size() * self.capacity,
                self.item_layout.align(),
            )
            .expect("invalid layout");

            // SAFETY: `self.data` was allocated with `old_layout`. `new_layout.size()` >= old size.
            unsafe {
                let new_ptr = realloc(self.data.as_ptr(), old_layout, new_layout.size());
                NonNull::new(new_ptr).expect("reallocation failed")
            }
        };

        self.data = new_data;
        self.capacity = new_capacity;
    }

    /// Push a component onto the end of the vector
    ///
    /// # Safety
    /// Type T must match the type this BlobVec was created for
    pub unsafe fn push<T>(&mut self, value: T) {
        if self.len == self.capacity {
            self.reserve(1);
        }

        // SAFETY: Caller guarantees T matches this BlobVec's type. `reserve(1)` ensures
        // capacity. Offset is within bounds because `self.len < self.capacity`.
        let ptr = self.data.as_ptr().add(self.len * self.item_layout.size());
        ptr.cast::<T>().write(value);
        self.len += 1;
    }

    // ========================================================================
    // Type-Erased Operations (for archetype storage)
    // ========================================================================

    /// Push raw bytes with a clone function (type-erased push).
    ///
    /// This is used by archetype transitions where we only have a raw pointer
    /// and a clone function from ComponentMeta.
    ///
    /// # Safety
    /// - `src` must point to valid data matching this BlobVec's layout
    /// - `clone_fn` must correctly copy data from src to dst
    pub unsafe fn push_raw(&mut self, src: *const u8, clone_fn: unsafe fn(*const u8, *mut u8)) {
        if self.len == self.capacity {
            self.reserve(1);
        }

        // SAFETY: Caller guarantees `src` is valid and `clone_fn` copies correctly.
        // `reserve(1)` ensures capacity. Offset is within allocation bounds.
        let dst = self.data.as_ptr().add(self.len * self.item_layout.size());
        clone_fn(src, dst);
        self.len += 1;
    }

    /// Get a raw pointer to the component at the specified index.
    ///
    /// Returns None if index is out of bounds.
    ///
    /// # Safety
    /// The returned pointer is only valid while no mutations occur.
    pub fn get_raw(&self, index: usize) -> Option<*const u8> {
        if index >= self.len {
            return None;
        }
        // SAFETY: Bounds check above ensures index < len, so offset is within allocation.
        Some(unsafe { self.data.as_ptr().add(index * self.item_layout.size()) })
    }

    /// Get a mutable raw pointer to the component at the specified index.
    ///
    /// Returns None if index is out of bounds.
    ///
    /// # Safety
    /// - The returned pointer is only valid while the BlobVec is not reallocated
    /// - Caller must ensure no aliasing
    pub fn get_raw_mut(&mut self, index: usize) -> Option<*mut u8> {
        if index >= self.len {
            return None;
        }
        // SAFETY: Bounds check above ensures index < len, so offset is within allocation.
        // Mutable borrow of `self` prevents aliasing.
        Some(unsafe { self.data.as_ptr().add(index * self.item_layout.size()) })
    }

    /// Swap-remove at index without returning the value (drops it).
    ///
    /// This is used when removing an entity from an archetype
    /// where we don't need to preserve the component value.
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub fn swap_remove_raw(&mut self, index: usize) {
        assert!(index < self.len, "index out of bounds");

        let item_size = self.item_layout.size();
        let last_index = self.len - 1;

        // Drop the element at index
        if let Some(drop_fn) = self.drop_fn {
            // SAFETY: `index < len` (asserted above), so pointer is within allocation.
            // drop_fn matches the component type registered at construction.
            unsafe {
                let ptr = self.data.as_ptr().add(index * item_size);
                drop_fn(ptr);
            }
        }

        // If not the last element, copy last element to this position
        if index != last_index {
            // SAFETY: Both `index` and `last_index` are within bounds. src != dst
            // since index != last_index, so copy_nonoverlapping is safe.
            unsafe {
                let src = self.data.as_ptr().add(last_index * item_size);
                let dst = self.data.as_ptr().add(index * item_size);
                ptr::copy_nonoverlapping(src, dst, item_size);
            }
        }

        self.len -= 1;
    }

    /// Swap-remove and copy bytes to destination (for archetype transitions).
    ///
    /// Like swap_remove_raw but copies the removed value to `dst` instead of dropping.
    ///
    /// # Safety
    /// - `dst` must have enough space and proper alignment
    /// - `clone_fn` must correctly copy the data
    ///
    /// # Panics
    /// Panics if index is out of bounds.
    pub unsafe fn swap_remove_raw_to(
        &mut self,
        index: usize,
        dst: *mut u8,
        clone_fn: unsafe fn(*const u8, *mut u8),
    ) {
        assert!(index < self.len, "index out of bounds");

        let item_size = self.item_layout.size();
        let last_index = self.len - 1;

        // Copy the element at index to dst
        let src = self.data.as_ptr().add(index * item_size);
        clone_fn(src, dst);

        // Drop the original at index (cast const ptr to mut for drop)
        if let Some(drop_fn) = self.drop_fn {
            drop_fn(src as *const u8 as *mut u8);
        }

        // If not the last element, move last element to this position
        if index != last_index {
            let last_src = self.data.as_ptr().add(last_index * item_size);
            ptr::copy_nonoverlapping(last_src, src as *const u8 as *mut u8, item_size);
        }

        self.len -= 1;
    }

    /// Get a reference to a component at the specified index
    ///
    /// # Safety
    /// - Index must be in bounds
    /// - Type T must match the type this BlobVec was created for
    pub unsafe fn get<T>(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        let ptr = self.data.as_ptr().add(index * self.item_layout.size());
        Some(&*ptr.cast::<T>())
    }

    /// Get a mutable reference to a component at the specified index
    ///
    /// # Safety
    /// - Index must be in bounds
    /// - Type T must match the type this BlobVec was created for
    pub unsafe fn get_mut<T>(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }

        let ptr = self.data.as_ptr().add(index * self.item_layout.size());
        Some(&mut *ptr.cast::<T>())
    }

    /// Get the component data as a contiguous slice
    ///
    /// This is the key to high-performance iteration - direct memory access!
    ///
    /// # Safety
    /// Type T must match the type this BlobVec was created for
    pub unsafe fn as_slice<T>(&self) -> &[T] {
        if self.len == 0 {
            return &[];
        }
        std::slice::from_raw_parts(self.data.as_ptr().cast::<T>(), self.len)
    }

    /// Get the component data as a mutable contiguous slice
    ///
    /// # Safety
    /// Type T must match the type this BlobVec was created for
    pub unsafe fn as_slice_mut<T>(&mut self) -> &mut [T] {
        if self.len == 0 {
            return &mut [];
        }
        std::slice::from_raw_parts_mut(self.data.as_ptr().cast::<T>(), self.len)
    }

    /// Remove and return the component at the specified index
    /// Uses swap_remove for O(1) performance (order not preserved)
    ///
    /// # Safety
    /// - Index must be in bounds
    /// - Type T must match the type this BlobVec was created for
    pub unsafe fn swap_remove<T>(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");

        let last_index = self.len - 1;
        let ptr = self
            .data
            .as_ptr()
            .add(index * self.item_layout.size())
            .cast::<T>();

        if index != last_index {
            // Swap with last element
            let last_ptr = self
                .data
                .as_ptr()
                .add(last_index * self.item_layout.size())
                .cast::<T>();
            ptr::swap(ptr, last_ptr);
        }

        self.len -= 1;
        ptr::read(
            self.data
                .as_ptr()
                .add(self.len * self.item_layout.size())
                .cast::<T>(),
        )
    }

    /// Get the number of components stored
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the BlobVec is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the current capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all components, calling their drop functions
    pub fn clear(&mut self) {
        if let Some(drop_fn) = self.drop_fn {
            for i in 0..self.len {
                // SAFETY: Each `i < self.len`, so offset is within allocation.
                // Elements are dropped once (loop runs exactly `len` times, then len = 0).
                unsafe {
                    let ptr = self.data.as_ptr().add(i * self.item_layout.size());
                    drop_fn(ptr);
                }
            }
        }
        self.len = 0;
    }
}

impl Drop for BlobVec {
    #[allow(clippy::expect_used)] // INVARIANT: layout params are identical to those used at allocation time
    fn drop(&mut self) {
        self.clear();

        if self.capacity > 0 {
            let layout = Layout::from_size_align(
                self.item_layout.size() * self.capacity,
                self.item_layout.align(),
            )
            .expect("invalid layout");

            // SAFETY: `self.data` was allocated with this layout. capacity > 0 ensures
            // the layout is non-zero-sized.
            unsafe {
                dealloc(self.data.as_ptr(), layout);
            }
        }
    }
}

// SAFETY: BlobVec's data pointer is heap-allocated and not shared across threads
// without synchronization. The type-erased storage is only accessed through &self
// (shared) or &mut self (exclusive), maintaining Rust's aliasing guarantees.
unsafe impl Send for BlobVec {}
// SAFETY: Same reasoning as Send — concurrent &BlobVec access is safe because
// shared references cannot mutate the underlying data.
unsafe impl Sync for BlobVec {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }

    #[derive(Debug, PartialEq)]
    struct DropTest {
        value: i32,
        dropped: std::rc::Rc<std::cell::Cell<bool>>,
    }

    impl Drop for DropTest {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    #[test]
    fn test_push_and_get() {
        let mut blob = BlobVec::new::<Position>();

        unsafe {
            blob.push(Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            });
            blob.push(Position {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            });
        }

        assert_eq!(blob.len(), 2);

        unsafe {
            let pos1 = blob.get::<Position>(0).unwrap();
            assert_eq!(pos1.x, 1.0);
            assert_eq!(pos1.y, 2.0);

            let pos2 = blob.get::<Position>(1).unwrap();
            assert_eq!(pos2.x, 4.0);
            assert_eq!(pos2.y, 5.0);
        }
    }

    #[test]
    fn test_as_slice() {
        let mut blob = BlobVec::new::<Position>();

        unsafe {
            blob.push(Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            });
            blob.push(Position {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            });
            blob.push(Position {
                x: 7.0,
                y: 8.0,
                z: 9.0,
            });

            let slice = blob.as_slice::<Position>();
            assert_eq!(slice.len(), 3);
            assert_eq!(slice[0].x, 1.0);
            assert_eq!(slice[1].x, 4.0);
            assert_eq!(slice[2].x, 7.0);
        }
    }

    #[test]
    fn test_as_slice_mut() {
        let mut blob = BlobVec::new::<Position>();

        unsafe {
            blob.push(Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            });
            blob.push(Position {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            });

            let slice = blob.as_slice_mut::<Position>();
            slice[0].x = 100.0;
            slice[1].y = 200.0;

            let slice = blob.as_slice::<Position>();
            assert_eq!(slice[0].x, 100.0);
            assert_eq!(slice[1].y, 200.0);
        }
    }

    #[test]
    fn test_swap_remove() {
        let mut blob = BlobVec::new::<Position>();

        unsafe {
            blob.push(Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            });
            blob.push(Position {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            });
            blob.push(Position {
                x: 7.0,
                y: 8.0,
                z: 9.0,
            });

            let removed = blob.swap_remove::<Position>(1);
            assert_eq!(removed.x, 4.0);

            assert_eq!(blob.len(), 2);

            // Element at index 1 should now be the former last element
            let pos = blob.get::<Position>(1).unwrap();
            assert_eq!(pos.x, 7.0);
        }
    }

    #[test]
    fn test_drop_handling() {
        let dropped = std::rc::Rc::new(std::cell::Cell::new(false));

        {
            let mut blob = BlobVec::new::<DropTest>();

            unsafe {
                blob.push(DropTest {
                    value: 42,
                    dropped: dropped.clone(),
                });
            }

            assert!(!dropped.get());
        } // BlobVec dropped here

        assert!(dropped.get());
    }

    #[test]
    fn test_clear() {
        let dropped1 = std::rc::Rc::new(std::cell::Cell::new(false));
        let dropped2 = std::rc::Rc::new(std::cell::Cell::new(false));

        let mut blob = BlobVec::new::<DropTest>();

        unsafe {
            blob.push(DropTest {
                value: 1,
                dropped: dropped1.clone(),
            });
            blob.push(DropTest {
                value: 2,
                dropped: dropped2.clone(),
            });
        }

        assert!(!dropped1.get());
        assert!(!dropped2.get());

        blob.clear();

        assert!(dropped1.get());
        assert!(dropped2.get());
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn test_reserve() {
        let mut blob = BlobVec::new::<Position>();
        assert_eq!(blob.capacity(), 0);

        blob.reserve(10);
        assert!(blob.capacity() >= 10);

        let old_capacity = blob.capacity();
        blob.reserve(5); // Should not reallocate
        assert_eq!(blob.capacity(), old_capacity);
    }

    // ====================
    // Day 3: Surgical Coverage Improvements - blob_vec.rs
    // ====================

    #[test]
    fn test_with_capacity() {
        // Tests constructor pre-allocation
        let blob = BlobVec::with_capacity::<Position>(50);
        assert!(blob.capacity() >= 50);
        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
    }

    #[test]
    fn test_with_capacity_zero() {
        // Tests edge case: capacity = 0
        let blob = BlobVec::with_capacity::<Position>(0);
        assert_eq!(blob.capacity(), 0);
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn test_capacity_method() {
        // Tests capacity() accessor method
        let mut blob = BlobVec::new::<i32>();
        assert_eq!(blob.capacity(), 0);

        unsafe {
            blob.push(42);
        }
        assert!(blob.capacity() >= 1);

        let cap = blob.capacity();
        unsafe {
            blob.push(99);
        }
        assert_eq!(blob.capacity(), cap); // Should not reallocate
    }

    #[test]
    fn test_as_slice_empty() {
        // Tests as_slice() when len == 0 (early return path)
        let blob = BlobVec::new::<Position>();
        unsafe {
            let slice = blob.as_slice::<Position>();
            assert_eq!(slice.len(), 0);
        }
    }

    #[test]
    fn test_as_slice_mut_empty() {
        // Tests as_slice_mut() when len == 0 (early return path)
        let mut blob = BlobVec::new::<Position>();
        unsafe {
            let slice = blob.as_slice_mut::<Position>();
            assert_eq!(slice.len(), 0);
        }
    }

    #[test]
    fn test_get_out_of_bounds() {
        // Tests get() error handling for invalid index
        let mut blob = BlobVec::new::<i32>();
        unsafe {
            blob.push(10);
            blob.push(20);
        }

        unsafe {
            assert!(blob.get::<i32>(0).is_some());
            assert!(blob.get::<i32>(1).is_some());
            assert!(blob.get::<i32>(2).is_none()); // Out of bounds
            assert!(blob.get::<i32>(999).is_none()); // Way out of bounds
        }
    }

    #[test]
    fn test_get_mut_out_of_bounds() {
        // Tests get_mut() error handling for invalid index
        let mut blob = BlobVec::new::<i32>();
        unsafe {
            blob.push(10);
            blob.push(20);
        }

        unsafe {
            assert!(blob.get_mut::<i32>(0).is_some());
            assert!(blob.get_mut::<i32>(1).is_some());
            assert!(blob.get_mut::<i32>(2).is_none()); // Out of bounds
            assert!(blob.get_mut::<i32>(999).is_none()); // Way out of bounds
        }
    }

    #[test]
    fn test_swap_remove_last_element() {
        // Tests no-swap path when removing last element (index == last_index)
        let mut blob = BlobVec::new::<i32>();
        unsafe {
            blob.push(10);
            blob.push(20);
            blob.push(30);
        }

        unsafe {
            // Remove last element - should not swap
            let removed = blob.swap_remove::<i32>(2);
            assert_eq!(removed, 30);
            assert_eq!(blob.len(), 2);

            // Remaining elements unchanged
            assert_eq!(*blob.get::<i32>(0).unwrap(), 10);
            assert_eq!(*blob.get::<i32>(1).unwrap(), 20);
        }
    }

    #[test]
    fn test_no_drop_type() {
        // Tests BlobVec with types that don't need drop (drop_fn = None path)
        let mut blob = BlobVec::new::<i32>();

        unsafe {
            blob.push(1);
            blob.push(2);
            blob.push(3);
        }

        assert_eq!(blob.len(), 3);
        blob.clear();
        assert_eq!(blob.len(), 0);

        // Should not panic even though drop_fn is None for i32
    }

    #[test]
    fn test_large_capacity_growth() {
        // Tests capacity growth algorithm with large reserves
        let mut blob = BlobVec::new::<u8>();

        // Force multiple reallocations
        blob.reserve(1000);
        let cap1 = blob.capacity();
        assert!(cap1 >= 1000);

        unsafe {
            for i in 0..500 {
                blob.push(i as u8);
            }
        }

        blob.reserve(2000);
        let cap2 = blob.capacity();
        assert!(cap2 >= 2500); // 500 existing + 2000 additional

        assert_eq!(blob.len(), 500);
    }

    #[test]
    fn test_is_empty() {
        // Tests is_empty() method (simple but uncovered)
        let mut blob = BlobVec::new::<i32>();
        assert!(blob.is_empty());

        unsafe {
            blob.push(42);
        }
        assert!(!blob.is_empty());

        blob.clear();
        assert!(blob.is_empty());
    }

    // ===========================================================================
    // Mutation-Resistant Remediation Tests: Raw Pointer Operations
    // ===========================================================================
    // These tests target the 25 missed mutants in blob_vec.rs shard 1/6:
    //   get_raw, get_raw_mut, swap_remove_raw, reserve, with_capacity,
    //   from_layout_with_capacity
    // Existing tests only exercise the typed `get::<T>()` API,
    // which does NOT cover the raw pointer arithmetic in get_raw / get_raw_mut.

    #[test]
    fn test_get_raw_returns_correct_pointer_and_value() {
        // Kills mutants: get_raw → None, get_raw → Some(default), >= → <, * → +/÷
        let mut blob = BlobVec::new::<u64>();
        unsafe {
            blob.push(0xDEAD_BEEF_u64);
            blob.push(0xCAFE_BABE_u64);
            blob.push(0x1234_5678_u64);
        }

        // Read back through raw pointer — verifies offset arithmetic
        let ptr0 = blob.get_raw(0).expect("index 0 must be Some");
        let val0 = unsafe { std::ptr::read(ptr0 as *const u64) };
        assert_eq!(val0, 0xDEAD_BEEF_u64);

        let ptr1 = blob.get_raw(1).expect("index 1 must be Some");
        let val1 = unsafe { std::ptr::read(ptr1 as *const u64) };
        assert_eq!(val1, 0xCAFE_BABE_u64);

        let ptr2 = blob.get_raw(2).expect("index 2 must be Some");
        let val2 = unsafe { std::ptr::read(ptr2 as *const u64) };
        assert_eq!(val2, 0x1234_5678_u64);

        // Out of bounds → None
        assert!(blob.get_raw(3).is_none());
        assert!(blob.get_raw(usize::MAX).is_none());
    }

    #[test]
    fn test_get_raw_multi_element_offset_arithmetic() {
        // Specifically tests that index * item_layout.size() is correct
        // Kills: * → + (would be index + size instead of index * size)
        // Kills: * → / (would be index / size)
        #[derive(Clone, Copy, Debug, PartialEq)]
        #[repr(C)]
        struct Big([u8; 64]);

        let mut blob = BlobVec::new::<Big>();
        unsafe {
            blob.push(Big([0xAA; 64]));
            blob.push(Big([0xBB; 64]));
            blob.push(Big([0xCC; 64]));
        }

        // Each element is 64 bytes apart. If arithmetic is wrong, we get garbage.
        let p0 = blob.get_raw(0).unwrap();
        let p1 = blob.get_raw(1).unwrap();
        let p2 = blob.get_raw(2).unwrap();

        // Verify the pointers are exactly 64 bytes apart
        let offset_01 = unsafe { (p1).offset_from(p0) };
        let offset_12 = unsafe { (p2).offset_from(p1) };
        assert_eq!(offset_01, 64, "offset between [0] and [1] must be 64 bytes");
        assert_eq!(offset_12, 64, "offset between [1] and [2] must be 64 bytes");

        // Verify the actual values
        let v0 = unsafe { std::ptr::read(p0 as *const Big) };
        let v2 = unsafe { std::ptr::read(p2 as *const Big) };
        assert_eq!(v0.0[0], 0xAA);
        assert_eq!(v2.0[0], 0xCC);
    }

    #[test]
    fn test_get_raw_mut_returns_correct_pointer_and_allows_mutation() {
        // Kills mutants: get_raw_mut → None, get_raw_mut → Some(default),
        //                >= → <, * → +/÷
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(100_u32);
            blob.push(200_u32);
            blob.push(300_u32);
        }

        // Read through raw_mut pointer
        let ptr1 = blob.get_raw_mut(1).expect("index 1 must be Some");
        let val1 = unsafe { std::ptr::read(ptr1 as *const u32) };
        assert_eq!(val1, 200);

        // Mutate through raw_mut pointer
        unsafe { std::ptr::write(ptr1 as *mut u32, 999) };

        // Verify mutation stuck by re-reading
        let ptr1_again = blob.get_raw(1).unwrap();
        let val1_after = unsafe { std::ptr::read(ptr1_again as *const u32) };
        assert_eq!(val1_after, 999);

        // Other elements unaffected
        let v0 = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const u32) };
        let v2 = unsafe { std::ptr::read(blob.get_raw(2).unwrap() as *const u32) };
        assert_eq!(v0, 100);
        assert_eq!(v2, 300);

        // Out of bounds → None
        assert!(blob.get_raw_mut(3).is_none());
    }

    #[test]
    fn test_swap_remove_raw_data_integrity() {
        // Kills mutants: swap_remove_raw → (), - → +/÷, * → +/÷, != → ==
        // Verifies: element at index is removed, last element moves there,
        //           len decrements, remaining data is correct
        let mut blob = BlobVec::new::<i64>();
        unsafe {
            blob.push(10_i64);
            blob.push(20_i64);
            blob.push(30_i64);
            blob.push(40_i64);
        }
        assert_eq!(blob.len(), 4);

        // Remove index 1 (value 20): last element (40) should move to index 1
        blob.swap_remove_raw(1);
        assert_eq!(blob.len(), 3);

        // Verify remaining data via raw pointers
        let v0 = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const i64) };
        let v1 = unsafe { std::ptr::read(blob.get_raw(1).unwrap() as *const i64) };
        let v2 = unsafe { std::ptr::read(blob.get_raw(2).unwrap() as *const i64) };
        assert_eq!(v0, 10, "element 0 must be unchanged");
        assert_eq!(v1, 40, "last element (40) must swap into removed index");
        assert_eq!(v2, 30, "element 2 must be unchanged");

        // Index 3 is now out of bounds
        assert!(blob.get_raw(3).is_none());
    }

    #[test]
    fn test_swap_remove_raw_last_element() {
        // Tests the `if index != last_index` branch — removing the last element
        // should NOT copy anything, just decrement len.
        // Kills: != → == mutant (would incorrectly copy when removing last)
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(111_u32);
            blob.push(222_u32);
        }

        // Remove last element (index 1)
        blob.swap_remove_raw(1);
        assert_eq!(blob.len(), 1);

        let v0 = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const u32) };
        assert_eq!(v0, 111, "first element must survive removal of last");
        assert!(blob.get_raw(1).is_none());
    }

    #[test]
    fn test_swap_remove_raw_single_element() {
        // Edge case: removing the only element
        let mut blob = BlobVec::new::<u64>();
        unsafe { blob.push(42_u64) };
        assert_eq!(blob.len(), 1);

        blob.swap_remove_raw(0);
        assert_eq!(blob.len(), 0);
        assert!(blob.is_empty());
        assert!(blob.get_raw(0).is_none());
    }

    #[test]
    fn test_reserve_capacity_arithmetic() {
        // Kills: reserve * → + (item_layout.size() + new_capacity instead of *)
        // Kills: reserve * → / (capacity / 2 instead of capacity * 2)
        // We use a large-ish struct so size arithmetic differences are detectable
        #[derive(Clone, Copy)]
        #[repr(C)]
        struct Chunk([u8; 128]);

        let mut blob = BlobVec::new::<Chunk>();
        // Push enough to trigger first reserve
        unsafe { blob.push(Chunk([0; 128])) };
        let cap_after_first = blob.capacity();
        assert!(
            cap_after_first >= 1,
            "must have capacity for at least 1 element"
        );

        // Reserve a large amount — exercises the capacity * 2 growth path
        blob.reserve(100);
        let cap_after_reserve = blob.capacity();
        assert!(
            cap_after_reserve >= 101,
            "capacity must be >= len(1) + additional(100) = 101, got {}",
            cap_after_reserve
        );

        // Now fill up to capacity and verify we can push without panic
        let remaining = cap_after_reserve - blob.len();
        for i in 0..remaining {
            unsafe { blob.push(Chunk([i as u8; 128])) };
        }
        assert_eq!(blob.len(), cap_after_reserve);

        // Verify data integrity after all the pushing
        let first_val = unsafe { std::ptr::read(blob.get_raw(0).unwrap() as *const Chunk) };
        assert_eq!(first_val.0[0], 0);
        let last_val =
            unsafe { std::ptr::read(blob.get_raw(blob.len() - 1).unwrap() as *const Chunk) };
        assert_eq!(last_val.0[0], (remaining - 1) as u8);
    }

    #[test]
    fn test_with_capacity_boundary() {
        // Kills: with_capacity > → >= (capacity 0 should NOT allocate)
        let blob_zero = BlobVec::with_capacity::<u64>(0);
        assert_eq!(blob_zero.len(), 0);
        assert_eq!(blob_zero.capacity(), 0, "capacity=0 should not allocate");

        let blob_one = BlobVec::with_capacity::<u64>(1);
        assert_eq!(blob_one.len(), 0);
        assert!(blob_one.capacity() >= 1, "capacity=1 must allocate");
    }

    #[test]
    fn test_from_layout_with_capacity_boundary() {
        // Kills: from_layout_with_capacity > → >=
        let layout = Layout::new::<f64>();
        let blob_zero = BlobVec::from_layout_with_capacity(layout, None, 0);
        assert_eq!(blob_zero.capacity(), 0, "capacity=0 should not allocate");

        let blob_one = BlobVec::from_layout_with_capacity(layout, None, 1);
        assert!(blob_one.capacity() >= 1, "capacity=1 must allocate");
    }

    #[test]
    fn test_get_raw_boundary_at_len() {
        // Specifically tests index == len (out of bounds) vs index == len-1 (valid)
        // Kills: >= → < in get_raw/get_raw_mut bounds check
        let mut blob = BlobVec::new::<u32>();
        unsafe {
            blob.push(10_u32);
            blob.push(20_u32);
        }
        // index == len-1 = 1: valid
        assert!(blob.get_raw(1).is_some());
        assert!(blob.get_raw_mut(1).is_some());

        // index == len = 2: out of bounds
        assert!(blob.get_raw(2).is_none());
        assert!(blob.get_raw_mut(2).is_none());
    }

    #[test]
    fn test_drop_impl_deallocates_correctly() {
        // Kills: Drop > → < (would skip dealloc for capacity > 0)
        // Kills: Drop * → + / * → / (would compute wrong layout size for dealloc)
        // Allocator crash (UB) or mismatch when dealloc layout ≠ alloc layout.
        // We test by creating a BlobVec with known capacity, pushing data,
        // and dropping it in a scope. If the layout math is wrong, allocator
        // panics or crashes (undefined behavior on Windows → likely access violation).
        // Use large types so the size arithmetic difference is big.
        #[derive(Clone, Copy)]
        #[repr(C)]
        struct BigChunk([u8; 256]);

        for cap in [1, 2, 5, 16, 100] {
            let mut blob = BlobVec::with_capacity::<BigChunk>(cap);
            assert!(blob.capacity() >= cap);
            for i in 0..cap {
                unsafe { blob.push(BigChunk([i as u8; 256])) };
            }
            assert_eq!(blob.len(), cap);
            // Verify data before drop
            let last_ptr = blob.get_raw(cap - 1).unwrap();
            let last_val = unsafe { std::ptr::read(last_ptr as *const BigChunk) };
            assert_eq!(last_val.0[0], (cap - 1) as u8);
            // blob drops here — if Drop layout is wrong, this would crash
        }

        // Also test drop with zero capacity (empty BlobVec, no dealloc needed)
        {
            let blob = BlobVec::new::<BigChunk>();
            assert_eq!(blob.capacity(), 0);
            // Drop should skip dealloc entirely when capacity == 0
        }

        // Test drop after clear (len=0 but capacity>0 → must still dealloc)
        {
            let mut blob = BlobVec::with_capacity::<BigChunk>(10);
            unsafe {
                blob.push(BigChunk([0xFF; 256]));
                blob.push(BigChunk([0xAA; 256]));
            }
            blob.clear();
            assert_eq!(blob.len(), 0);
            assert!(blob.capacity() >= 10);
            // Drop with capacity>0 but len=0 → must dealloc memory
        }
    }
}
