use freya::{prelude::*, radio::use_radio, router::Outlet};

use crate::{DataChannel, app::Route};

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

        let radio = use_radio(DataChannel::LocationUpdate);

        let is_location_enabled = radio.slice(DataChannel::LocationEnabledUpdate, |s| {
            &s.is_location_enabled
        });
        let error_state = radio.slice(DataChannel::ErrorStateUpdate, |s| &s.error_state);

        NativeRouter::new().child(
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
                .child(
                    rect()
                        .position(Position::new_absolute().bottom(0.0))
                        .layer(Layer::Overlay)
                        .width(Size::Fill)
                        .height(Size::px(120.0))
                        .direction(Direction::Vertical)
                        .main_align(Alignment::End)
                        .maybe_child(if !*is_location_enabled.read() {
                            Some(
                                rect()
                                    .width(Size::Fill)
                                    .height(Size::px(60.0))
                                    .background(Color::from_hex("#FF000080").unwrap())
                                    .center()
                                    .child(label().text("Location disabled")),
                            )
                        } else {
                            None
                        })
                        .maybe_child(error_state.read().cloned().map(|error_state| {
                            rect()
                                .width(Size::Fill)
                                .height(Size::px(60.0))
                                .background(Color::from_hex("#FF000080").unwrap())
                                .center()
                                .child(label().text(format!("{:?}", error_state)))
                        })),
                ),
        )
    }
}
