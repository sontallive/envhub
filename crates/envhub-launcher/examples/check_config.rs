use envhub_core::load_state;

fn main() {
    match load_state() {
        Ok(state) => {
            if let Some(app) = state.apps.get("claudex") {
                println!("Found claudex: target_binary = '{}'", app.target_binary);
            } else {
                println!("Did not find claudex in loaded state.");
                println!("Available apps: {:?}", state.apps.keys());
            }
        }
        Err(e) => eprintln!("Failed to load state: {:?}", e),
    }
}
