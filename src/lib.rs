#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

mod app;
mod components;
mod hooks;
mod launch_config;
mod layouts;
mod pages;
mod utils;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(droid_app: AndroidApp) {
    use crate::launch_config::build_launch_config;
    use freya::android::AndroidPlugin;
    use freya::prelude::launch;
    use freya_winit::renderer::NativeEvent;
    use winit::{event_loop::EventLoop, platform::android::EventLoopBuilderExtAndroid};

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Debug),
    );

    let event_loop = EventLoop::<NativeEvent>::with_user_event()
        .with_android_app(droid_app.clone())
        .build()
        .expect("Failed to build event loop");

    let mut config = build_launch_config();

    config = config
        .with_event_loop(event_loop)
        .with_plugin(AndroidPlugin::new(droid_app));

    launch(config);
}
