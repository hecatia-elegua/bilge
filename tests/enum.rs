use bilge::prelude::*;

#[bitsize(1)]
#[derive(FromBits, PartialEq, Debug)]
enum Date { No, Yes }

#[bitsize(2)]
#[derive(TryFromBits)]
enum Activity { Restaurant, Skating, Movies }

#[test]
fn conversions() {
    // `From` in both directions
    assert_eq!(u1::new(0), u1::from(Date::No));
    assert_eq!(u1::new(1), u1::from(Date::Yes));
    assert_eq!(Date::No,   u1::new(0).into());
    assert_eq!(Date::Yes,  u1::new(1).into());

    // Of course converting to number is always infallible,
    // since the discriminants need to fit the bitsize anyways.
    // `TryFrom<uN>` and `From<Enum>`
    for value in 0..4{
        let value = u2::new(value);
        let date_activity = Activity::try_from(value);
        match date_activity {
            Ok(a) => {
                match a {
                    Activity::Restaurant => assert_eq!(u2::new(0u8), value),
                    Activity::Skating =>    assert_eq!(u2::new(1u8), value),
                    Activity::Movies =>     assert_eq!(u2::new(2u8), value),
                }
                assert_eq!(u2::from(a), value);
            },
            Err(e) => assert_eq!(format!("{e:?}"), "BitsError"),
        }
    }
}
