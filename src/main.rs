pub mod configure;

use clap::{Command, SubCommand};
use configure::configure;

fn main() {
    let matches = Command::new("s3-client-rs")
        .version("1.0")
        .author("chengxuguang. <417914077@qq.com>")
        .subcommand(SubCommand::with_name("configure").about("init s3 config"))
        .subcommand(SubCommand::with_name("cp").about("copy files"))
        .subcommand(SubCommand::with_name("sync").about("sync files"))
        .get_matches();

    if let Some(_) = matches.subcommand_matches("configure") {
        println!("configure");
        configure();
    } else if let Some(_) = matches.subcommand_matches("cp") {
        println!("cp");
    } else if let Some(_) = matches.subcommand_matches("sync") {
        println!("sync");
    }
}
