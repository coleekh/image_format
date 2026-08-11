fn main() {
    let image = image::open("C:/Dev/rust/image_format/res/input.png")
        .unwrap()
        .to_rgba8();

    // encode
    let mut encoded = Vec::with_capacity(image.width() as usize * image.height() as usize * 4);
    let mut encoder = image_format::Encoder::new(&mut encoded, image.width(), image.height());
    encoder.encode(&image).unwrap();

    println!(
        "Encoded {} bytes; Raw size: {}",
        encoded.len(),
        image.width() * image.height() * 4
    );
    // decode
    let mut decoder = image_format::Decoder::new(&encoded[..]).unwrap();
    let output = decoder.decode().unwrap();

    if let Err(e) = output.save("C:/Dev/rust/image_format/res/output.png") {
        eprintln!("Error saving image: {e}");
    }

    assert_eq!(output, image, "decoded image does not match original image");
}
