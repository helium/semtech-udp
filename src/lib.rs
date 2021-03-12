#[macro_use]
extern crate arrayref;

pub type Result<T = ()> = std::result::Result<T, Error>;

mod packet;
pub use packet::*;

#[cfg(feature = "server")]
pub mod server_runtime;

#[cfg(feature = "client")]
pub mod client_runtime;

#[cfg(test)]
mod tests;
