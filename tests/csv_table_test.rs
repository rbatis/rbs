use rbs::{from_value, to_value, Value, VecStruct};
use rbs::value::map::ValueMap;

#[test]
fn test_vec_struct_from_array_of_maps() {
    // Format 1: Array of Maps - manual construction
    let map1 = {
        let mut m = ValueMap::new();
        m.insert("type".into(), "table".into());
        m.insert("name".into(), "activity".into());
        m.insert("tbl_name".into(), "activity".into());
        m.insert("sql".into(), "CREATE TABLE...".into());
        Value::Map(m)
    };
    let value = Value::Array(vec![map1]);

    let result: VecStruct = from_value(value).unwrap();
    assert_eq!(result.0.len(), 1);
    assert_eq!(result.0[0]["type"].as_str().unwrap(), "table");
    assert_eq!(result.0[0]["name"].as_str().unwrap(), "activity");
}

#[test]
fn test_vec_struct_from_csv_format() {
    // Format 2: CSV format with headers
    // [["type","name","tbl_name","sql"],["table","activity","activity","CREATE TABLE..."]]
    let headers = Value::Array(vec![
        "type".into(),
        "name".into(),
        "tbl_name".into(),
        "sql".into(),
    ]);
    let row1 = Value::Array(vec![
        "table".into(),
        "activity".into(),
        "activity".into(),
        "CREATE TABLE activity...".into(),
    ]);
    let value = Value::Array(vec![headers, row1]);

    let result: VecStruct = from_value(value).unwrap();
    assert_eq!(result.0.len(), 1);
    assert_eq!(result.0[0]["type"].as_str().unwrap(), "table");
    assert_eq!(result.0[0]["name"].as_str().unwrap(), "activity");
    assert_eq!(result.0[0]["tbl_name"].as_str().unwrap(), "activity");
    assert_eq!(result.0[0]["sql"].as_str().unwrap(), "CREATE TABLE activity...");
}

#[test]
fn test_vec_struct_roundtrip() {
    // Start with Array of Maps
    let map1 = {
        let mut m = ValueMap::new();
        m.insert("type".into(), "table".into());
        m.insert("name".into(), "activity".into());
        Value::Map(m)
    };
    let value = Value::Array(vec![map1]);

    let result: VecStruct = from_value(value).unwrap();
    let output: Value = to_value(result).unwrap();

    // Should serialize back to array of maps
    assert!(output.is_array());
    let arr = output.as_array().unwrap();
    assert!(arr[0].is_map());
}

#[test]
fn test_vec_struct_empty() {
    let value = Value::Array(vec![]);
    let result: VecStruct = from_value(value).unwrap();
    assert!(result.0.is_empty());
}

#[test]
fn test_vec_struct_to_csv_value() {
    let map1 = {
        let mut m = ValueMap::new();
        m.insert("type".into(), "table".into());
        m.insert("name".into(), "activity".into());
        Value::Map(m)
    };
    let value = Value::Array(vec![map1]);

    let result: VecStruct = from_value(value).unwrap();
    let csv = result.to_csv_value();

    // CSV format: first row is headers, second row is data
    let arr = csv.as_array().unwrap();
    assert!(arr[0].is_array()); // headers row
    let headers = arr[0].as_array().unwrap();
    assert_eq!(headers.len(), 2); // "type" and "name"
}

#[test]
fn test_vec_struct_multiple_rows() {
    // CSV format with multiple data rows
    let headers = Value::Array(vec!["type".into(), "name".into()]);
    let row1 = Value::Array(vec!["table".into(), "activity".into()]);
    let row2 = Value::Array(vec!["index".into(), "idx_activity".into()]);
    let value = Value::Array(vec![headers, row1, row2]);

    let result: VecStruct = from_value(value).unwrap();
    assert_eq!(result.0.len(), 2);
    assert_eq!(result.0[0]["type"].as_str().unwrap(), "table");
    assert_eq!(result.0[1]["type"].as_str().unwrap(), "index");
}
