//! AstraWeave ECS
//! Provides Bevy-like API with Query tuples, Res/ResMut

use std::{
    any::TypeId,
    collections::{BTreeMap, HashMap},
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub trait Component: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Component for T {}

pub trait Resource: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Resource for T {}
