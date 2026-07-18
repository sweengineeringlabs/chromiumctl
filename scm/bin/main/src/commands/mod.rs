pub mod launch;
pub mod eval;
pub mod screenshot;
pub mod navigate;
pub mod wait;
pub mod click;
pub mod input;
pub mod get_dom;
pub mod metrics;
pub mod mock;
pub mod reap;
pub mod set_files;
pub mod stop;

mod args;
mod connection;
mod error;

pub use args::{expect_value, parse_value, validate_connect_args};
pub use connection::attach;
pub use error::CliError;
