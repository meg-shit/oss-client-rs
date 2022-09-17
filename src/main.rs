#![allow(unused_must_use)]
#![allow(unused)]
pub mod config;
pub mod configure;
pub mod s3;
pub mod utils;

use clap::{Arg, Command, SubCommand};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("s3-client-rs")
        .version("1.0")
        .author("chengxuguang. <417914077@qq.com>")
        .subcommand(SubCommand::with_name("configure").about("init s3 config"))
        .subcommand(
            SubCommand::with_name("cp")
                .about("copy files")
                .arg(Arg::new("src").required(true))
                .arg(Arg::new("target").required(true)),
        )
        .subcommand(SubCommand::with_name("sync").about("sync files"))
        .get_matches();

    match matches.subcommand_name() {
        Some("configure") => {
            config::parser(true)?;
        }
        Some("cp") => {
            let src = matches
                .subcommand()
                .unwrap()
                .1
                .get_one::<String>("src")
                .unwrap();

            let target = matches
                .subcommand()
                .unwrap()
                .1
                .get_one::<String>("target")
                .unwrap();
            s3::upload_file(src, target).await?;
        }
        Some("sync") => {}
        _ => {}
    }
    Ok(())
}
