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

use prost_types::{value::Kind, ListValue, Struct, Value};
use serde_json::Map;

pub fn json_to_prost_value(value: serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value { kind: None },
        serde_json::Value::Bool(v) => Value {
            kind: Some(Kind::BoolValue(v)),
        },
        serde_json::Value::Number(number) => Value {
            kind: Some(Kind::NumberValue(
                number.as_f64().expect("Number can't be represented as f64"),
            )),
        },
        serde_json::Value::String(v) => Value {
            kind: Some(Kind::StringValue(v)),
        },
        serde_json::Value::Array(array) => Value {
            kind: Some(Kind::ListValue(json_to_prost_list(array))),
        },
        serde_json::Value::Object(object) => Value {
            kind: Some(Kind::StructValue(json_to_prost_struct(object))),
        },
    }
}

pub fn json_to_prost_list(array: Vec<serde_json::Value>) -> ListValue {
    ListValue {
        values: array.into_iter().map(json_to_prost_value).collect(),
    }
}

pub fn json_to_prost_struct(object: Map<String, serde_json::Value>) -> Struct {
    Struct {
        fields: object
            .into_iter()
            .map(|(key, value)| (key, json_to_prost_value(value)))
            .collect(),
    }
}
