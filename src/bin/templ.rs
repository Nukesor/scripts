//! Mini tool to quickly template any file.
//!
//! Currently uses Tera for templating, but might switch to `upon` soon.
//! Takes a single template file and multiple files that specify variables.
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use chrono::{Datelike, TimeDelta};
use clap::{ArgAction, Parser};
use log::{debug, info};
use script_utils::{logging, prelude::*};
use serde_yaml::Value;
use tera::{Context as TeraContext, Tera};

#[derive(Parser, Debug)]
#[clap(
    name = "templ",
    about = "Apply variables to a template.",
    author = "Arne Beer <contact@arne.beer>"
)]
struct CliArguments {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// The path to the template.
    pub template: PathBuf,

    /// Paths to the variable files (only YAML for now).
    /// Variables in Files that're passed in later will have precedence.
    pub variables: Vec<PathBuf>,

    /// Where the output file will be written to.
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

    // Write the template to disk.
    let mut file = File::create(&args.output)
        .context(format!("Failed to create file at: {:?}", &args.output))?;
    file.write_all(rendered.as_bytes())
        .context("Failed to write output to file.")?;

    Ok(())
}

fn create_context(args: &CliArguments) -> Result<TeraContext> {
    let mut context = get_default_context()?;

    for file in args.variables.iter() {
        // Read the yaml file into a HashMap of [Value], which can
        // be easily consumed by the Tera context.
        let file = File::open(file).context(format!(
            "Failed to open template file at: {:?}",
            &args.variables
        ))?;
        let reader = BufReader::new(&file);
        let variables: HashMap<String, Value> = serde_yaml::from_reader(reader)
            .context(format!("Failed to read template file at: {file:?}"))?;

        // Merge all variables together
        variables.into_iter().for_each(|(key, value)| {
            context.insert(key, value);
        });
    }

    let context = TeraContext::from_serialize(context).context("Failed to build tera context.")?;

    debug!("Variables: {:#?}", &context);

    Ok(context)
}

/// Build a default context for various circumstances
fn get_default_context() -> Result<HashMap<String, Value>> {
    let mut context: HashMap<String, Value> = HashMap::new();
    let today = chrono::Local::now();
    let start_of_month = today
        - TimeDelta::try_days(today.day0().into())
            .context("Failed to create start of month time delta")?;
    let day_in_last_month = start_of_month - TimeDelta::try_days(10).unwrap();

    // Add a few German values related to the current date.
    let mut de: HashMap<String, Value> = HashMap::new();
    let german_months = [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ];
    de.insert(
        "current_month".into(),
        serde_yaml::to_value(german_months[start_of_month.month0() as usize]).unwrap(),
    );
    de.insert(
        "year_of_current_month".into(),
        serde_yaml::to_value(start_of_month.year()).unwrap(),
    );
    de.insert(
        "last_month".into(),
        serde_yaml::to_value(german_months[day_in_last_month.month0() as usize]).unwrap(),
    );
    de.insert(
        "year_of_last_month".into(),
        serde_yaml::to_value(day_in_last_month.year()).unwrap(),
    );
    context.insert(
        "de".into(),
        serde_yaml::to_value(de).context("Couldn't serialize default values")?,
    );

    Ok(context)
}
