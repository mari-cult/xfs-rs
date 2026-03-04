use crate::error::DeviceError;

pub trait BlockDevice {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError>;
}

#[cfg(feature = "std")]
pub struct StdFileDevice {
    file: std::fs::File,
}

#[cfg(feature = "std")]
impl StdFileDevice {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, DeviceError> {
        let file = std::fs::File::open(path).map_err(|_| DeviceError::Io)?;
        Ok(Self { file })
    }
}

#[cfg(feature = "std")]
impl BlockDevice for StdFileDevice {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), DeviceError> {
        use std::io::Read;
        use std::io::Seek;
        use std::io::SeekFrom;

        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| DeviceError::Io)?;
        let mut read_total = 0usize;
        while read_total < buf.len() {
            let n = self
                .file
                .read(&mut buf[read_total..])
                .map_err(|_| DeviceError::Io)?;
            if n == 0 {
                return Err(DeviceError::ShortRead {
                    expected: buf.len(),
                    actual: read_total,
                });
            }
            read_total += n;
        }
        Ok(())
    }
}
