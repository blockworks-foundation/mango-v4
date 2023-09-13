use mango_v4::types::I80F48;

#[test]
fn fixed_error() {

    let a: u64 = 66000;
    let b: u64 = 1000;
    assert!(I80F48::from(a) > b); // fails!


}
