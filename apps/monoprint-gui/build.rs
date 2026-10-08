fn main() {
    // The feed is a release input, never a runtime override from the environment.
    println!("cargo:rerun-if-env-changed=MONOPRINT_UPDATE_URL");
    println!("cargo:rerun-if-env-changed=MONOPRINT_UPDATE_CHANNEL");
}
