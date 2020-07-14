// This is unexpected but rust wants me to specify my extern crates
// in main module.
#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate serde;
extern crate serde_json;

use std::fs;
use std::io;

use docopt::Docopt;
use serde::Deserialize;

mod trace_format;
use trace_format::Trace;

const USAGE: &'static str = "
Trace Tool.

Usage:
  trace_tool list [options]
  trace_tool filter <process>... [options]
  trace_tool (-h | --help)
  trace_tool (-v | --version)

Options:
  -h --help        Show this screen.
  -i --input=<input>  Input file [default: resources/sample_trace.json]
  -o --output=<output>  Input file [default: output.json]

  list             Lists the processes
  filter           Create new trace with only the given Renderer processes 
                   (non-renderer processes such as GPU, and BROWSER are still included)
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_process: Vec<String>,
    flag_help: bool,
    flag_input: String,
    flag_output: String,
    cmd_list: bool,
    cmd_filter: bool,
}

fn main() -> Result<(), io::Error> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    // give priority to help
    if args.flag_help {
        println!("{}", USAGE);
        return Ok(());
    } else if args.cmd_list {
        return list(&args.flag_input);
    } else if args.cmd_filter {
        return filter(args.arg_process, &args.flag_input, &args.flag_output);
    } else {
        println!("{}", USAGE);
        return Ok(());
    }
}

// Commands

fn filter(
    mut filtered_process_ids: Vec<String>,
    input_file: &str,
    output_file: &str,
) -> Result<(), io::Error> {
    let trace = read(input_file)?;

    for p in trace.processes() {
        if !p.is_renderer() {
            filtered_process_ids.push(p.id.to_string().clone());
        }
    }

    let filtered = trace.filter(filtered_process_ids);
    print(&filtered);
    write(&filtered, output_file)?;

    Ok(())
}

fn list(input_file: &str) -> Result<(), io::Error> {
    let trace = read(input_file)?; 
    print(&trace);

    Ok(())
}

// Utility functions

fn read(file: &str) -> Result<Trace, io::Error> {
    let content = fs::read_to_string(file)?;
    let trace: Trace = serde_json::from_str(&content)?;

    Ok(trace)
}

fn write(trace: &Trace, file: &str) -> Result<(), io::Error> {
    let content = serde_json::to_string(trace).expect("cannot serialize");
    fs::write(file, content)?;

    Ok(())
}

fn print_summary(trace: &Trace) {
    println!(
        "{} with {} processes and {:.2}s duration.",
        trace.info(),
        trace.processes().len(),
        trace.timings().duration.as_secs_f32()
    );

    println!("timing histogram: {:?}", trace.timing_buckets());
}

fn print(trace: &Trace) {
    print_summary(trace);
    for (i, p) in trace.processes().iter().enumerate() {
        println!("{:>2} ▶ {} ", i, p);
    }
}
