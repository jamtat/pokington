//! Run the product command parser with an injected store for offline acceptance.
//!
//! Usage: `cargo run --example command_data -- ABSOLUTE_STORE choose -c 3`
use std::{error::Error, path::PathBuf, process::ExitCode};

use rand::{SeedableRng, rngs::StdRng};

use pokington::{
    command::{self, Runtime},
    store::Store,
};

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error>> {
    let mut args = std::env::args_os();
    let program = args.next().ok_or("missing executable name")?;
    let path = args
        .next()
        .ok_or("usage: command_data ABSOLUTE_STORE COMMAND [ARGS]")?;
    let args: Vec<_> = std::iter::once(program).chain(args).collect();
    Ok(ExitCode::from(
        command::run(
            args,
            || {
                Ok(Runtime::new(
                    Store::at(PathBuf::from(path))?,
                    StdRng::from_rng(&mut rand::rng()),
                ))
            },
            &mut std::io::stdout(),
            &mut std::io::stderr(),
        )
        .await,
    ))
}
