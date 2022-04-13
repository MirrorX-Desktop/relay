#[test]
fn bincode_test() {
    let o1: Result<i32, Option<String>> = Result::Ok(1);
    assert_eq!(bincode::serialize(&o1).is_ok(), true)
}
