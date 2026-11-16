use freya::{
    prelude::*,
    radio::use_radio,
    router::{Routable, Router, RouterConfig},
};

use crate::{
    hooks::{use_departures, use_location, use_stops, use_ticker},
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

pub fn app() -> impl IntoElement {
    use_init_theme(custom_theme);

    let radio = use_radio::<Data, DataChannel>(DataChannel::NoUpdate);
    use_stops(&radio);
    use_location(&radio);
    use_departures(&radio);
    use_ticker(&radio);

    Router::<Route>::new(|| RouterConfig::default().with_initial_path(Route::Timetable))
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Timetable,
}
