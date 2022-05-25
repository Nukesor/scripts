//! Create a blurred image from the current screen.
//!
//! 1. Get a current screenshot via scrot.
//! 2. Run a custom point filter on the image data.
//! 3. Save it.
use std::{fs::File, io::BufWriter, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use clap::Parser;
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    io::Reader as ImageReader,
    DynamicImage, ImageBuffer, ImageEncoder, Pixel, Rgb, RgbImage,
};

use log::debug;
use rayon::{
    iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use script_utils::{bail, logging, prelude::*};

#[derive(Parser, Debug)]
#[clap(
    name = "blur",
    about = "Make a screenshot via scrot and blur it as a lockscreen",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// The scale we should blur to.
    /// I.e. `5` would result in a relative 20% downscale.
    pub scale: usize,

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
    get_screenshot()?;
    let mut image = load_image()?;

    // Blur the image.
    image = blur_image(args.scale, image)?;

    // Save the image as PNG to the filesystem.
    // Sadly, i3lock only supports PNG for now.
    let start = Instant::now();
    let buffered_file_write = &mut BufWriter::new(File::create(args.dest_path)?);
    let (width, height) = image.dimensions();
    PngEncoder::new_with_quality(
        buffered_file_write,
        CompressionType::Fast,
        FilterType::NoFilter,
    )
    .write_image(&image.into_raw(), width, height, image::ColorType::Rgb8)
    .context("Failed to save image to disk")?;
    debug!("Image write time: {}ms", start.elapsed().as_millis());

    Ok(())
}

/// Make a screenshot via scrot and capture the image (png) bytes.
fn get_screenshot() -> Result<()> {
    let start = Instant::now();
    Cmd::new("scrot --delay 0 --quality 95 --silent /tmp/blur-screenshot.jpg").run_success()?;
    debug!("scrot execution time: {}ms", start.elapsed().as_millis());

    Ok(())
}

/// Initialize the image from the raw bytes.
fn load_image() -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    let start = Instant::now();

    let image = ImageReader::open("/tmp/blur-screenshot.jpg")?.decode()?;
    let image = match image {
        DynamicImage::ImageRgb8(image) => image,
        _ => bail!("Expected Rgb8 format from scrot"),
    };

    debug!("Image init time: {}ms", start.elapsed().as_millis());
    Ok(image)
}

#[derive(Clone)]
struct ImageSpecs {
    width: usize,
    channel_count: usize,
    scale: usize,
}

/// Blur the image.
///
/// This is done by applying a point filter to (scale x scale) chunks.
fn blur_image(
    scale: usize,
    image: ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    let start = Instant::now();

    let (width, height) = image.dimensions();
    // Get the channel count (bytes per pixel).
    let channel_count = Rgb::<u8>::CHANNEL_COUNT as usize;
    // Convert the image into its raw bytes.
    let mut source_bytes = image.into_raw();

    // Define the chunks based on the image width, bytes per pixel and scaling factor.
    // Each chunk thereby has `scale` rows as data.
    let chunk_size = width as usize * channel_count * scale;

    let mut target_bytes: Vec<u8> = vec![0; source_bytes.len()];
    let target_chunks = target_bytes.par_chunks_mut(chunk_size);

    // We need additional info about the image dimensions and specs in the worker threads.
    // That's why we also zip a vector of these specs into the actual data iterator.
    let specs = ImageSpecs {
        width: width as usize,
        channel_count,
        scale,
    };
    let spec_vec = vec![specs; height as usize / scale + 1];

    source_bytes
        .par_chunks_mut(chunk_size)
        .zip(target_chunks)
        .zip(spec_vec.par_iter())
        .for_each(blur_row_chunk);

    debug!("Image conversion time: {}ms", start.elapsed().as_millis());

    RgbImage::from_raw(width, height, target_bytes)
        .context("Failed to create rgb image from target buffer")
}

/// Take a chunk of rows and pixelate them.
/// The pixelation process is dependand on a scale factor. For instance, a scale factor
/// of 3 will change 3x3 pixel chunks to the pixel of the center pixel.
///
/// This is done like this:
///
/// The following represents a 9x3 pixel matrix.
/// Each number represents a color.
/// ```
/// 1 2 3 4 5 6 7 8 9
/// 9 8 7 6 5 4 3 2 1
/// 7 7 7 8 8 8 9 9 9
/// ```
///
/// At first, we only look at the middle row.
/// ```
/// 9 8 7 6 5 4 3 2 1
/// ```
///
///
/// Step 1:
/// We then change the color each 3x pixel grid to that of the center pixel:
/// ```
/// 8 8 8 5 5 5 2 2 2
/// ```
///
/// Step 2:
/// The center row is then copied to the target buffer:
/// ```
/// 8 8 8 5 5 5 2 2 2
/// 8 8 8 5 5 5 2 2 2
/// 8 8 8 5 5 5 2 2 2
/// ```
fn blur_row_chunk(((source, target), specs): ((&mut [u8], &mut [u8]), &ImageSpecs)) {
    let channels = specs.channel_count;
    // Get the number of rows.
    let rows = source.len() / (specs.width * channels);
    let row_bytes = specs.width * channels;
    // Get the middle row (floored).
    let middle = rows / 2;

    // Calculate the start/end index of the middle row.
    let middle_row_start = middle * row_bytes;
    let middle_row_end = (middle + 1) * row_bytes;

    // Step 1:
    // Create an iterator through each pixel chunk of the middle row.
    let mut middle_pixel_iter = source
        .get_mut(middle_row_start..middle_row_end)
        .expect("Chunk size smaller than expected")
        .chunks_exact_mut(specs.scale * channels);

    // Calculate the indices for the middle pixel of each (full) pixel chunk.
    let middle_pixel_start = (specs.scale / 2) * channels;
    let middle_pixel_end = ((specs.scale / 2) + 1) * channels;
    while let Some(chunk) = middle_pixel_iter.next() {
        let middle_pixel = chunk
            .get_mut(middle_pixel_start..middle_pixel_end)
            .expect("Wrong middle pixel indices")
            .to_owned();

        // Replace all pixels in the row with the middle pixel.
        for pixel in chunk.chunks_mut(3) {
            pixel.clone_from_slice(&middle_pixel);
        }
    }

    // For the remainder of the row, we just take the first pixel instead of the middle.
    // The remainder appears if the width isn't devidable by our `scale` factor.
    let remainder = middle_pixel_iter.into_remainder();
    // Only copy stuff if there's more than pixel.
    if remainder.len() > channels {
        let first_pixel = remainder.get(0..channels).unwrap().to_owned();
        for pixel in remainder.chunks_mut(channels) {
            pixel.clone_from_slice(&first_pixel);
        }
    }

    // Step 2
    // Copy the final row into all source rows of our chunk.
    let source_middle_row = source.get(middle_row_start..middle_row_end).unwrap();
    for row in target.chunks_mut(row_bytes) {
        row.clone_from_slice(source_middle_row);
    }
}
