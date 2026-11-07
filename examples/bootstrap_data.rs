//! Focused S02 bootstrap/installed-data acceptance harness; the product CLI is S03.
use std::{error::Error, path::PathBuf};

use pokington::{
    bootstrap::Bootstrap,
    store::{Store, validate_png},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(1..=2).contains(&args.len())
        || args
            .get(1)
            .is_some_and(|arg| !["--force", "--offline"].contains(&arg.as_str()))
    {
        return Err("usage: bootstrap_data ABSOLUTE_STORE [--force|--offline]".into());
    }
    let store_path = args.first().ok_or("missing store path")?;
    let store = Store::at(PathBuf::from(store_path))?;
    if args.get(1).is_some_and(|arg| arg == "--offline") {
        let catalogue = store.load_active()?.ok_or("no installed catalogue")?;
        for form in catalogue.forms() {
            validate_png(&store.read_artwork(&form.local_artwork)?)?;
        }
        eprintln!(
            "Offline data verified: {} species, {} forms",
            catalogue.species.len(),
            catalogue.forms().count()
        );
    } else {
        Bootstrap::new()?
            .run(&store, args.len() == 2, &mut std::io::stderr().lock())
            .await?;
    }
    Ok(())
}
