use freya::{
    animation::{AnimNum, OnCreation, OnFinish, use_animation},
    icons::lucide,
    prelude::*,
};

pub fn loader() -> impl IntoElement {
    let rotation = use_animation(|conf| {
        conf.on_creation(OnCreation::Run);
        conf.on_finish(OnFinish::restart());
        AnimNum::new(0.0, 360.0).time(600)
    });

    SvgViewer::new(lucide::loader_circle())
        .rotation(*&rotation.read().value())
        .width(Size::px(48.0))
        .height(Size::px(48.0))
        .color(Color::WHITE)
}
