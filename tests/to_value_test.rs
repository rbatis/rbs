use rbs::to_value;

#[test]
fn test_to_value() {
    let v= to_value!{1};
    let v2 =  to_value(1).unwrap();
    assert_eq!(v, v2);
}