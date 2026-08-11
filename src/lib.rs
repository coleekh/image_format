use bytes::{Buf, BufMut};

mod image_coders;

const MAGIC: [u8; 8] = [0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00];
const IDENTIFIER_MASK: u8 = 0b0000_1111;
const RGBA: u8 = 0;
const RUN: u8 = 1;
const INDEX: u8 = 2;
const DIFFERENCE: u8 = 3;

pub struct Decoder<R: std::io::Read> {
    pub reader: R,
    pub width: u32,
    pub height: u32,
}

pub struct Encoder<W: std::io::Write> {
    pub writer: W,
    pub width: u32,
    pub height: u32,
}

impl<R: byteorder::ReadBytesExt> Decoder<R> {
    pub fn new(mut reader: R) -> std::io::Result<Self> {
        let mut magic = [0; 8];
        reader.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid magic number",
            ));
        }

        let width = reader.read_u32::<byteorder::BigEndian>()?;
        let height = reader.read_u32::<byteorder::BigEndian>()?;

        Ok(Self {
            reader,
            width,
            height,
        })
    }

    pub fn decode(&mut self) -> std::io::Result<image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        let mut output = image::ImageBuffer::new(self.width, self.height);
        // let mut data_iter = input.iter().copied().enumerate();
        let mut pixel_iter = output.pixels_mut();

        let mut last_pixel = image::Rgba([0, 0, 0, 0]);

        let mut print_count = 0;

        let mut pixel_buffer = [[0, 0, 0, 0]; 64];

        loop {
            let Some(pixel) = pixel_iter.next() else {
				break;
			};

            let identifier = self.reader.read_u8()?;
            match identifier & IDENTIFIER_MASK {
                RGBA => {
                    let (r, g, b, a) = (
                        self.reader.read_u8()?,
                        self.reader.read_u8()?,
                        self.reader.read_u8()?,
                        self.reader.read_u8()?,
                    );
                    *pixel = image::Rgba([r, g, b, a]);
                    last_pixel = *pixel;

                    let index = hash_pixel(&[r, g, b, a]);
                    pixel_buffer[index as usize] = [r, g, b, a];
                }

                RUN => {
                    let count = identifier >> 4;

                    // runs cover at least one pixel
                    *pixel = last_pixel;

                    for pixel in pixel_iter.by_ref().take(count as usize) {
                        *pixel = last_pixel;
                    }
                }

                INDEX => {
                    let index = identifier >> 4;
                    // *pixel = image::Rgba([255, 0, 0, 255]);
                    *pixel = image::Rgba(pixel_buffer[index as usize]);
                    last_pixel = *pixel;
                }

                DIFFERENCE => {
                    let diff = identifier >> 4;
                    let r = diff | 0b_1;
                    let g = diff | 0b_01;
                    let b = diff | 0b_001;
                    // let r = last_pixel[0].wrapping_sub(pixel_buffer[diff as usize][0]);
                    // let g = last_pixel[1].wrapping_sub(pixel_buffer[diff as usize][1]);
                    // let b = last_pixel[2].wrapping_sub(pixel_buffer[diff as usize][2]);
                    // let a = last_pixel[3].wrapping_sub(pixel_buffer[diff as usize][3]);
                    *pixel = image::Rgba([255, 0, 0, last_pixel[3]]);
                    // *pixel = image::Rgba([
                    //     last_pixel[0].wrapping_add_signed(r as i8 * 2 - 1),
                    //     last_pixel[1].wrapping_add_signed(g as i8 * 2 - 1),
                    //     last_pixel[2].wrapping_add_signed(b as i8 * 2 - 1),
                    //     last_pixel[3],
                    // ]);
                    last_pixel = *pixel;
                }

                _ => {
                    unimplemented!();
                }
            }
        }
        Ok(output)
    }
}

impl<W: byteorder::WriteBytesExt> Encoder<W> {
    pub fn new(writer: W, width: u32, height: u32) -> Self {
        Self {
            writer,
            width,
            height,
        }
    }

    pub fn encode(
        &mut self,
        input: &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    ) -> std::io::Result<()> {
        self.writer.write_all(&MAGIC)?;
        self.writer
            .write_u32::<byteorder::BigEndian>(input.width())?;
        self.writer
            .write_u32::<byteorder::BigEndian>(input.height())?;

        let mut pixels = input.pixels();

        // encoding is based off the first pixel
        let (image::Rgba(first)) = if let Some(first) = pixels.next() {
            first
        } else {
            // todo: if input is an empty image then return an error
            return Ok(());
        };
        self.writer.write_u8(RGBA)?;
        self.writer.write_all(first)?;

        let mut last_pixel = first;
        let mut run_length = 0;

        let mut print_count = 0;

        // run buffer holds only two values: RUN | !IDENTIFIER_MASK which represents a full run and one partial run
        let mut run_buffer: smallvec::SmallVec<[u8; 64]> =
            smallvec::smallvec![RUN | !IDENTIFIER_MASK; 64];

        let mut pixel_buffer = [[0, 0, 0, 0]; 64];
        // encode the rest of the pixels
        loop {
            let next = pixels.next();

            if next == Some(&image::Rgba(*last_pixel)) && run_length < usize::MAX {
                run_length += 1;
            } else {
                // if run ends then write the run
                if run_length > 0 {
                    // a run can only be up to 16 pixels iclusive
                    let count_of_full_runs = (run_length - 1) / 64;

                    if count_of_full_runs > run_buffer.len() {
                        run_buffer.extend(
                            std::iter::repeat(RUN | !IDENTIFIER_MASK).take(count_of_full_runs),
                        );
                    }
                    // offset the run length by 1;
                    // runs of 0 length should not exist
                    run_buffer[count_of_full_runs] = RUN | ((run_length - 1) % 16 << 4) as u8;
                    self.writer
                        .write_all(&run_buffer[0..count_of_full_runs + 1])?;
                    run_buffer[count_of_full_runs] = RUN | !IDENTIFIER_MASK;

                    run_length = 0;
                }
                if let Some((image::Rgba(pixel))) = next {
                    let index = hash_pixel(pixel);
                    if pixel_buffer[index as usize] == *pixel {
                        self.writer.write_u8(INDEX | (index << 4))?;
                    } else {
                        // let r = pixel[0].wrapping_sub(last_pixel[0]);
                        // let g = pixel[1].wrapping_sub(last_pixel[1]);
                        // let b = pixel[2].wrapping_sub(last_pixel[2]);
                        let r = (pixel[0] as i8).wrapping_sub(last_pixel[0] as i8);
                        let g = (pixel[1] as i8).wrapping_sub(last_pixel[1] as i8);
                        let b = (pixel[2] as i8).wrapping_sub(last_pixel[2] as i8);
                        if pixel[3] == last_pixel[3]
                            && (r == -1 || r == 1)
                            && (g == -1 || g == 1)
                            && (b == -1 || b == 1)
                        {
                            let diff =
                                (r == 1) as u8 | (((g == 1) as u8) << 1) | (((b == 1) as u8) << 2);
                            self.writer.write_u8(DIFFERENCE | (diff << 4))?;
                        } else {
                            // write the full pixel
                            self.writer.write_u8(RGBA)?;
                            self.writer.write_all(pixel.as_slice())?;
                        }

                        pixel_buffer[index as usize] = *pixel;
                    }

                    last_pixel = pixel;
                } else {
                    println!("ending encoding");
                    break;
                }
            }
        }

        Ok(())
    }
}

fn hash_pixel(pixel: &[u8; 4]) -> u8 {
    // convert to u16 to prevent overflow
    ((pixel[0] as u16 * 3 + pixel[1] as u16 * 5 + pixel[2] as u16 * 7 + pixel[3] as u16 * 11) % 16)
        as u8
}
