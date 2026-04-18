use rbs::{from_value, Value};
use serde::{Deserialize, Serialize};

#[test]
fn test_vec_struct_from_array_of_maps() {
    #[derive(Debug, Serialize,Deserialize)]
    pub struct MockTable{
        pub name: String,
        pub sql:String
    }

    let values = Value::Array(vec![
        Value::Array(
        vec![
        "type".to_string().into(), 
        "name".to_string().into(), 
        "sql".to_string().into()
        ]),
        Value::Array(
        vec![
        "t1".to_string().into(), 
        "n1".to_string().into(), 
        "s1".to_string().into()
        ])
    ]);

    let result: Vec<MockTable> = from_value(values).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "n1");
    assert_eq!(result[0].sql, "s1");
}
