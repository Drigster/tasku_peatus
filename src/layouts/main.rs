use freya::{prelude::*, router::Outlet};

use crate::app::Route;

#[derive(PartialEq)]
pub struct AppLayout;
impl Component for AppLayout {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();

        // System bar insets don't change per frame, so resolve them once. On
        // Android this is a full JNI round trip (attach thread, load class by
        // name, static call) and must not run on every render.
        let offsets: (f32, f32, f32, f32) = use_hook(|| {
            #[cfg(target_os = "android")]
            {
                match crate::utils::jni_utils::get_bar_sizes() {
                    Ok(offsets) => offsets,
                    Err(e) => {
                        log::error!("Error getting bar sizes: {e:?}");
                        (0.0, 0.0, 0.0, 0.0)
                    }
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                (0.0, 0.0, 0.0, 0.0)
            }
        });

        let scale_factor: State<f32> = use_state(|| 2.625);
        let scale = *scale_factor.read();

        let (background_color, title_color) = {
            let theme = theme.read();
            (theme.colors.secondary, theme.colors.text_secondary)
        };

        rect()
            .padding((
                offsets.0 / scale,
                offsets.1 / scale,
                offsets.2 / scale,
                offsets.3 / scale,
            ))
            .expanded()
            .background(background_color)
            .child(
                rect()
                    .width(Size::Fill)
                    .height(Size::px(50.0))
                    .center()
                    .shadow(
                        Shadow::new()
                            .y(4.0)
                            .blur(4.0)
                            .color(Color::BLACK.with_a(64)),
                    )
                    .child(
                        label()
                            .font_size(20.0)
                            .font_weight(FontWeight::MEDIUM)
                            .color(title_color)
                            .text("Timetable"),
                    ),
            )
            .child(Outlet::<Route>::new())
    }
}
