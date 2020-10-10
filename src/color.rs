use easygpu::color::{Rgba, Rgba8};
use palette::{rgb::Srgba, Component, Shade, Srgb};

#[derive(Default, Clone, Debug, Copy, PartialEq)]
pub struct Color(Rgba);

impl<U: Component> From<Srgba<U>> for Color {
    fn from(color: Srgba<U>) -> Self {
        let color = color.into_format::<_, f32>();
        Self(Rgba {
            r: color.color.red,
            g: color.color.green,
            b: color.color.blue,
            a: color.alpha,
        })
    }
}

impl<U: Component> From<Srgb<U>> for Color {
    fn from(color: Srgb<U>) -> Self {
        let color = color.into_format::<f32>();
        Self(Rgba {
            r: color.red,
            g: color.green,
            b: color.blue,
            a: 1.,
        })
    }
}

impl Into<Srgba> for Color {
    fn into(self) -> Srgba {
        Srgba::new(self.0.r, self.0.g, self.0.b, self.0.a)
    }
}

impl From<Rgba> for Color {
    fn from(color: Rgba) -> Self {
        Self(color)
    }
}

impl Into<Rgba> for Color {
    fn into(self) -> Rgba {
        self.0
    }
}

impl Into<Rgba8> for Color {
    fn into(self) -> Rgba8 {
        self.0.into()
    }
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self(Rgba { r, g, b, a })
    }

    pub fn lighten(self, amount: f32) -> Color {
        let color: Srgba = self.into();
        let linear = color.into_linear();
        Srgba::from_linear(linear.lighten(amount)).into()
    }

    pub fn darken(self, amount: f32) -> Color {
        let color: Srgba = self.into();
        let linear = color.into_linear();
        Srgba::from_linear(linear.darken(amount)).into()
    }

    pub fn red(&self) -> f32 {
        self.0.r
    }

    pub fn green(&self) -> f32 {
        self.0.g
    }

    pub fn blue(&self) -> f32 {
        self.0.b
    }

    pub fn alpha(&self) -> f32 {
        self.0.a
    }

    pub fn rgba(&self) -> [f32; 4] {
        [self.0.r, self.0.g, self.0.b, self.0.a]
    }
}

impl Color {
    pub const ALICEBLUE: Color = Color::new(240., 248., 255., 1.);
    pub const ANTIQUEWHITE: Color = Color::new(250., 235., 215., 1.);
    pub const AQUA: Color = Color::new(0., 255., 255., 1.);
    pub const AQUAMARINE: Color = Color::new(127., 255., 212., 1.);
    pub const AZURE: Color = Color::new(240., 255., 255., 1.);
    pub const BEIGE: Color = Color::new(245., 245., 220., 1.);
    pub const BISQUE: Color = Color::new(255., 228., 196., 1.);
    pub const BLACK: Color = Color::new(0., 0., 0., 1.);
    pub const BLANCHEDALMOND: Color = Color::new(255., 235., 205., 1.);
    pub const BLUE: Color = Color::new(0., 0., 255., 1.);
    pub const BLUEVIOLET: Color = Color::new(138., 43., 226., 1.);
    pub const BROWN: Color = Color::new(165., 42., 42., 1.);
    pub const BURLYWOOD: Color = Color::new(222., 184., 135., 1.);
    pub const CADETBLUE: Color = Color::new(95., 158., 160., 1.);
    pub const CHARTREUSE: Color = Color::new(127., 255., 0., 1.);
    pub const CHOCOLATE: Color = Color::new(210., 105., 30., 1.);
    pub const CORAL: Color = Color::new(255., 127., 80., 1.);
    pub const CORNFLOWERBLUE: Color = Color::new(100., 149., 237., 1.);
    pub const CORNSILK: Color = Color::new(255., 248., 220., 1.);
    pub const CRIMSON: Color = Color::new(220., 20., 60., 1.);
    pub const CYAN: Color = Color::new(0., 255., 255., 1.);
    pub const DARKBLUE: Color = Color::new(0., 0., 139., 1.);
    pub const DARKCYAN: Color = Color::new(0., 139., 139., 1.);
    pub const DARKGOLDENROD: Color = Color::new(184., 134., 11., 1.);
    pub const DARKGRAY: Color = Color::new(169., 169., 169., 1.);
    pub const DARKGREEN: Color = Color::new(0., 100., 0., 1.);
    pub const DARKGREY: Color = Color::new(169., 169., 169., 1.);
    pub const DARKKHAKI: Color = Color::new(189., 183., 107., 1.);
    pub const DARKMAGENTA: Color = Color::new(139., 0., 139., 1.);
    pub const DARKOLIVEGREEN: Color = Color::new(85., 107., 47., 1.);
    pub const DARKORANGE: Color = Color::new(255., 140., 0., 1.);
    pub const DARKORCHID: Color = Color::new(153., 50., 204., 1.);
    pub const DARKRED: Color = Color::new(139., 0., 0., 1.);
    pub const DARKSALMON: Color = Color::new(233., 150., 122., 1.);
    pub const DARKSEAGREEN: Color = Color::new(143., 188., 143., 1.);
    pub const DARKSLATEBLUE: Color = Color::new(72., 61., 139., 1.);
    pub const DARKSLATEGRAY: Color = Color::new(47., 79., 79., 1.);
    pub const DARKSLATEGREY: Color = Color::new(47., 79., 79., 1.);
    pub const DARKTURQUOISE: Color = Color::new(0., 206., 209., 1.);
    pub const DARKVIOLET: Color = Color::new(148., 0., 211., 1.);
    pub const DEEPPINK: Color = Color::new(255., 20., 147., 1.);
    pub const DEEPSKYBLUE: Color = Color::new(0., 191., 255., 1.);
    pub const DIMGRAY: Color = Color::new(105., 105., 105., 1.);
    pub const DIMGREY: Color = Color::new(105., 105., 105., 1.);
    pub const DODGERBLUE: Color = Color::new(30., 144., 255., 1.);
    pub const FIREBRICK: Color = Color::new(178., 34., 34., 1.);
    pub const FLORALWHITE: Color = Color::new(255., 250., 240., 1.);
    pub const FORESTGREEN: Color = Color::new(34., 139., 34., 1.);
    pub const FUCHSIA: Color = Color::new(255., 0., 255., 1.);
    pub const GAINSBORO: Color = Color::new(220., 220., 220., 1.);
    pub const GHOSTWHITE: Color = Color::new(248., 248., 255., 1.);
    pub const GOLD: Color = Color::new(255., 215., 0., 1.);
    pub const GOLDENROD: Color = Color::new(218., 165., 32., 1.);
    pub const GRAY: Color = Color::new(128., 128., 128., 1.);
    pub const GREY: Color = Color::new(128., 128., 128., 1.);
    pub const GREEN: Color = Color::new(0., 128., 0., 1.);
    pub const GREENYELLOW: Color = Color::new(173., 255., 47., 1.);
    pub const HONEYDEW: Color = Color::new(240., 255., 240., 1.);
    pub const HOTPINK: Color = Color::new(255., 105., 180., 1.);
    pub const INDIANRED: Color = Color::new(205., 92., 92., 1.);
    pub const INDIGO: Color = Color::new(75., 0., 130., 1.);
    pub const IVORY: Color = Color::new(255., 255., 240., 1.);
    pub const KHAKI: Color = Color::new(240., 230., 140., 1.);
    pub const LAVENDER: Color = Color::new(230., 230., 250., 1.);
    pub const LAVENDERBLUSH: Color = Color::new(255., 240., 245., 1.);
    pub const LAWNGREEN: Color = Color::new(124., 252., 0., 1.);
    pub const LEMONCHIFFON: Color = Color::new(255., 250., 205., 1.);
    pub const LIGHTBLUE: Color = Color::new(173., 216., 230., 1.);
    pub const LIGHTCORAL: Color = Color::new(240., 128., 128., 1.);
    pub const LIGHTCYAN: Color = Color::new(224., 255., 255., 1.);
    pub const LIGHTGOLDENRODYELLOW: Color = Color::new(250., 250., 210., 1.);
    pub const LIGHTGRAY: Color = Color::new(211., 211., 211., 1.);
    pub const LIGHTGREEN: Color = Color::new(144., 238., 144., 1.);
    pub const LIGHTGREY: Color = Color::new(211., 211., 211., 1.);
    pub const LIGHTPINK: Color = Color::new(255., 182., 193., 1.);
    pub const LIGHTSALMON: Color = Color::new(255., 160., 122., 1.);
    pub const LIGHTSEAGREEN: Color = Color::new(32., 178., 170., 1.);
    pub const LIGHTSKYBLUE: Color = Color::new(135., 206., 250., 1.);
    pub const LIGHTSLATEGRAY: Color = Color::new(119., 136., 153., 1.);
    pub const LIGHTSLATEGREY: Color = Color::new(119., 136., 153., 1.);
    pub const LIGHTSTEELBLUE: Color = Color::new(176., 196., 222., 1.);
    pub const LIGHTYELLOW: Color = Color::new(255., 255., 224., 1.);
    pub const LIME: Color = Color::new(0., 255., 0., 1.);
    pub const LIMEGREEN: Color = Color::new(50., 205., 50., 1.);
    pub const LINEN: Color = Color::new(250., 240., 230., 1.);
    pub const MAGENTA: Color = Color::new(255., 0., 255., 1.);
    pub const MAROON: Color = Color::new(128., 0., 0., 1.);
    pub const MEDIUMAQUAMARINE: Color = Color::new(102., 205., 170., 1.);
    pub const MEDIUMBLUE: Color = Color::new(0., 0., 205., 1.);
    pub const MEDIUMORCHID: Color = Color::new(186., 85., 211., 1.);
    pub const MEDIUMPURPLE: Color = Color::new(147., 112., 219., 1.);
    pub const MEDIUMSEAGREEN: Color = Color::new(60., 179., 113., 1.);
    pub const MEDIUMSLATEBLUE: Color = Color::new(123., 104., 238., 1.);
    pub const MEDIUMSPRINGGREEN: Color = Color::new(0., 250., 154., 1.);
    pub const MEDIUMTURQUOISE: Color = Color::new(72., 209., 204., 1.);
    pub const MEDIUMVIOLETRED: Color = Color::new(199., 21., 133., 1.);
    pub const MIDNIGHTBLUE: Color = Color::new(25., 25., 112., 1.);
    pub const MINTCREAM: Color = Color::new(245., 255., 250., 1.);
    pub const MISTYROSE: Color = Color::new(255., 228., 225., 1.);
    pub const MOCCASIN: Color = Color::new(255., 228., 181., 1.);
    pub const NAVAJOWHITE: Color = Color::new(255., 222., 173., 1.);
    pub const NAVY: Color = Color::new(0., 0., 128., 1.);
    pub const OLDLACE: Color = Color::new(253., 245., 230., 1.);
    pub const OLIVE: Color = Color::new(128., 128., 0., 1.);
    pub const OLIVEDRAB: Color = Color::new(107., 142., 35., 1.);
    pub const ORANGE: Color = Color::new(255., 165., 0., 1.);
    pub const ORANGERED: Color = Color::new(255., 69., 0., 1.);
    pub const ORCHID: Color = Color::new(218., 112., 214., 1.);
    pub const PALEGOLDENROD: Color = Color::new(238., 232., 170., 1.);
    pub const PALEGREEN: Color = Color::new(152., 251., 152., 1.);
    pub const PALETURQUOISE: Color = Color::new(175., 238., 238., 1.);
    pub const PALEVIOLETRED: Color = Color::new(219., 112., 147., 1.);
    pub const PAPAYAWHIP: Color = Color::new(255., 239., 213., 1.);
    pub const PEACHPUFF: Color = Color::new(255., 218., 185., 1.);
    pub const PERU: Color = Color::new(205., 133., 63., 1.);
    pub const PINK: Color = Color::new(255., 192., 203., 1.);
    pub const PLUM: Color = Color::new(221., 160., 221., 1.);
    pub const POWDERBLUE: Color = Color::new(176., 224., 230., 1.);
    pub const PURPLE: Color = Color::new(128., 0., 128., 1.);
    pub const REBECCAPURPLE: Color = Color::new(102., 51., 153., 1.);
    pub const RED: Color = Color::new(255., 0., 0., 1.);
    pub const ROSYBROWN: Color = Color::new(188., 143., 143., 1.);
    pub const ROYALBLUE: Color = Color::new(65., 105., 225., 1.);
    pub const SADDLEBROWN: Color = Color::new(139., 69., 19., 1.);
    pub const SALMON: Color = Color::new(250., 128., 114., 1.);
    pub const SANDYBROWN: Color = Color::new(244., 164., 96., 1.);
    pub const SEAGREEN: Color = Color::new(46., 139., 87., 1.);
    pub const SEASHELL: Color = Color::new(255., 245., 238., 1.);
    pub const SIENNA: Color = Color::new(160., 82., 45., 1.);
    pub const SILVER: Color = Color::new(192., 192., 192., 1.);
    pub const SKYBLUE: Color = Color::new(135., 206., 235., 1.);
    pub const SLATEBLUE: Color = Color::new(106., 90., 205., 1.);
    pub const SLATEGRAY: Color = Color::new(112., 128., 144., 1.);
    pub const SLATEGREY: Color = Color::new(112., 128., 144., 1.);
    pub const SNOW: Color = Color::new(255., 250., 250., 1.);
    pub const SPRINGGREEN: Color = Color::new(0., 255., 127., 1.);
    pub const STEELBLUE: Color = Color::new(70., 130., 180., 1.);
    pub const TAN: Color = Color::new(210., 180., 140., 1.);
    pub const TEAL: Color = Color::new(0., 128., 128., 1.);
    pub const THISTLE: Color = Color::new(216., 191., 216., 1.);
    pub const TOMATO: Color = Color::new(255., 99., 71., 1.);
    pub const TURQUOISE: Color = Color::new(64., 224., 208., 1.);
    pub const VIOLET: Color = Color::new(238., 130., 238., 1.);
    pub const WHEAT: Color = Color::new(245., 222., 179., 1.);
    pub const WHITE: Color = Color::new(255., 255., 255., 1.);
    pub const WHITESMOKE: Color = Color::new(245., 245., 245., 1.);
    pub const YELLOW: Color = Color::new(255., 255., 0., 1.);
    pub const YELLOWGREEN: Color = Color::new(154., 205., 50., 1.);
}
