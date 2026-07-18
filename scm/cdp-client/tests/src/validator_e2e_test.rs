#![allow(clippy::unwrap_used)]

use cdp_client::Validator;

struct AgeGuard(i32);

impl Validator for AgeGuard {
    type Error = String;

    fn validate(&self) -> Result<(), String> {
        if self.0 < 0 {
            Err(format!("age must be non-negative, got {}", self.0))
        } else {
            Ok(())
        }
    }
}

#[test]
fn test_validator_returns_ok_for_valid_value() {
    assert!(AgeGuard(25).validate().is_ok());
}

#[test]
fn test_validator_returns_err_for_invalid_value() {
    let err = AgeGuard(-1).validate().unwrap_err();
    assert!(err.contains("-1"), "error should mention the bad value: {err}");
}

#[test]
fn test_validator_zero_is_valid() {
    assert!(AgeGuard(0).validate().is_ok());
}
