//! A high-level API for Modbus Register Maps based on [tokio-modbus](https://github.com/slowtec/tokio-modbus).

//! ## Traits
//!
//! The library defines
//!
//! - [`core::InputRegisterMap`] and [`core::HoldingRegisterMap`] traits to read from (and write to) the Modbus registers in batch, and
//! - [`simulator::InputRegisterModel`] and [`simulator::HoldingRegisterModel`] traits to simulate a Modbus device
//!
//! ## Derive macro
//!
//! For convenience it provides derive macros to implement the traits automatically. The derive macros depends on `modbus` field attribute.
//!
//! See [examples/](https://github.com/vladimirvrabely/modbus-mapping/tree/main/modbus-mapping/examples) for simple usage.
//!
//! The `modbus` attributes **can** be added to struct fields to link them with modbus register mapping entries.
//! Then, the field `modbus` **must** contain the following key-values pairs:
//! - `addr` - input or holding register start address, `u16` integer,
//! - `ty` - modbus data type, one of `"i16"`, `"i32"`, `"i64"`, `"u16"`, `"u32"`, `"u64"`, `"f16"`, `"f32"` or `"raw(size)"`,
//! - `ord` - word order, either `"be"` for big-endian or `"le"` for little-endian,
//! - `x` - scale factor; multiply the stored value by it to get the actual value
//! - `unit` - measurement unit of the actual value (i.e. actual value = stored value x scale factor)
//!
//! The struct `modbus` attribute is optional and provides configuration for `InputRegisterMap` and `HoldingRegisterMap` traits when reading registers.
//! It  **can only** contain these key-value pairs:
//! - `max_cnt_per_request` - maximum number of registers to read in a single Modbus request; default value is `123` which is the maximumum allowed value
//! - `allow_register_gaps` - an optimization flag to allow Modbus client to read longer register blocks which possibly contain unrequested (or undefined) registers in between the required ones.
//!   If `true`, the client makes less requests but read more data. Otherwise, if `false`, the client makes more requests but read only the necessary data.
//!
//! The `modbus_doc` attribute is to create documentation (by adding doc attribute) from `modbus` field attributes information.

/// Utilities for encoding from and decoding to Modbus registers
pub mod codec;
/// Core traits to read from and write to Modbus registers
pub mod core;

/// Traits and utilities to create device simulator (based on tokio-modbus [servers examples](https://github.com/slowtec/tokio-modbus/tree/main/examples))
#[cfg(feature = "simulator")]
pub mod simulator;

pub mod derive {
    /// Re-export.
    pub use modbus_mapping_derive::{
        modbus_doc, HoldingRegisterMap, HoldingRegisterModel, InputRegisterMap, InputRegisterModel,
    };
}
