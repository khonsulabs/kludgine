use approx::relative_eq;
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

    pub fn visible(&self) -> bool {
        !relative_eq!(self.0.a, 0.)
    }

    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.0.a = alpha;
        self
    }
}

impl Color {
    pub const CLEAR_WHITE: Color = Color::new(1., 1., 1., 0.);
    pub const CLEAR_BLACK: Color = Color::new(0., 0., 0., 0.);

    // CSS-name derived constants
    pub const ALICEBLUE: Color = Color::new(240. / 255., 248. / 255., 1., 1.);
    pub const ANTIQUEWHITE: Color = Color::new(250. / 255., 235. / 255., 215. / 255., 1.);
    pub const AQUA: Color = Color::new(0., 1., 1., 1.);
    pub const AQUAMARINE: Color = Color::new(127. / 255., 1., 212. / 255., 1.);
    pub const AZURE: Color = Color::new(240. / 255., 1., 1., 1.);
    pub const BEIGE: Color = Color::new(245. / 255., 245. / 255., 220. / 255., 1.);
    pub const BISQUE: Color = Color::new(1., 228. / 255., 196. / 255., 1.);
    pub const BLACK: Color = Color::new(0., 0., 0., 1.);
    pub const BLANCHEDALMOND: Color = Color::new(1., 235. / 255., 205. / 255., 1.);
    pub const BLUE: Color = Color::new(0., 0., 1., 1.);
    pub const BLUEVIOLET: Color = Color::new(138. / 255., 43. / 255., 226. / 255., 1.);
    pub const BROWN: Color = Color::new(165. / 255., 42. / 255., 42. / 255., 1.);
    pub const BURLYWOOD: Color = Color::new(222. / 255., 184. / 255., 135. / 255., 1.);
    pub const CADETBLUE: Color = Color::new(95. / 255., 158. / 255., 160. / 255., 1.);
    pub const CHARTREUSE: Color = Color::new(127. / 255., 1., 0., 1.);
    pub const CHOCOLATE: Color = Color::new(210. / 255., 105. / 255., 30. / 255., 1.);
    pub const CORAL: Color = Color::new(1., 127. / 255., 80. / 255., 1.);
    pub const CORNFLOWERBLUE: Color = Color::new(100. / 255., 149. / 255., 237. / 255., 1.);
    pub const CORNSILK: Color = Color::new(1., 248. / 255., 220. / 255., 1.);
    pub const CRIMSON: Color = Color::new(220. / 255., 20. / 255., 60. / 255., 1.);
    pub const CYAN: Color = Color::new(0., 1., 1., 1.);
    pub const DARKBLUE: Color = Color::new(0., 0., 139. / 255., 1.);
    pub const DARKCYAN: Color = Color::new(0., 139. / 255., 139. / 255., 1.);
    pub const DARKGOLDENROD: Color = Color::new(184. / 255., 134. / 255., 11. / 255., 1.);
    pub const DARKGRAY: Color = Color::new(169. / 255., 169. / 255., 169. / 255., 1.);
    pub const DARKGREEN: Color = Color::new(0., 100. / 255., 0., 1.);
    pub const DARKGREY: Color = Color::new(169. / 255., 169. / 255., 169. / 255., 1.);
    pub const DARKKHAKI: Color = Color::new(189. / 255., 183. / 255., 107. / 255., 1.);
    pub const DARKMAGENTA: Color = Color::new(139. / 255., 0., 139. / 255., 1.);
    pub const DARKOLIVEGREEN: Color = Color::new(85. / 255., 107. / 255., 47. / 255., 1.);
    pub const DARKORANGE: Color = Color::new(1., 140. / 255., 0., 1.);
    pub const DARKORCHID: Color = Color::new(153. / 255., 50. / 255., 204. / 255., 1.);
    pub const DARKRED: Color = Color::new(139. / 255., 0., 0., 1.);
    pub const DARKSALMON: Color = Color::new(233. / 255., 150. / 255., 122. / 255., 1.);
    pub const DARKSEAGREEN: Color = Color::new(143. / 255., 188. / 255., 143. / 255., 1.);
    pub const DARKSLATEBLUE: Color = Color::new(72. / 255., 61. / 255., 139. / 255., 1.);
    pub const DARKSLATEGRAY: Color = Color::new(47. / 255., 79. / 255., 79. / 255., 1.);
    pub const DARKSLATEGREY: Color = Color::new(47. / 255., 79. / 255., 79. / 255., 1.);
    pub const DARKTURQUOISE: Color = Color::new(0., 206. / 255., 209. / 255., 1.);
    pub const DARKVIOLET: Color = Color::new(148. / 255., 0., 211. / 255., 1.);
    pub const DEEPPINK: Color = Color::new(1., 20. / 255., 147. / 255., 1.);
    pub const DEEPSKYBLUE: Color = Color::new(0., 191. / 255., 1., 1.);
    pub const DIMGRAY: Color = Color::new(105. / 255., 105. / 255., 105. / 255., 1.);
    pub const DIMGREY: Color = Color::new(105. / 255., 105. / 255., 105. / 255., 1.);
    pub const DODGERBLUE: Color = Color::new(30. / 255., 144. / 255., 1., 1.);
    pub const FIREBRICK: Color = Color::new(178. / 255., 34. / 255., 34. / 255., 1.);
    pub const FLORALWHITE: Color = Color::new(1., 250. / 255., 240. / 255., 1.);
    pub const FORESTGREEN: Color = Color::new(34. / 255., 139. / 255., 34. / 255., 1.);
    pub const FUCHSIA: Color = Color::new(1., 0., 1., 1.);
    pub const GAINSBORO: Color = Color::new(220. / 255., 220. / 255., 220. / 255., 1.);
    pub const GHOSTWHITE: Color = Color::new(248. / 255., 248. / 255., 1., 1.);
    pub const GOLD: Color = Color::new(1., 215. / 255., 0., 1.);
    pub const GOLDENROD: Color = Color::new(218. / 255., 165. / 255., 32. / 255., 1.);
    pub const GRAY: Color = Color::new(128. / 255., 128. / 255., 128. / 255., 1.);
    pub const GREY: Color = Color::new(128. / 255., 128. / 255., 128. / 255., 1.);
    pub const GREEN: Color = Color::new(0., 128. / 255., 0., 1.);
    pub const GREENYELLOW: Color = Color::new(173. / 255., 1., 47. / 255., 1.);
    pub const HONEYDEW: Color = Color::new(240. / 255., 1., 240. / 255., 1.);
    pub const HOTPINK: Color = Color::new(1., 105. / 255., 180. / 255., 1.);
    pub const INDIANRED: Color = Color::new(205. / 255., 92. / 255., 92. / 255., 1.);
    pub const INDIGO: Color = Color::new(75. / 255., 0., 130. / 255., 1.);
    pub const IVORY: Color = Color::new(1., 1., 240. / 255., 1.);
    pub const KHAKI: Color = Color::new(240. / 255., 230. / 255., 140. / 255., 1.);
    pub const LAVENDER: Color = Color::new(230. / 255., 230. / 255., 250. / 255., 1.);
    pub const LAVENDERBLUSH: Color = Color::new(1., 240. / 255., 245. / 255., 1.);
    pub const LAWNGREEN: Color = Color::new(124. / 255., 252. / 255., 0., 1.);
    pub const LEMONCHIFFON: Color = Color::new(1., 250. / 255., 205. / 255., 1.);
    pub const LIGHTBLUE: Color = Color::new(173. / 255., 216. / 255., 230. / 255., 1.);
    pub const LIGHTCORAL: Color = Color::new(240. / 255., 128. / 255., 128. / 255., 1.);
    pub const LIGHTCYAN: Color = Color::new(224. / 255., 1., 1., 1.);
    pub const LIGHTGOLDENRODYELLOW: Color = Color::new(250. / 255., 250. / 255., 210. / 255., 1.);
    pub const LIGHTGRAY: Color = Color::new(211. / 255., 211. / 255., 211. / 255., 1.);
    pub const LIGHTGREEN: Color = Color::new(144. / 255., 238. / 255., 144. / 255., 1.);
    pub const LIGHTGREY: Color = Color::new(211. / 255., 211. / 255., 211. / 255., 1.);
    pub const LIGHTPINK: Color = Color::new(1., 182. / 255., 193. / 255., 1.);
    pub const LIGHTSALMON: Color = Color::new(1., 160. / 255., 122. / 255., 1.);
    pub const LIGHTSEAGREEN: Color = Color::new(32. / 255., 178. / 255., 170. / 255., 1.);
    pub const LIGHTSKYBLUE: Color = Color::new(135. / 255., 206. / 255., 250. / 255., 1.);
    pub const LIGHTSLATEGRAY: Color = Color::new(119. / 255., 136. / 255., 153. / 255., 1.);
    pub const LIGHTSLATEGREY: Color = Color::new(119. / 255., 136. / 255., 153. / 255., 1.);
    pub const LIGHTSTEELBLUE: Color = Color::new(176. / 255., 196. / 255., 222. / 255., 1.);
    pub const LIGHTYELLOW: Color = Color::new(1., 1., 224. / 255., 1.);
    pub const LIME: Color = Color::new(0., 1., 0., 1.);
    pub const LIMEGREEN: Color = Color::new(50. / 255., 205. / 255., 50. / 255., 1.);
    pub const LINEN: Color = Color::new(250. / 255., 240. / 255., 230. / 255., 1.);
    pub const MAGENTA: Color = Color::new(1., 0., 1., 1.);
    pub const MAROON: Color = Color::new(128. / 255., 0., 0., 1.);
    pub const MEDIUMAQUAMARINE: Color = Color::new(102. / 255., 205. / 255., 170. / 255., 1.);
    pub const MEDIUMBLUE: Color = Color::new(0., 0., 205. / 255., 1.);
    pub const MEDIUMORCHID: Color = Color::new(186. / 255., 85. / 255., 211. / 255., 1.);
    pub const MEDIUMPURPLE: Color = Color::new(147. / 255., 112. / 255., 219. / 255., 1.);
    pub const MEDIUMSEAGREEN: Color = Color::new(60. / 255., 179. / 255., 113. / 255., 1.);
    pub const MEDIUMSLATEBLUE: Color = Color::new(123. / 255., 104. / 255., 238. / 255., 1.);
    pub const MEDIUMSPRINGGREEN: Color = Color::new(0., 250. / 255., 154. / 255., 1.);
    pub const MEDIUMTURQUOISE: Color = Color::new(72. / 255., 209. / 255., 204. / 255., 1.);
    pub const MEDIUMVIOLETRED: Color = Color::new(199. / 255., 21. / 255., 133. / 255., 1.);
    pub const MIDNIGHTBLUE: Color = Color::new(25. / 255., 25. / 255., 112. / 255., 1.);
    pub const MINTCREAM: Color = Color::new(245. / 255., 1., 250. / 255., 1.);
    pub const MISTYROSE: Color = Color::new(1., 228. / 255., 225. / 255., 1.);
    pub const MOCCASIN: Color = Color::new(1., 228. / 255., 181. / 255., 1.);
    pub const NAVAJOWHITE: Color = Color::new(1., 222. / 255., 173. / 255., 1.);
    pub const NAVY: Color = Color::new(0., 0., 128. / 255., 1.);
    pub const OLDLACE: Color = Color::new(253. / 255., 245. / 255., 230. / 255., 1.);
    pub const OLIVE: Color = Color::new(128. / 255., 128. / 255., 0., 1.);
    pub const OLIVEDRAB: Color = Color::new(107. / 255., 142. / 255., 35. / 255., 1.);
    pub const ORANGE: Color = Color::new(1., 165. / 255., 0., 1.);
    pub const ORANGERED: Color = Color::new(1., 69. / 255., 0., 1.);
    pub const ORCHID: Color = Color::new(218. / 255., 112. / 255., 214. / 255., 1.);
    pub const PALEGOLDENROD: Color = Color::new(238. / 255., 232. / 255., 170. / 255., 1.);
    pub const PALEGREEN: Color = Color::new(152. / 255., 251. / 255., 152. / 255., 1.);
    pub const PALETURQUOISE: Color = Color::new(175. / 255., 238. / 255., 238. / 255., 1.);
    pub const PALEVIOLETRED: Color = Color::new(219. / 255., 112. / 255., 147. / 255., 1.);
    pub const PAPAYAWHIP: Color = Color::new(1., 239. / 255., 213. / 255., 1.);
    pub const PEACHPUFF: Color = Color::new(1., 218. / 255., 185. / 255., 1.);
    pub const PERU: Color = Color::new(205. / 255., 133. / 255., 63. / 255., 1.);
    pub const PINK: Color = Color::new(1., 192. / 255., 203. / 255., 1.);
    pub const PLUM: Color = Color::new(221. / 255., 160. / 255., 221. / 255., 1.);
    pub const POWDERBLUE: Color = Color::new(176. / 255., 224. / 255., 230. / 255., 1.);
    pub const PURPLE: Color = Color::new(128. / 255., 0., 128. / 255., 1.);
    pub const REBECCAPURPLE: Color = Color::new(102. / 255., 51. / 255., 153. / 255., 1.);
    pub const RED: Color = Color::new(1., 0., 0., 1.);
    pub const ROSYBROWN: Color = Color::new(188. / 255., 143. / 255., 143. / 255., 1.);
    pub const ROYALBLUE: Color = Color::new(65. / 255., 105. / 255., 225. / 255., 1.);
    pub const SADDLEBROWN: Color = Color::new(139. / 255., 69. / 255., 19. / 255., 1.);
    pub const SALMON: Color = Color::new(250. / 255., 128. / 255., 114. / 255., 1.);
    pub const SANDYBROWN: Color = Color::new(244. / 255., 164. / 255., 96. / 255., 1.);
    pub const SEAGREEN: Color = Color::new(46. / 255., 139. / 255., 87. / 255., 1.);
    pub const SEASHELL: Color = Color::new(1., 245. / 255., 238. / 255., 1.);
    pub const SIENNA: Color = Color::new(160. / 255., 82. / 255., 45. / 255., 1.);
    pub const SILVER: Color = Color::new(192. / 255., 192. / 255., 192. / 255., 1.);
    pub const SKYBLUE: Color = Color::new(135. / 255., 206. / 255., 235. / 255., 1.);
    pub const SLATEBLUE: Color = Color::new(106. / 255., 90. / 255., 205. / 255., 1.);
    pub const SLATEGRAY: Color = Color::new(112. / 255., 128. / 255., 144. / 255., 1.);
    pub const SLATEGREY: Color = Color::new(112. / 255., 128. / 255., 144. / 255., 1.);
    pub const SNOW: Color = Color::new(1., 250. / 255., 250. / 255., 1.);
    pub const SPRINGGREEN: Color = Color::new(0., 1., 127. / 255., 1.);
    pub const STEELBLUE: Color = Color::new(70. / 255., 130. / 255., 180. / 255., 1.);
    pub const TAN: Color = Color::new(210. / 255., 180. / 255., 140. / 255., 1.);
    pub const TEAL: Color = Color::new(0., 128. / 255., 128. / 255., 1.);
    pub const THISTLE: Color = Color::new(216. / 255., 191. / 255., 216. / 255., 1.);
    pub const TOMATO: Color = Color::new(1., 99. / 255., 71. / 255., 1.);
    pub const TURQUOISE: Color = Color::new(64. / 255., 224. / 255., 208. / 255., 1.);
    pub const VIOLET: Color = Color::new(238. / 255., 130. / 255., 238. / 255., 1.);
    pub const WHEAT: Color = Color::new(245. / 255., 222. / 255., 179. / 255., 1.);
    pub const WHITE: Color = Color::new(1., 1., 1., 1.);
    pub const WHITESMOKE: Color = Color::new(245. / 255., 245. / 255., 245. / 255., 1.);
    pub const YELLOW: Color = Color::new(1., 1., 0., 1.);
    pub const YELLOWGREEN: Color = Color::new(154. / 255., 205. / 255., 50. / 255., 1.);
}
