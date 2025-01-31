#![allow(clippy::unwrap_used)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use bevy::prelude::*;
use mlua::{FromLua, Lua, Table, Value};
use serde::Deserialize;

use crate::chunk::set_block_registry;

use super::prototypes::{
    BlockPrototypesBuilder, PrototypesBuilder, RawBlockPrototype,
};

pub struct ModLoaderPlugin;

impl Plugin for ModLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, lua_setup);
    }
}

#[derive(Debug)]
struct Mod {
    name: String,
    path: PathBuf,
    //dependancies: Vec<Box<Mod>>,
    //dependants: Vec<Box<Mod>>
}

impl Mod {
    fn from_path(path: &Path) -> Self {
        #[allow(unused)]
        #[derive(Debug, Deserialize)]
        struct ModInfo {
            #[serde(rename = "mod")]
            mod_data: ModData,
            dependencies: HashMap<String, String>,
        }

        #[allow(unused)]
        #[derive(Debug, Deserialize)]
        struct ModData {
            name: String,
            version: String,
            talc_version: String,
            authors: Vec<String>,
            description: String,
            homepage: String,
            repository: String,
            #[serde(default)]
            exclude: Vec<String>,
        }

        let contents = std::fs::read_to_string(path.join("info.toml")).unwrap();
        let mod_info: ModInfo = toml::from_str(&contents).unwrap();

        Self {
            name: mod_info.mod_data.name,
            path: path.to_path_buf(),
        }
    }
}

/*
#[derive(Debug)]
struct ModLoadError {
    offender: Rc<Mod>,
    reason: String
}

impl Display for ModLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to load mods.\n{}\n{}", self.offender.name, self.reason)
    }
}

impl Error for ModLoadError {}

impl From<RhaiError> for ModLoadError {
    fn from(value: RhaiError) -> Self {
        todo!()
    }
}
*/

fn detect_mods() -> Box<[Mod]> {
    let mut mods: Vec<Mod> = vec![];
    let mods_path: PathBuf = "assets/mods".into();

    for entry in fs::read_dir(mods_path).expect("Could not find mods directory.") {
        let entry = entry.expect("Could not find mods directory.");
        let path = entry.path();

        // Check if the entry is a directory
        if path.is_dir() {
            // Check for info.toml in this directory
            let info_toml = path.join("info.toml");
            if info_toml.is_file() {
                mods.push(Mod::from_path(&path));
            }
        }
    }

    mods.into_boxed_slice()
}

fn data_stage(lua: &Lua, mods: &[Mod]) -> Result<()> {
    for mod_ in mods {
        let chunk = fs::read_to_string(mod_.path.join("data.lua"))?;
        lua.load(chunk).exec()?;
    }
    Ok(())
}

fn data_updates_stage(lua: &Lua, mods: &[Mod]) -> Result<()> {
    for mod_ in mods {
        let chunk = fs::read_to_string(mod_.path.join("data_updates.lua"))?;
        lua.load(chunk).exec()?;
    }
    Ok(())
}

fn data_final_fixes_stage(lua: &Lua, mods: &[Mod]) -> Result<()> {
    for mod_ in mods {
        let chunk = fs::read_to_string(mod_.path.join("data_final_fixes.lua"))?;
        lua.load(chunk).exec()?;
    }
    Ok(())
}

fn lua_setup(mut commands: Commands) {
    let mods = detect_mods();

    let lua = Lua::new();
    lua.enable_jit(true);

    //engine.set_module_resolver(FileModuleResolver::new_with_path("assets/mods"));

    data_stage(&lua, &mods).expect("Failed to load data stage");
    data_updates_stage(&lua, &mods).expect("Failed to load data updates stage");
    data_final_fixes_stage(&lua, &mods).expect("Failed to load data final fixes stage");

    let globals = lua.globals();
    let data = globals.get::<Table>("data").unwrap();

    let mut block_prototypes = BlockPrototypesBuilder::new();

    data.for_each(|k: String, v: Value| {
        if k == "block" {
            v.as_table().unwrap().for_each(|_: String, v: Value| {
                block_prototypes.add(
                    RawBlockPrototype::from_lua(v, &lua).expect("Could not parse block prototype"),
                );
                Ok(())
            })?;
        }
        Ok(())
    })
    .expect("Found non-string key in data table.");

    let block_prototypes = block_prototypes.build();
    set_block_registry(&block_prototypes);
    commands.insert_resource(block_prototypes);
}
