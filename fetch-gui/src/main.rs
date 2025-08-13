use crate::gui::run_fetch_application;

fn main() -> Result<(), ()> {
    match run_fetch_application() {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("Error running application: {}", e);
            Err(())
        }
    }
}

mod gui;