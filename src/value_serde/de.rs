use indexmap::IndexMap;
use std::fmt::{self, Debug, Formatter};

use crate::value::map::ValueMap;
use crate::value::Value;
use serde::de::{DeserializeSeed, IntoDeserializer, SeqAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

/// from_value
#[inline]
pub fn from_value<T>(val: Value) -> Result<T, crate::Error>
where
    T: for<'de> Deserialize<'de>,
{
    Deserialize::deserialize(&val)
}

#[inline]
pub fn from_value_ref<T>(val: &Value) -> Result<T, crate::Error>
where
    T: for<'de> Deserialize<'de>,
{
    Deserialize::deserialize(val)
}

impl<'de> Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            #[cold]
            fn expecting(&self, fmt: &mut Formatter<'_>) -> Result<(), fmt::Error> {
                Debug::fmt(&"any valid MessagePack value", fmt)
            }

            #[inline]
            fn visit_some<D>(self, de: D) -> Result<Value, D::Error>
            where
                D: serde::de::Deserializer<'de>,
            {
                Deserialize::deserialize(de)
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Bool(value))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::U32(v))
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::U64(value))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::I32(v))
            }

            #[inline]
            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::I64(value))
            }

            #[inline]
            fn visit_f32<E>(self, value: f32) -> Result<Value, E> {
                Ok(Value::F32(value))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::F64(value))
            }

            #[inline]
            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::String(value))
            }

            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(String::from(value))
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut vec = {
                    match visitor.size_hint() {
                        None => {
                            vec![]
                        }
                        Some(l) => Vec::with_capacity(l),
                    }
                };
                while let Some(elem) = visitor.next_element()? {
                    vec.push(elem);
                }
                Ok(Value::Array(vec))
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Binary(v.to_owned()))
            }

            #[inline]
            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Value::Binary(v))
            }

            #[inline]
            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut pairs = {
                    match visitor.size_hint() {
                        None => IndexMap::new(),
                        Some(l) => IndexMap::with_capacity(l),
                    }
                };
                while let Some(key) = visitor.next_key()? {
                    let val = visitor.next_value()?;
                    pairs.insert(key, val);
                }

                Ok(Value::Map(ValueMap(pairs)))
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_newtype_struct("", self)
            }
        }

        de.deserialize_any(ValueVisitor)
    }
}

/// Check if values represent CSV format: first element is array of strings
fn is_csv_format(values: &[Value]) -> bool {
    if values.is_empty() {
        return false;
    }
    match &values[0] {
        Value::Array(arr) if !arr.is_empty() && arr.iter().all(|v| v.is_str()) => true,
        _ => false,
    }
}

/// Convert CSV format `[["k1","k2"],[v1,v2],...]` to `[{k1:v1, k2:v2}, ...]`
fn convert_csv_to_maps(values: Vec<Value>) -> Result<Vec<Value>, crate::Error> {
    if values.len() < 2 {
        return Err(crate::Error::E("CSV must have header and at least one data row".to_string()));
    }

    // First row is headers
    let headers: Vec<String> = match &values[0] {
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
        _ => return Err(crate::Error::E("CSV first row must be array of strings".to_string())),
    };

    if headers.is_empty() {
        return Err(crate::Error::E("CSV headers cannot be empty".to_string()));
    }

    let mut maps = Vec::with_capacity(values.len() - 1);
    for (idx, row) in values.iter().skip(1).enumerate() {
        let row_arr = match row {
            Value::Array(arr) => arr,
            _ => return Err(crate::Error::E(format!("CSV row {} must be Array", idx + 1))),
        };

        let mut map = ValueMap::with_capacity(headers.len());
        for (col_idx, header) in headers.iter().enumerate() {
            let value = row_arr.get(col_idx).cloned().unwrap_or(Value::Null);
            map.insert(Value::String(header.clone()), value);
        }
        maps.push(Value::Map(map));
    }

    Ok(maps)
}

/// SeqAccess that handles both normal arrays and CSV format
struct SeqAccessDeserializer {
    values: Vec<Value>,
    index: usize,
    csv_mode: bool,
}

impl SeqAccessDeserializer {
    fn new(values: Vec<Value>) -> Self {
        let csv_mode = is_csv_format(&values);
        Self {
            values,
            index: 0,
            csv_mode,
        }
    }
}

impl<'de> SeqAccess<'de> for SeqAccessDeserializer {
    type Error = crate::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        // In CSV mode, convert remaining values to maps on first call
        if self.csv_mode && self.index == 0 {
            let csv_values = convert_csv_to_maps(std::mem::take(&mut self.values))?;
            self.values = csv_values;
            self.csv_mode = false;
        }

        if self.index >= self.values.len() {
            return Ok(None);
        }

        let val = &self.values[self.index];
        self.index += 1;
        seed.deserialize(val).map(Some)
    }
}

impl<'de> Deserializer<'de> for &Value {
    type Error = crate::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            Value::Bool(v) => visitor.visit_bool(*v),
            Value::I32(v) => visitor.visit_i32(*v),
            Value::I64(v) => visitor.visit_i64(*v),
            Value::U32(v) => visitor.visit_u32(*v),
            Value::U64(v) => visitor.visit_u64(*v),
            Value::F32(v) => visitor.visit_f32(*v),
            Value::F64(v) => visitor.visit_f64(*v),
            Value::String(v) => visitor.visit_str(v),
            Value::Binary(v) => visitor.visit_bytes(v),
            Value::Array(v) => {
                let len = v.len();
                let mut de = SeqAccessDeserializer::new(v.clone());
                let ret = visitor.visit_seq(&mut de);
                // Check if all elements were consumed (using actual values length, not original)
                if de.index >= de.values.len() || ret.is_err() {
                    ret
                } else {
                    Err(serde::de::Error::invalid_length(
                        len,
                        &"fewer elements in array",
                    ))
                }
            }
            Value::Map(v) => {
                let len = v.len();
                let mut de = MapDeserializer::new(v.into_iter());
                let map = visitor.visit_map(&mut de)?;
                if de.iter.len() == 0 {
                    Ok(map)
                } else {
                    Err(serde::de::Error::invalid_length(
                        len,
                        &"fewer elements in map",
                    ))
                }
            }
            Value::Ext(_tag, data) => Deserializer::deserialize_any(&*data.as_ref(), visitor),
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let v = match self {
            Value::String(v) => visitor.visit_enum(EnumDeserializer {
                variant: v.as_str(),
                value: Some(Value::String(v.clone())),
            }),
            Value::Map(m) => {
                if let Some((v, _)) = m.0.iter().next() {
                    let variant = v.as_str().unwrap_or_default();
                    visitor.visit_enum(EnumDeserializer {
                        variant: variant,
                        value: Some(Value::Map(m.clone())),
                    })
                } else {
                    return Err(serde::de::Error::invalid_type(
                        Unexpected::Other(&format!("{:?}", m)),
                        &"must be object map {\"Key\":\"Value\"}",
                    ));
                }
            }
            _ => {
                return Err(serde::de::Error::invalid_type(
                    Unexpected::Other(&format!("{:?}", self)),
                    &"string or map",
                ));
            }
        };
        v
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let iter = self.into_iter();
        if iter.len() == 0 {
            visitor.visit_unit()
        } else {
            Err(serde::de::Error::invalid_type(
                Unexpected::Seq,
                &"empty array",
            ))
        }
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit seq
        bytes byte_buf map tuple_struct struct
        identifier tuple ignored_any
    }
}

struct MapDeserializer<'a> {
    val: Option<&'a Value>,
    key: Option<&'a Value>,
    iter: indexmap::map::Iter<'a, Value, Value>,
}

impl<'a> MapDeserializer<'a> {
    fn new(m: indexmap::map::Iter<'a, Value, Value>) -> Self {
        Self {
            key: None,
            val: None,
            iter: m,
        }
    }
}

impl<'de, 'a> serde::de::MapAccess<'de> for MapDeserializer<'a> {
    type Error = crate::Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, val)) => {
                self.val = Some(val);
                self.key = Some(key);
                seed.deserialize(*self.key.as_ref().unwrap()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.val.take() {
            Some(val) => seed.deserialize(val).map_err(|mut e| {
                if let Some(key) = self.key.as_ref() {
                    e = e.append(", key = `");
                    e = e.append((*key).as_str().unwrap_or_default());
                    e = e.append("`");
                }
                e
            }),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }
}

impl<'de, 'a> Deserializer<'de> for MapDeserializer<'a> {
    type Error = crate::Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit option
        seq bytes byte_buf map unit_struct newtype_struct
        tuple_struct struct identifier tuple enum ignored_any
    }
}

struct EnumDeserializer<'a> {
    variant: &'a str,
    value: Option<Value>,
}

impl<'de, 'a> serde::de::EnumAccess<'de> for EnumDeserializer<'a> {
    type Error = crate::Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer), crate::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

struct VariantDeserializer {
    value: Option<Value>,
}

impl<'de> serde::de::VariantAccess<'de> for VariantDeserializer {
    type Error = crate::Error;

    fn unit_variant(self) -> Result<(), crate::Error> {
        match self.value {
            Some(_v) => Ok(()),
            None => Err(serde::de::Error::invalid_value(
                Unexpected::Other(&format!("none")),
                &"not support",
            )),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, crate::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.value {
            Some(v) => {
                let m = v.into_map();
                if let Some(m) = m {
                    let mut v = m.0;
                    if let Some(item) = v.pop() {
                        seed.deserialize(&item.1)
                    } else {
                        Err(serde::de::Error::custom(format!(
                            "Deserialize newtype_variant must be {}, and len = 1",
                            "{\"key\",\"v\"}"
                        )))
                    }
                } else {
                    Err(serde::de::Error::custom(format!(
                        "Deserialize newtype_variant must be {}, and len = 1",
                        "{\"key\",\"v\"}"
                    )))
                }
            }
            None => Err(serde::de::Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, crate::Error>
    where
        V: Visitor<'de>,
    {
        //todo impl tuple_variant
        return Err(crate::Error::E(
            "rbs Deserialize unimplemented tuple_variant".to_string(),
        ));
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, crate::Error>
    where
        V: Visitor<'de>,
    {
        //todo impl struct_variant
        return Err(crate::Error::E(
            "rbs Deserialize unimplemented struct_variant".to_string(),
        ));
    }
}
