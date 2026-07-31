mod app;
mod components;
mod launch_config;
mod layouts;
mod pages;
mod utils;

use freya::prelude::launch;

use crate::launch_config::build_launch_config;

#[allow(dead_code)]
fn main() {
    env_logger::init();

    launch(build_launch_config());
}
