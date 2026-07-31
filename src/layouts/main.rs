use freya::{prelude::*, radio::use_radio, router::Outlet};

use crate::{app::Route, launch_config::DataChannel};

#[derive(PartialEq)]
pub struct AppLayout;
impl Component for AppLayout {
    fn render(&self) -> impl IntoElement {
        let theme = use_theme();

        let offsets: (f32, f32, f32, f32) = {
            #[cfg(target_os = "android")]
            {
                match crate::utils::jni_utils::get_bar_sizes() {
                    Ok(offsets) => {
                        println!("[Print] Offsets {:?}", offsets);
                        offsets
                    }
                    Err(e) => {
                        println!("[Print] Error getting bar sizes: {:?}", e);
                        (0.0, 0.0, 0.0, 0.0)
                    }
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                (0.0, 0.0, 0.0, 0.0)
            }
        };

        println!("[Print] Offsets {:?}", offsets);

        let scale_factor: State<f32> = use_state(|| 2.625);

        rect()
            .padding((
                offsets.0 / *scale_factor.read(),
                offsets.1 / *scale_factor.read(),
                offsets.2 / *scale_factor.read(),
                offsets.3 / *scale_factor.read(),
            ))
            .expanded()
            .background(theme.read().colors.secondary)
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
                            .color(theme.read().colors.text_secondary)
                            .text("Timetable"),
                    ),
            )
            .child(Outlet::<Route>::new())
    }
}
