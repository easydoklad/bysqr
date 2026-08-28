//! PAY by square, INVOICE by square and INVOICE ITEMS encoder and decoder.
//!
//! The [`pay`], [`invoice`] and [`invoice_items`] modules expose the typed
//! document APIs. [`decode`] and [`try_deserialize`] provide format-agnostic
//! entry points, while [`qr`] renders encoded payloads as SVG, PNG or JPEG.
//! Enable the `qr-reader` feature to decode supported documents from raster
//! images.

pub mod codec;
pub mod diagnostic;
pub mod document;
pub mod error;
pub mod invoice;
pub mod invoice_items;
pub mod pay;
pub mod qr;
#[cfg(feature = "qr-reader")]
pub mod qr_reader;
#[cfg(feature = "wasm")]
mod wasm;

pub use document::{decode, try_deserialize, Document};
