use std::io::Cursor;

use plist::{Error, Value};
use plist_plus::Plist;

#[macro_export]
macro_rules! ok_or {
    ($result: expr, $return: expr) => {
        match $result {
            Ok(v) => v,
            Err(_) => $return,
        }
    };
}

#[macro_export]
macro_rules! some_or {
    ($result: expr, $log: expr, $return: expr) => {
        match $result {
            Some(v) => v,
            None => {
                $log;
                $return;
            }
        }
    };
}

pub trait RustyPlistConversion {
    /// Converts the bytes to a rusty plist Value.
    ///
    /// # Arguments
    /// - `bytes` - Bytes for the plist
    ///
    /// # Returns
    /// A Result with the rusty plist Value. Errors
    /// are not handled; you are expected to do that
    /// when calling the method using `match`
    fn from_bytes(bytes: &[u8]) -> Result<Value, Error>;

    /// Converts a plist_plus Plist to a rusty plist Value.
    ///
    /// Note: this method converts the Plist to a string,
    /// and then to bytes to then pass to bytes_to_plist.
    /// Turning the Plist into a string was the best method
    /// of getting raw data I could find.
    /// It hasn't been properly tested; it might not work
    /// with binary plists, or with similar edge cases.
    /// (it should work with binary plists since
    /// Plist.to_string() outputs the entire plist as
    /// a string, which would already be converted by plist_plus.)
    ///
    /// # Arguments
    /// - `plist` - A plist_plus Plist to feed into a rusty plist Value
    ///
    /// # Returns
    /// A Result with the rusty plist Value. Errors
    /// are not handled; you are expected to do that
    /// when calling the method using `match`
    fn from_plist_plus(plist: Plist) -> Result<Value, Error>;
}

impl RustyPlistConversion for Value {
    fn from_bytes(bytes: &[u8]) -> Result<Value, Error> {
        Value::from_reader(Cursor::new(bytes))
    }

    fn from_plist_plus(plist: Plist) -> Result<Value, Error> {
        Value::from_bytes(plist.to_string().as_bytes())
    }
}
