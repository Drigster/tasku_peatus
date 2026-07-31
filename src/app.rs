use freya::{
    prelude::*,
    radio::{RadioStation, use_share_radio},
    router::{Routable, RouterConfig},
};
use freya_router::prelude::Router;

use crate::{
    hooks::{use_departures, use_location, use_stops},
    launch_config::{Data, DataChannel},
};
use crate::{layouts::AppLayout, pages::Timetable};

fn custom_theme() -> Theme {
    let mut theme = dark_theme();
    theme.name = "custom";
    theme.colors = ColorsSheet {
        primary: Color::from_rgb(227, 227, 227),
        secondary: Color::from_rgb(49, 161, 218),
        text_primary: Color::BLACK,
        text_secondary: Color::WHITE,
        ..DARK_COLORS
    };
    theme
}

pub struct MyApp {
    pub radio_station: RadioStation<Data, DataChannel>,
}
impl App for MyApp {
    fn render(&self) -> impl IntoElement {
        use_share_radio(move || self.radio_station);

        use_init_theme(|| custom_theme());

        use_stops();
        use_location();
        use_departures();

        Router::<Route>::new(|| RouterConfig::default().with_initial_path(Route::Timetable))
    }
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Timetable,
}
