//! This file contains data repersentations for all prototypes.
//! It also facilitates converts from lua prototypes into rust.

use std::sync::Arc;

use anyhow::Context;
use bevy::color::Color;
use bevy::platform_support::collections::hash_map::Iter;
use bevy::prelude::*;
use bevy::platform_support::collections::HashMap;
use mlua::FromLua;

use super::lua_conversions::LuaColor;

/// Prototypes are assembled from lua with a pipeline system.
/// This struct repersents stage 1:
/// Raw protypes from lua are converted into a Rust datatype.
pub trait RawPrototype {}

/// Prototypes are assembled from lua with a pipeline system.
/// This struct repersents stage 2:
/// Raw protypes from lua are converted into a Rust datatype.
pub trait Prototype
where
    Self: PartialEq,
{
}

pub(super) trait PrototypesBuilder {
    type BuiltFrom: RawPrototype;
    type Final: Prototypes;
    fn new() -> Self;
    fn add(&mut self, prototype: Self::BuiltFrom);
    fn build(self) -> Self::Final;
}

pub trait Prototypes where {
    type T: Prototype;
    fn get(&self, name: &str) -> Option<&'static Self::T>;
    fn iter(&self) -> Iter<'_, std::boxed::Box<str>, &'static Self::T>;
}

#[derive(Resource, Clone)]
pub struct BlockPrototypes(Arc<HashMap<Box<str>, &'static BlockPrototype>>);

impl Prototypes for BlockPrototypes {
    type T = BlockPrototype;

    fn get(&self, name: &str) -> Option<&'static BlockPrototype> {
        self.0.get(name).map(|v| &**v)
    }

    fn iter(&self) -> Iter<'_, std::boxed::Box<str>, &'static Self::T> {
        self.0.iter()
    }
}

pub(super) struct BlockPrototypesBuilder(usize, HashMap<Box<str>, &'static BlockPrototype>);

impl PrototypesBuilder for BlockPrototypesBuilder {
    type BuiltFrom = RawBlockPrototype;
    type Final = BlockPrototypes;

    fn new() -> Self {
        Self(0, HashMap::default())
    }

    fn add(&mut self, prototype: Self::BuiltFrom) {
        let prototype = BlockPrototype {
            id: u16::try_from(self.0).expect("Only 2^16 block prototypes are allowed."),
            name: prototype.name,
            is_transparent: prototype.is_transparent,
            is_meshable: prototype.is_meshable,
            color: prototype.color,
        };

        let name = prototype.name.clone();
        assert!(
            self.1
                .insert(name.clone(), Box::leak(prototype.into()))
                .is_none(),
            "Prototype {name} registered twice."
        );
        self.0 += 1;
    }

    fn build(self) -> Self::Final {
        BlockPrototypes(Arc::new(self.1))
    }
}

#[derive(Clone)]
pub(super) struct RawBlockPrototype {
    name: Box<str>,
    is_transparent: bool,
    is_meshable: bool,
    color: Color,
}

impl RawPrototype for RawBlockPrototype {}

impl FromLua for RawBlockPrototype {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let error = |message: String| {
            mlua::Error::ToLuaConversionError{
                message: Some(message),
                to: "Rust Block Prototype",
                from: "Lua Block Prototype".to_string(),
            }
        };

        let Some(table) = value.as_table() else { Err(error("Block prototypes are expected to be a table.".to_string()))? };

        let name: Box<str> = table.get::<String>("name").context("Could not parse BlockPrototype::name field.")?.into();
        let is_transparent = table.get::<bool>("is_transparent").context("Could not parse BlockPrototype::is_transparent field.")?;
        let is_meshable = table.get::<bool>("is_meshable").context("Could not parse BlockPrototype::is_meshable field.")?;
        let color: Color = table.get::<LuaColor>("color").context("Could not parse BlockPrototype::color field.")?.into();

        Ok(Self {
            name,
            is_transparent,
            is_meshable,
            color
        })
    }
}

#[derive(Debug)]
pub struct BlockPrototype {
    pub id: u16,
    pub name: Box<str>,
    pub is_transparent: bool,
    pub is_meshable: bool,
    pub color: Color,
}

impl PartialEq for BlockPrototype {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl Prototype for BlockPrototype {}
