use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

#[derive(Default, Clone, Copy)]
pub struct Options {
    pub rec601: bool,
    pub limited: bool,
    pub flip: bool,
}

pub struct Frame {
    pub w: usize,
    pub h: usize,
    data: Vec<u8>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Frame {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut f = File::open(path)?;
        let mut header = [0u8; 28];
        f.read_exact(&mut header)?;
        if &header[..12] != b"7th \x01\x01\x01\x00\x08\x00\x00\x00" {
            return Err(invalid(
                "Unsupported header. This build reads the supported 8-bit 4:2:2 layout.",
            ));
        }
        let word = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap()) as usize;
        let (w, h) = (word(12), word(16));
        if w == 0
            || w % 2 != 0
            || h == 0
            || w > 65535
            || h > 65535
            || word(20) != w + 65536
            || word(24) != h
        {
            return Err(invalid("Unsupported dimensions or packing fields."));
        }
        let len = w * h * 2;
        if len > 512 * 1024 * 1024 || f.metadata()?.len() != (28 + len) as u64 {
            return Err(invalid(
                "Unexpected payload length for packed 4:2:2, or frame exceeds 512 MiB.",
            ));
        }
        let mut data = vec![0; len];
        f.read_exact(&mut data)?;
        Ok(Self { w, h, data })
    }

    pub fn render(&self, max_edge: usize, opt: Options) -> (usize, usize, Vec<u8>) {
        let edge = self.w.max(self.h);
        let scale = max_edge.min(edge);
        let w = (self.w * scale / edge).max(1);
        let h = (self.h * scale / edge).max(1);
        let (kr, kb) = if opt.rec601 {
            (0.299, 0.114)
        } else {
            (0.2126, 0.0722)
        };
        let kg = 1.0 - kr - kb;
        let mut rgb = Vec::with_capacity(w * h * 3);
        for y in 0..h {
            let row = (2 * y + 1) * self.h / (2 * h);
            let row = if opt.flip { self.h - 1 - row } else { row };
            for x in 0..w {
                let col = (2 * x + 1) * self.w / (2 * w);
                let i = (row * self.w + (col & !1)) * 2;
                let mut luma = self.data[i + 2 * (col % 2)] as f32;
                let mut cb = self.data[i + 1] as f32 - 128.0;
                let mut cr = self.data[i + 3] as f32 - 128.0;
                if opt.limited {
                    luma = (luma - 16.0) * 255.0 / 219.0;
                    cb *= 255.0 / 224.0;
                    cr *= 255.0 / 224.0;
                }
                let r = luma + 2.0 * (1.0 - kr) * cr;
                let b = luma + 2.0 * (1.0 - kb) * cb;
                let g = (luma - kr * r - kb * b) / kg;
                rgb.extend([r, g, b].map(|v| v.round().clamp(0.0, 255.0) as u8));
            }
        }
        (w, h, rgb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_both_lumas_and_flips_rows() {
        let f = Frame {
            w: 2,
            h: 2,
            data: vec![0, 128, 255, 128, 31, 128, 87, 128],
        };
        assert_eq!(
            f.render(2, Options::default()).2,
            [0, 0, 0, 255, 255, 255, 31, 31, 31, 87, 87, 87]
        );
        assert_eq!(
            &f.render(
                2,
                Options {
                    flip: true,
                    ..Default::default()
                }
            )
            .2[..3],
            &[31; 3]
        );
    }
}
