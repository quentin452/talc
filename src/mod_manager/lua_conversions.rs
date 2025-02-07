//! Provides conversions from lua tables into various rust types.

use bevy::color::Color;
use mlua::FromLua;

pub(super) struct LuaColor {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

impl FromLua for LuaColor {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        let error = |message: String| mlua::Error::ToLuaConversionError {
            message: Some(message),
            to: "Rust Color",
            from: "Lua Value".to_string(),
        };

        let Some(table) = value.as_table() else {
            Err(error("Colors are expected to be a table.".to_string()))?
        };

        let red = table
            .get::<f32>("r")
            .unwrap_or_else(|_| table.get::<f32>(1).unwrap_or(1.0));
        let green = table
            .get::<f32>("g")
            .unwrap_or_else(|_| table.get::<f32>(2).unwrap_or(1.0));
        let blue = table
            .get::<f32>("b")
            .unwrap_or_else(|_| table.get::<f32>(3).unwrap_or(1.0));
        let alpha = table
            .get::<f32>("a")
            .unwrap_or_else(|_| table.get::<f32>(4).unwrap_or(1.0));

        let any_above_1 = red > 1. || green > 1. || blue > 1. || alpha > 1.;

        let rescale_channel = |channel: f32| match channel {
            ..0. => Err(error(format!(
                "Found a color channel below minimum value {channel} < 0"
            ))),
            0.0..=255.0 => {
                if any_above_1 {
                    Ok(channel / 255.)
                } else {
                    Ok(channel)
                }
            }
            255.0.. => Err(error(format!(
                "Found a color channel above maximum value {channel} > 255"
            ))),
            _ => Err(error(format!("Invalid color channel {channel}."))),
        };

        let red = rescale_channel(red)?;
        let green = rescale_channel(green)?;
        let blue = rescale_channel(blue)?;
        let alpha = rescale_channel(alpha)?;

        Ok(Self {
            red,
            green,
            blue,
            alpha,
        })
    }
}

impl From<LuaColor> for Color {
    fn from(value: LuaColor) -> Self {
        Self::srgba(value.red, value.green, value.blue, value.alpha)
    }
}
