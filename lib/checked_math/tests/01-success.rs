use checked_math::checked_math;

fn f() -> u8 {
    3u8
}

struct S {}
impl S {
    fn m(&self) -> u8 {
        2u8
    }
}

fn main() {
    let num = 2u8;

    let result = checked_math!{ (num + (2u8 / 10)) * 5 };
    assert!(result == Some(10));

    let result = checked_math!{ ((num.pow(20) << 20) + 255) + 2u8 * 2u8 };
    assert!(result == None);

    let result = checked_math!{ -std::i8::MIN };
    assert!(result == None);

    let result = checked_math!{ 12u8 + 6u8 / 3 };
    assert!(result == Some(14));

    let result = checked_math!{ 12u8 + 6u8 / f() };
    assert!(result == Some(14));

    let result = checked_math!{ 12u8 + 6u8 / num };
    assert!(result == Some(15));

    let s = S{};
    let result = checked_math!{ 12u8 + s.m() };
    assert!(result == Some(14));
}
