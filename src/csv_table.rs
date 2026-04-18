use crate::{Error, Value};
use crate::value::map::ValueMap;
use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper type that deserializes from two Value formats:
/// 1. `[{"key":"value"}, ...]` - Array of Maps (standard)
/// 2. `[["k1","k2"], ["v1","v2"], ...]` - Array of Arrays with header row (CSV format)
///
/// When CSV format is detected (first element is array of strings = headers),
/// subsequent arrays are converted to maps using the headers as keys.
#[derive(Debug, Clone, PartialEq)]
pub struct VecStruct(pub Vec<ValueMap>);

impl VecStruct {
    /// Convert to Value (always outputs Array of Maps format)
    pub fn to_value(&self) -> Value {
        Value::Array(self.0.iter().map(|m| Value::Map(m.clone())).collect())
    }

    /// Try to convert CSV format `[["k1","k2"],[v1,v2]]` to Vec<ValueMap>
    pub fn from_csv_value(val: Value) -> Result<Self, Error> {
        let arr = match val {
            Value::Array(arr) => arr,
            _ => return Err(Error::E("CSV format requires Array".to_string())),
        };

        if arr.is_empty() {
            return Ok(VecStruct(vec![]));
        }

        // Check if first element is an array of strings (headers)
        let is_csv_format = matches!(
            &arr[0],
            Value::Array(first) if !first.is_empty()
                && first.iter().all(|v| v.is_str())
        );

        if !is_csv_format {
            return Err(Error::E(
                "CSV format: first row must be array of string headers".to_string(),
            ));
        }

        let headers: Vec<String> = arr[0]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if headers.is_empty() {
            return Err(Error::E("CSV headers cannot be empty".to_string()));
        }

        let mut rows = Vec::with_capacity(arr.len() - 1);
        for (idx, row) in arr.iter().skip(1).enumerate() {
            let row_arr = match row {
                Value::Array(arr) => arr,
                _ => {
                    return Err(Error::E(format!(
                        "CSV row {} must be Array",
                        idx + 1
                    )))
                }
            };

            let mut map = ValueMap::with_capacity(headers.len());
            for (col_idx, header) in headers.iter().enumerate() {
                let value = row_arr.get(col_idx).unwrap_or(&Value::Null).clone();
                map.insert(Value::String(header.clone()), value);
            }
            rows.push(map);
        }

        Ok(VecStruct(rows))
    }

    /// Convert Vec<ValueMap> to CSV Value format `[[k1,k2,...],[v1,v2,...]]`
    pub fn to_csv_value(&self) -> Value {
        if self.0.is_empty() {
            return Value::Array(vec![]);
        }

        // Collect all unique headers from all rows
        let mut all_headers: Vec<String> = Vec::new();
        for row in &self.0 {
            for (k, _) in row {
                if let Some(key) = k.as_str() {
                    if !all_headers.contains(&key.to_string()) {
                        all_headers.push(key.to_string());
                    }
                }
            }
        }

        let mut result: Vec<Value> = Vec::new();

        // Header row
        result.push(Value::Array(
            all_headers.iter().map(|s| Value::String(s.clone())).collect(),
        ));

        // Data rows
        for row in &self.0 {
            let mut row_values: Vec<Value> = Vec::with_capacity(all_headers.len());
            for header in &all_headers {
                let value = row.0.get(&Value::String(header.clone())).cloned().unwrap_or(Value::Null);
                row_values.push(value);
            }
            result.push(Value::Array(row_values));
        }

        Value::Array(result)
    }
}

impl AsRef<Vec<ValueMap>> for VecStruct {
    fn as_ref(&self) -> &Vec<ValueMap> {
        &self.0
    }
}

impl Into<Vec<ValueMap>> for VecStruct {
    fn into(self) -> Vec<ValueMap> {
        self.0
    }
}

impl From<Vec<ValueMap>> for VecStruct {
    fn from(v: Vec<ValueMap>) -> Self {
        VecStruct(v)
    }
}

impl Serialize for VecStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always serialize as Array of Maps
        self.to_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VecStruct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecStructVisitor;

        impl<'de> Visitor<'de> for VecStructVisitor {
            type Value = VecStruct;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an array of structs or CSV format array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values: Vec<Value> = Vec::new();
                while let Some(elem) = seq.next_element()? {
                    values.push(elem);
                }

                if values.is_empty() {
                    return Ok(VecStruct(vec![]));
                }

                // Check if it's CSV format: first element is Array of string headers
                let is_csv = matches!(
                    values.first(),
                    Some(Value::Array(arr)) if !arr.is_empty()
                        && arr.iter().all(|v| v.is_str())
                );

                if is_csv {
                    // CSV format: convert to Vec<ValueMap>
                    let csv_value = Value::Array(values);
                    VecStruct::from_csv_value(csv_value)
                        .map_err(|e| serde::de::Error::custom(e.to_string()))
                } else {
                    // Standard Array of Maps
                    let maps: Vec<ValueMap> = values
                        .into_iter()
                        .filter_map(|v| v.into_map())
                        .collect();
                    Ok(VecStruct(maps))
                }
            }
        }

        deserializer.deserialize_seq(VecStructVisitor)
    }
}
