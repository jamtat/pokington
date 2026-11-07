use std::process::ExitCode;

use rand::{SeedableRng, rngs::StdRng};

use pokington::{
    command::{self, Runtime},
    store::Store,
};

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    ExitCode::from(
        command::run(
            args,
            || {
                Ok(Runtime::new(
                    Store::platform()?,
                    StdRng::from_rng(&mut rand::rng()),
                ))
            },
            &mut std::io::stdout(),
            &mut std::io::stderr(),
        )
        .await,
    )
}
