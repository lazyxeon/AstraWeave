//! Component type metadata for type-erased BlobVec operations.
//!
//! This module provides the runtime type information needed to safely
//! work with BlobVec storage without knowing concrete types at compile time.
//!
//! # Architecture
//!
//! ```text
//! ComponentMeta<T>                         BlobVec
//! ┌────────────────┐                      ┌────────────────┐
//! │ layout         │ ──────────────────── │ item_layout    │
//! │ drop_fn        │ ──────────────────── │ drop_fn        │
//! │ clone_fn       │ (for archetype move) │ data           │
//! └────────────────┘                      └────────────────┘
//! ```
//!
//! # Safety
//!
//! This module uses unsafe code to provide type-erased operations.
//! All safety invariants are documented and enforced through the API design.

use std::alloc::Layout;
use std::any::TypeId;
use std::collections::HashMap;

use crate::blob_vec::BlobVec;
use crate::Component;

/// Metadata for a component type, enabling type-erased BlobVec operations.
///
/// This is the key to replacing `Box<dyn Any>` with `BlobVec`:
/// - Stores layout for proper allocation
/// - Stores drop function for correct cleanup
/// - Stores clone function for archetype transitions
#[derive(Clone)]
pub struct ComponentMeta {
    /// Memory layout of the component type
    pub layout: Layout,
    /// Drop function (None for types that don't need drop)
    pub drop_fn: Option<unsafe fn(*mut u8)>,
    /// Clone function for copying during archetype transitions
    /// Takes (src, dst) pointers
    pub clone_fn: unsafe fn(*const u8, *mut u8),
    /// Human-readable type name for debugging
    pub type_name: &'static str,
}

impl ComponentMeta {
    /// Create ComponentMeta for a specific component type.
    ///
    /// # Example
    /// ```
    /// use astraweave_ecs::component_meta::ComponentMeta;
    ///
    /// #[derive(Clone, Copy)]
    /// struct Position { x: f32, y: f32 }
    ///
    /// let meta = ComponentMeta::of::<Position>();
    /// assert_eq!(meta.layout.size(), std::mem::size_of::<Position>());
    /// ```
    pub fn of<T: Component + Clone>() -> Self {
        Self {
            layout: Layout::new::<T>(),
            drop_fn: if std::mem::needs_drop::<T>() {
                Some(Self::drop_fn_impl::<T>)
            } else {
                None
            },
            clone_fn: Self::clone_fn_impl::<T>,
            type_name: std::any::type_name::<T>(),
        }
    }

    /// Drop function implementation for type T.
    ///
    /// # Safety
    /// `ptr` must point to a valid, initialized value of type T.
    unsafe fn drop_fn_impl<T>(ptr: *mut u8) {
        ptr.cast::<T>().drop_in_place();
    }

    /// Clone function implementation for type T.
    ///
    /// # Safety
    /// - `src` must point to a valid, initialized value of type T
    /// - `dst` must be properly aligned and have enough space for T
    unsafe fn clone_fn_impl<T: Clone>(src: *const u8, dst: *mut u8) {
        let src_ref = &*src.cast::<T>();
        let cloned = src_ref.clone();
        dst.cast::<T>().write(cloned);
    }

    /// Create a BlobVec with this component's layout and drop function.
    pub fn create_blob_vec(&self) -> BlobVec {
        BlobVec::from_layout(self.layout, self.drop_fn)
    }

    /// Create a BlobVec with pre-allocated capacity.
    pub fn create_blob_vec_with_capacity(&self, capacity: usize) -> BlobVec {
        BlobVec::from_layout_with_capacity(self.layout, self.drop_fn, capacity)
    }
}

/// Registry mapping TypeId to ComponentMeta.
///
/// This is used by ArchetypeStorage to create properly-typed BlobVecs
/// for any component type encountered at runtime.
#[derive(Default)]
pub struct ComponentMetaRegistry {
    metas: HashMap<TypeId, ComponentMeta>,
}

impl ComponentMetaRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a component type.
    ///
    /// Returns true if the type was newly registered, false if already present.
    pub fn register<T: Component + Clone>(&mut self) -> bool {
        let type_id = TypeId::of::<T>();
        if self.metas.contains_key(&type_id) {
            return false;
        }
        self.metas.insert(type_id, ComponentMeta::of::<T>());
        true
    }

    /// Get metadata for a component type.
    ///
    /// Returns None if the type is not registered.
    #[inline]
    pub fn get(&self, type_id: TypeId) -> Option<&ComponentMeta> {
        self.metas.get(&type_id)
    }

    /// Check if a component type is registered.
    #[inline]
    pub fn is_registered(&self, type_id: TypeId) -> bool {
        self.metas.contains_key(&type_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Health {
        current: i32,
        max: i32,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct WithDrop {
        value: i32,
        data: Vec<u8>,
    }

    #[test]
    fn test_component_meta_layout() {
        let meta = ComponentMeta::of::<Position>();
        assert_eq!(meta.layout.size(), std::mem::size_of::<Position>());
        assert_eq!(meta.layout.align(), std::mem::align_of::<Position>());
        assert!(meta.drop_fn.is_none()); // Copy type doesn't need drop
    }

    #[test]
    fn test_component_meta_drop_fn() {
        let meta = ComponentMeta::of::<WithDrop>();
        assert!(meta.drop_fn.is_some()); // Has Vec, needs drop
    }

    #[test]
    fn test_component_meta_clone_fn() {
        let meta = ComponentMeta::of::<Health>();

        let src = Health {
            current: 80,
            max: 100,
        };
        let mut dst = std::mem::MaybeUninit::<Health>::uninit();

        unsafe {
            (meta.clone_fn)(
                &src as *const Health as *const u8,
                dst.as_mut_ptr() as *mut u8,
            );
            let cloned = dst.assume_init();
            assert_eq!(cloned, src);
        }
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ComponentMetaRegistry::new();

        assert!(registry.register::<Position>());
        assert!(!registry.register::<Position>()); // Already registered

        assert!(registry.is_registered(TypeId::of::<Position>()));
        assert!(!registry.is_registered(TypeId::of::<Health>()));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = ComponentMetaRegistry::new();
        registry.register::<Position>();

        let meta = registry.get(TypeId::of::<Position>()).unwrap();
        assert_eq!(
            meta.type_name,
            "astraweave_ecs::component_meta::tests::Position"
        );

        assert!(registry.get(TypeId::of::<Health>()).is_none());
    }

    #[test]
    fn test_create_blob_vec() {
        let meta = ComponentMeta::of::<Position>();
        let blob = meta.create_blob_vec();
        assert_eq!(blob.len(), 0);
    }

    #[test]
    fn test_create_blob_vec_with_capacity() {
        let meta = ComponentMeta::of::<Position>();
        let blob = meta.create_blob_vec_with_capacity(100);
        assert!(blob.capacity() >= 100);
    }
}
