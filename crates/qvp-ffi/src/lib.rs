//! C ABI over `qvp-core`. The single binding surface for wasm (web), Swift,
//! Kotlin (JNI shim), Dart FFI and the React Native shim. See `include/qvp.h`.
//!
//! Conventions: colours are 0xRRGGBBAA; `QVP_NONE` (0xFFFFFFFF) means absent;
//! strings come back through `QvpStr` (pointer + length, UTF-8, not NUL
//! terminated) and stay valid until the next call that returns a string on the
//! same thread; array outputs take a caller buffer + capacity and return the
//! total count (which may exceed the capacity — call again with a bigger buffer).
#![allow(clippy::missing_safety_doc)]

// One module per section of the header. `include/qvp.h` gives the order; rustfmt sorts the list.
mod abi;
mod atlas;
mod convert;
mod crop;
mod dress;
mod highlight;
mod hit;
mod layout;
mod memory;
mod names;
mod page;
mod passage;
mod style;
mod text;
mod types;
mod view;

pub use abi::QvpStr;
pub use atlas::*;
pub use crop::*;
pub use dress::*;
pub use highlight::*;
pub use hit::*;
pub use layout::*;
pub use memory::*;
pub use names::*;
pub use page::*;
pub use passage::*;
pub use style::*;
pub use text::*;
pub use types::*;
pub use view::*;
