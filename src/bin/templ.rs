//! A convenience wrapper to quickly apply a variable file to a tera template.
use std::{
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::Parser;

use log::{debug, info};
use script_utils::{logging, prelude::*};
use tera::{Context as TeraContext, Tera};

#[derive(Parser, Debug)]
#[clap(
    name = "templ",
    about = "Apply variables to a template.",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,

    /// The path to the template.
    pub template: PathBuf,

    /// The path to the variable file (YAML for now).
    pub variables: PathBuf,

    /// Where the output should be written to.
    pub output: PathBuf,
}

fn main() -> Result<()> {
    // Parse commandline options.
    let args = CliArguments::parse();

    logging::init_logger(args.verbose);

    // Read template file
    if !args.template.exists() {
        eprintln!("Couldn't find template file at path {:?}", args.template);
    }
    let template = read_file(&args.template).context("Failed to read template file")?;

    // Generate tera context and render the template.
    let context = create_context(&args).context("Failed to create Tera context")?;
    let rendered = Tera::one_off(&template, &context, false).context("Failed to render file.")?;

    info!("Rendered template:\n##########\n{rendered}\n##########");

    // Write the templte to disk.
    let mut file = File::create(&args.output)
        .context(format!("Failed to create file at: {:?}", &args.output))?;
    file.write_all(rendered.as_bytes())
        .context("Failed to write output to file.")?;

    Ok(())
}

fn create_context(args: &CliArguments) -> Result<TeraContext> {
    // Open the file in read-only mode with buffer.
    let file = File::open(&args.variables).context(format!(
        "Failed to open template file at: {:?}",
        &args.variables
    ))?;
    let reader = BufReader::new(file);

    // Convert the yaml represention to a json representation, as the Tera Context can directly
    let variables: serde_json::Value = serde_yaml::from_reader(reader).context(format!(
        "Failed to read template file at: {:?}",
        &args.variables
    ))?;

    debug!("Variables: {:?}", &variables);

    // work with those.
    let context = TeraContext::from_value(variables).context("Failed to build tera context.")?;

    Ok(context)
}
