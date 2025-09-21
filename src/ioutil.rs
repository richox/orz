use std::io::{Read, Write};

pub struct CountRead<R: Read>(R, usize);
pub struct CountWrite<W: Write>(W, usize);

impl<R: Read> CountRead<R> {
    pub fn new(r: R) -> Self {
        CountRead(r, 0)
    }

    pub fn count(&self) -> usize {
        self.1
    }
}

impl<W: Write> CountWrite<W> {
    pub fn new(w: W) -> Self {
        CountWrite(w, 0)
    }

    pub fn count(&self) -> usize {
        self.1
    }
}

impl<R: Read> Read for CountRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = self.0.read(buf)?;
        self.1 += len;
        Ok(len)
    }
}

impl<W: Write> Write for CountWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = self.0.write(buf)?;
        self.1 += len;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

pub trait ReadExt {
    fn read_len(&mut self) -> std::io::Result<usize>;
}

pub trait WriteExt {
    fn write_len(&mut self, len: usize) -> std::io::Result<()>;
}

impl<R: Read> ReadExt for R {
    fn read_len(&mut self) -> std::io::Result<usize> {
        let mut buf = [0u8];
        let mut len = 0usize;
        let mut factor = 1;
        loop {
            self.read_exact(&mut buf)?;
            let v = buf[0];
            if v < 128 {
                len += (v as usize) * factor;
                break;
            }
            len += (v - 128) as usize * factor;
            factor *= 128;
        }
        Ok(len)
    }
}

impl<W: Write> WriteExt for W {
    fn write_len(&mut self, mut len: usize) -> std::io::Result<()> {
        while len >= 128 {
            let v = len % 128;
            len /= 128;
            self.write_all(&[128 + v as u8])?;
        }
        self.write_all(&[len as u8])?;
        Ok(())
    }
}
