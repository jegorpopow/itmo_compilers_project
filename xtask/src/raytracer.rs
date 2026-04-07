use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter},
    path::Path,
};

use anyhow::{Context, Error};
use culpa::throws;
use png::Encoder;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Pixel {
    red: u8,
    green: u8,
    blue: u8,
}

#[throws]
fn next(lines: &mut impl Iterator<Item = io::Result<String>>, path: &Path) -> String {
    lines
        .next()
        .context("Unexpected EOF")
        .and_then(|line| line.context("IO error"))
        .with_context(|| format!("Error reading from {}", path.display()))?
}

#[throws]
pub(crate) fn render(input: &Path, output: &Path) {
    let mut lines = BufReader::new(
        File::open(input)
            .with_context(|| format!("Error opening input file ({})", input.display()))?,
    )
    .lines();
    let height = next(&mut lines, input)?.parse().context("Invalid height")?;
    let width = next(&mut lines, input)?.parse().context("Invalid width")?;
    let mut result = Vec::with_capacity((width * height) as usize * 3);
    for x in 0..width {
        for y in 0..height {
            let Pixel { red, green, blue } = json5::from_str(&next(&mut lines, input)?)
                .with_context(|| format!("Error parsing data for pixel at x = {x}, y = {y}"))?;
            result.push(red);
            result.push(green);
            result.push(blue);
        }
    }
    let mut out =
        Encoder::new(
            BufWriter::new(File::create(output).with_context(|| {
                format!("Could not create output image at {}", output.display())
            })?),
            width,
            height,
        );
    out.set_color(png::ColorType::Rgb);
    out.write_header()
        .and_then(|mut out| out.write_image_data(&result))
        .with_context(|| format!("Error encoding image to {}", output.display()))?
}
