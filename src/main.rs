use anyhow::Result;
use clap::{command, Arg, Command};
use std::env;
use std::process::exit;
use std::sync::Arc;
use thinp::report::*;

use dm_archive::create;
use dm_archive::dump_stream;
use dm_archive::list;
use dm_archive::pack;
use dm_archive::unpack;

//-----------------------

fn mk_report() -> Arc<Report> {
    if atty::is(atty::Stream::Stdout) {
        Arc::new(mk_progress_bar_report())
    } else {
        Arc::new(mk_simple_report())
    }
}

fn main_() -> Result<()> {
    let default_archive = match env::var("DM_ARCHIVE_DIR") {
        Err(_) => String::new(),
        Ok(s) => s,
    };

    let archive_arg = if default_archive.is_empty() {
        Arg::new("ARCHIVE")
            .help("Specify archive directory")
            .required(true)
            .long("archive")
            .short('a')
            .value_name("ARCHIVE")
            .takes_value(true)
    } else {
        Arg::new("ARCHIVE")
            .help("Specify archive directory")
            .default_value(&default_archive)
            .long("archive")
            .short('a')
            .value_name("ARCHIVE")
            .takes_value(true)
    };

    let stream_arg = Arg::new("STREAM")
        .help("Specify an archived stream to unpack")
        .required(true)
        .long("stream")
        .short('s')
        .value_name("STREAM")
        .takes_value(true);

    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("creates a new archive")
                .arg(archive_arg.clone())
                .arg(
                    Arg::new("BLOCK_SIZE")
                        .help("Specify the average block size used when deduplicating data")
                        .required(false)
                        .long("block-size")
                        .value_name("BLOCK_SIZE")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("HASH_CACHE_SIZE_MEG")
                        .help("Specify how much memory is used for caching hash entries")
                        .required(false)
                        .long("hash-cache-size-meg")
                        .value_name("HASH_CACHE_SIZE_MEG")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("DATA_CACHE_SIZE_MEG")
                        .help("Specify how much memory is used for caching data")
                        .required(false)
                        .long("data-cache-size-meg")
                        .value_name("DATA_CACHE_SIZE_MEG")
                        .takes_value(true),
                ),
        )
        .subcommand(
            Command::new("pack")
                .about("packs a stream into the archive")
                .arg(
                    Arg::new("INPUT")
                        .help("Specify a device or file to archive")
                        .required(true)
                        .value_name("INPUT")
                        .takes_value(true),
                )
                .arg(archive_arg.clone())
                .arg(
                    Arg::new("DELTA_STREAM")
                        .help(
                            "Specify the stream that contains an older version of this thin device",
                        )
                        .required(false)
                        .long("delta-stream")
                        .value_name("DELTA_STREAM")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("DELTA_DEVICE")
                        .help(
                            "Specify the device that contains an older version of this thin device",
                        )
                        .required(false)
                        .long("delta-device")
                        .value_name("DELTA_DEVICE")
                        .takes_value(true),
                ),
        )
        .subcommand(
            Command::new("unpack")
                .about("unpacks a stream from the archive")
                .arg(
                    Arg::new("OUTPUT")
                        .help("Specify a device or file as the destination")
                        .required(true)
                        .value_name("OUTPUT")
                        .takes_value(true),
                )
                .arg(archive_arg.clone())
                .arg(stream_arg.clone()),
        )
        .subcommand(
            Command::new("verify")
                .about("verifies stream in the archive against the original file/dev")
                .arg(
                    Arg::new("INPUT")
                        .help("Specify a device or file containing the correct version of the data")
                        .required(true)
                        .value_name("INPUT")
                        .takes_value(true),
                )
                .arg(archive_arg.clone())
                .arg(stream_arg.clone()),
        )
        .subcommand(
            Command::new("dump-stream")
                .about("dumps stream instructions (development tool)")
                .arg(archive_arg.clone())
                .arg(stream_arg.clone()),
        )
        .subcommand(
            Command::new("list")
                .about("lists the streams in the archive")
                .arg(archive_arg.clone()),
        )
        .get_matches();

    let report = mk_report();
    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            create::run(sub_matches, report)?;
        }
        Some(("pack", sub_matches)) => {
            pack::run(sub_matches, report)?;
        }
        Some(("unpack", sub_matches)) => {
            unpack::run_unpack(sub_matches, report)?;
        }
        Some(("verify", sub_matches)) => {
            unpack::run_verify(sub_matches, report)?;
        }
        Some(("list", sub_matches)) => {
            list::run(sub_matches, report)?;
        }
        Some(("dump-stream", sub_matches)) => {
            dump_stream::run(sub_matches, report)?;
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents 'None'"),
    }

    Ok(())
}

fn main() {
    let code = match main_() {
        Ok(()) => 0,
        Err(e) => {
            // FIXME: write to report
            eprintln!("{:?}", e);
            // We don't print out the error since -q may be set
            1
        }
    };

    exit(code)
}

//-----------------------
