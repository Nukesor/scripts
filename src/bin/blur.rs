use std::{io::Cursor, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use clap::Parser;
use image::{io::Reader as ImageReader, DynamicImage, ImageBuffer, ImageFormat, Rgb};

use log::debug;
use script_utils::{bail, logging, prelude::*};

#[derive(Parser, Debug)]
#[clap(
    name = "blur",
    about = "Make a screenshot via scrot and blur it as a lockscreen",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Where the screenshot file should be put.
    pub dest_path: PathBuf,

    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();
    logging::init_logger(args.verbose);

    // Make screenshot and init the image.
    let image_bytes = get_screenshot()?;
    let mut image = load_image(image_bytes)?;

    // Blur the image.
    image = blur_image(image);

    // Save the image to the filesystem.
    let start = Instant::now();
    image.save("/tmp/screenshot.jpg")?;
    debug!("Image write time: {}ms", start.elapsed().as_millis());

    Ok(())
}

/// Make a screenshot via scrot and capture the image (png) bytes.
fn get_screenshot() -> Result<Vec<u8>> {
    let start = Instant::now();
    let capture = Cmd::new("scrot --delay 0 --silent -").run_success()?;
    debug!("scrot execution time: {}ms", start.elapsed().as_millis());

    Ok(capture.stdout)
}

/// Initialize the image from the raw bytes.
fn load_image(image_bytes: Vec<u8>) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    let start = Instant::now();

    let image = ImageReader::with_format(Cursor::new(image_bytes), ImageFormat::Png).decode()?;
    let image = match image {
        DynamicImage::ImageRgb8(image) => image,
        _ => bail!("Expected Rgb8 format from scrot"),
    };

    debug!("Image init time: {}ms", start.elapsed().as_millis());
    Ok(image)
}

fn blur_image(image: ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let start = Instant::now();
    debug!("Image conversion time: {}ms", start.elapsed().as_millis());
    image
}
