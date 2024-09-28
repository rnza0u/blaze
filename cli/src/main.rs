use clap::Parser;
use command::Command;
use context::CliContext;

mod command;
mod context;
mod subcommand;
mod subcommands;

fn main() {
    if let Err(err) = CliContext::try_new().and_then(|ctx| Command::try_parse()?.execute(ctx)) {
        if err.is::<clap::Error>() {
            let clap_error = err.downcast::<clap::Error>().unwrap();
            clap_error.print().unwrap();
            std::process::exit(2);
        }

        eprintln!();
        eprintln!("❌ Blaze command failed !");
        eprintln!();

        eprintln!("Error: {}", err);
        eprintln!();

        eprintln!("Trace:");

        for (i, nested_error) in err.chain().enumerate().skip(1) {
            eprintln!("\t{i}: {nested_error}");
        }

        std::process::exit(1);
    }
}
