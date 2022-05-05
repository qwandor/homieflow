// Copyright 2022 the homieflow authors.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use google_smart_home::{
    device::commands::{ColorAbsolute, ColorValue},
    query::response::Color,
};
use homie_controller::{ColorFormat, ColorHsv, ColorRgb, Device, Node, Property};
use std::collections::HashMap;

/// Given an ID of the form `"device_id/node_id"`, looks up the corresponding Homie node (if any).
pub fn get_homie_device_by_id<'a>(
    devices: &'a HashMap<String, Device>,
    id: &str,
) -> Option<(&'a Device, &'a Node)> {
    let id_parts: Vec<_> = id.split('/').collect();
    if let [device_id, node_id] = id_parts.as_slice() {
        if let Some(device) = devices.get(*device_id) {
            if let Some(node) = device.nodes.get(*node_id) {
                return Some((device, node));
            }
        }
    }
    None
}

/// Converts the value of the given property to a Google Home JSON color value, if it is the
/// appropriate type.
pub fn property_value_to_color(property: &Property) -> Option<Color> {
    let color_format = property.color_format().ok()?;
    let color_value = match color_format {
        ColorFormat::Rgb => {
            let rgb: ColorRgb = property.value().ok()?;
            let rgb_int = ((rgb.r as u32) << 16) + ((rgb.g as u32) << 8) + (rgb.b as u32);
            Color::SpectrumRgb(rgb_int)
        }
        ColorFormat::Hsv => {
            let hsv: ColorHsv = property.value().ok()?;
            Color::SpectrumHsv {
                hue: hsv.h.into(),
                saturation: hsv.s as f64 / 100.0,
                value: hsv.v as f64 / 100.0,
            }
        }
    };
    Some(color_value)
}

/// Converts a Google Home `ColorAbsolute` command to the appropriate value to set on the given
/// Homie property, if it is the appropriate format.
pub fn color_absolute_to_property_value(
    property: &Property,
    color_absolute: &ColorAbsolute,
) -> Option<String> {
    let color_format = property.color_format().ok()?;
    match color_format {
        ColorFormat::Rgb => {
            if let ColorValue::Rgb { spectrum_rgb } = color_absolute.color.value {
                let rgb = ColorRgb::new(
                    (spectrum_rgb >> 16) as u8,
                    (spectrum_rgb >> 8) as u8,
                    spectrum_rgb as u8,
                );
                return Some(rgb.to_string());
            }
        }
        ColorFormat::Hsv => {
            if let ColorValue::Hsv { spectrum_hsv } = &color_absolute.color.value {
                let hsv = ColorHsv::new(
                    spectrum_hsv.hue as u16,
                    (spectrum_hsv.saturation * 100.0) as u8,
                    (spectrum_hsv.value * 100.0) as u8,
                );
                return Some(hsv.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use google_smart_home::{
        device::commands::{Color, Hsv},
        query,
    };
    use homie_controller::Datatype;

    use super::*;

    #[test]
    fn color_rgb() {
        let property = Property {
            id: "color".to_string(),
            name: Some("Colour".to_string()),
            datatype: Some(Datatype::Color),
            settable: true,
            retained: true,
            unit: None,
            format: Some("rgb".to_string()),
            value: Some("17,34,51".to_string()),
        };

        assert_eq!(
            property_value_to_color(&property),
            Some(query::response::Color::SpectrumRgb(0x112233))
        );
        assert_eq!(
            color_absolute_to_property_value(
                &property,
                &ColorAbsolute {
                    color: Color {
                        name: None,
                        value: ColorValue::Rgb {
                            spectrum_rgb: 0x445566
                        }
                    }
                }
            ),
            Some("68,85,102".to_string())
        );
    }

    #[test]
    fn color_hsv() {
        let property = Property {
            id: "color".to_string(),
            name: Some("Colour".to_string()),
            datatype: Some(Datatype::Color),
            settable: true,
            retained: true,
            unit: None,
            format: Some("hsv".to_string()),
            value: Some("280,50,60".to_string()),
        };

        assert_eq!(
            property_value_to_color(&property),
            Some(query::response::Color::SpectrumHsv {
                hue: 280.0,
                saturation: 0.5,
                value: 0.6
            })
        );
        assert_eq!(
            color_absolute_to_property_value(
                &property,
                &ColorAbsolute {
                    color: Color {
                        name: None,
                        value: ColorValue::Hsv {
                            spectrum_hsv: Hsv {
                                hue: 290.0,
                                saturation: 0.2,
                                value: 0.3
                            }
                        }
                    }
                }
            ),
            Some("290,20,30".to_string())
        );
    }
}
