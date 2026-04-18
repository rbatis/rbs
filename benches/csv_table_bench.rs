#![feature(test)]
extern crate test;

use rbs::{from_value, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MockTable {
    pub name: String,
    pub sql: String,
}

fn create_csv_values(row_count: usize) -> Value {
    let mut arr = Vec::with_capacity(row_count + 1);
    arr.push(Value::Array(vec!["name".into(), "sql".into()]));
    for i in 0..row_count {
        arr.push(Value::Array(vec![
            format!("n{}", i).into(),
            format!("s{}", i).into(),
        ]));
    }
    Value::Array(arr)
}

fn create_map_values(row_count: usize) -> Value {
    use rbs::value::map::ValueMap;
    let mut arr = Vec::with_capacity(row_count);
    for i in 0..row_count {
        let mut m = ValueMap::with_capacity(2);
        m.insert("name".into(), format!("n{}", i).into());
        m.insert("sql".into(), format!("s{}", i).into());
        arr.push(Value::Map(m));
    }
    Value::Array(arr)
}

//test bench_from_csv_10000_rows ... bench:  10,697,820.00 ns/iter (+/- 370,886.00)
#[bench]
fn bench_from_csv_10000_rows(b: &mut test::Bencher) {
    let values = create_csv_values(10000);
    b.iter(|| {
        let _: Vec<MockTable> = from_value(values.clone()).unwrap();
    });
}

//test bench_from_map_10000_rows ... bench:  14,355,790.00 ns/iter (+/- 728,740.00)
#[bench]
fn bench_from_map_10000_rows(b: &mut test::Bencher) {
    let values = create_map_values(10000);
    b.iter(|| {
        let _: Vec<MockTable> = from_value(values.clone()).unwrap();
    });
}
