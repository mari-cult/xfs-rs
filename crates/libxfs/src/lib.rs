#![no_std]

#[cfg(any(test, feature = "std"))]
extern crate std;

pub mod device;
pub mod endian;
pub mod crc;
pub mod error;
pub mod geometry;
pub mod on_disk;
pub mod reader;

pub use device::BlockDevice;
#[cfg(feature = "std")]
pub use device::StdFileDevice;
pub use error::{DeviceError, ParseError, ReadError};
pub use reader::{
    read_agf, read_agfl, read_agi, read_finobt_root, read_inobt_root, read_superblock,
};
