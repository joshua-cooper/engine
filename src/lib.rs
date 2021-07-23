pub mod account;
pub mod engine;
pub mod event;

use {
    self::engine::{Engine, EngineError},
    derive_more::{Add, AddAssign, AsRef, Display, From, FromStr, Into, Sub, SubAssign},
    rust_decimal::Decimal,
    std::io::{Read, Write},
};

#[derive(Debug, Display, Default, Clone, Copy, PartialEq, Eq, Hash, FromStr, From, Into, AsRef)]
pub struct ClientId(u16);

#[derive(Debug, Display, Clone, Default, Copy, PartialEq, Eq, Hash, FromStr, From, Into, AsRef)]
pub struct TransactionId(u32);

#[derive(
    Debug,
    Display,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    FromStr,
    From,
    Into,
    AsRef,
    Add,
    Sub,
    AddAssign,
    SubAssign,
)]
pub struct Amount(Decimal);

pub fn run(mut reader: impl Read, mut writer: impl Write) -> Result<(), EngineError> {
    let mut engine = Engine::new();
    engine.read_events(&mut reader)?;
    engine.write_accounts_state(&mut writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn single_account() {
        let events = "\
            type,       client, tx, amount
            deposit,    1,      1,  100.1234
            withdrawal, 1,      2,  50
            dispute,    1,      1
            resolve,    1,      1
            withdrawal, 1,      4,  50
            withdrawal, 1,      6,  50
            withdrawal, 1,      8,  0.1
            deposit,    1,      9,  12.92
            dispute,    1,      9 \
        ";

        let expected = "\
            client,available,held,total,locked\n\
            1,0.0234,12.92,12.9434,false\n\
        ";
        let mut actual = Vec::new();
        crate::run(events.as_bytes(), &mut actual).unwrap();
        let actual = std::str::from_utf8(&actual).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn single_account_gets_locked() {
        let events = "\
            type,       client, tx, amount
            deposit,    1,      1,  100.1234
            deposit,    1,      2,  20
            dispute,    1,      2,
            chargeback, 1,      2
            withdrawal, 1,      3,  10
            deposit,    1,      4,  10
            withdrawal, 1,      5,  50 \
        ";

        let expected = "\
            client,available,held,total,locked\n\
            1,100.1234,0,100.1234,true\n\
        ";

        let mut actual = Vec::new();
        crate::run(events.as_bytes(), &mut actual).unwrap();
        let actual = std::str::from_utf8(&actual).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn multiple_accounts() {
        let events = "\
            type,       client, tx, amount
            deposit,    1,      1, 101.291
            deposit,    1,      2, 101.291
            deposit,    1,      3, 101.291
            dispute,    1,      3
            deposit,    2,      4, 55.55
            dispute,    2,      4
            withdrawal, 2,      5, 10
            chargeback, 1,      3
            resolve,    2,      4
            withdrawal, 2,      6, 10 \
        ";

        // Order of rows may not be guaranteed so both possibilities should be checked.
        let expected1 = "\
            client,available,held,total,locked\n\
            1,202.582,0.000,202.582,true\n\
            2,45.55,0.00,45.55,false\n\
        ";

        let expected2 = "\
            client,available,held,total,locked\n\
            2,45.55,0.00,45.55,false\n\
            1,202.582,0.000,202.582,true\n\
        ";

        let mut actual = Vec::new();
        crate::run(events.as_bytes(), &mut actual).unwrap();
        let actual = std::str::from_utf8(&actual).unwrap();

        assert!(expected1 == actual || expected2 == actual);
    }
}
