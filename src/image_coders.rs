pub struct CoolEncoder<'a, W: std::io::Write> {
    pub writer: &'a mut W,
}

pub struct CoolDecoder<R: std::io::Read> {
    pub reader: R,

    pub width: u32,
    pub height: u32,

    pub color_type: image::ColorType,
}

impl<'a, W: std::io::Write> CoolEncoder<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W: std::io::Write> image::ImageEncoder for CoolEncoder<'a, W> {
    fn write_image(
        mut self,
        buf: &[u8],
        width: u32,
        height: u32,
        color_type: image::ColorType,
    ) -> image::ImageResult<()> {
        assert_eq!(color_type, image::ColorType::Rgb8);
        for _ in buf.chunks_exact(3) {
            self.writer.write_all(&[0, 0, 0])?;
        }
        Ok(())
    }
}

impl<R: std::io::Read> CoolDecoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            width: 0,
            height: 0,
            color_type: image::ColorType::Rgb8,
        }
    }
}

impl<'a, R: std::io::Read> image::ImageDecoder<'a> for CoolDecoder<R> {
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn color_type(&self) -> image::ColorType {
        image::ColorType::Rgb8
    }

    fn read_image(self, buf: &mut [u8]) -> image::ImageResult<()> {
        Ok(())
    }

    type Reader = Box<dyn std::io::BufRead + 'a>;
    fn into_reader(self) -> image::ImageResult<Self::Reader> {
        unimplemented!()
    }
}
