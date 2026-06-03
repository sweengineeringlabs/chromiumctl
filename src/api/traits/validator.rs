/// Validates an instance, returning a typed error on failure.
pub trait Validator {
    /// The error type produced when validation fails.
    type Error;

    /// Validate this instance.
    fn validate(&self) -> Result<(), Self::Error>;
}
