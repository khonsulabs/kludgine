(function() {var implementors = {
"kludgine":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.Color.html\" title=\"struct kludgine::Color\">Color</a>&gt; for Color"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.Texture.html\" title=\"struct kludgine::Texture\">Texture</a>&gt; for <a class=\"struct\" href=\"kludgine/struct.SharedTexture.html\" title=\"struct kludgine::SharedTexture\">SharedTexture</a>"],["impl&lt;'a, Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/text/struct.Text.html\" title=\"struct kludgine::text::Text\">Text</a>&lt;'a, Unit&gt;&gt; for <a class=\"struct\" href=\"kludgine/struct.Drawable.html\" title=\"struct kludgine::Drawable\">Drawable</a>&lt;<a class=\"struct\" href=\"kludgine/text/struct.Text.html\" title=\"struct kludgine::text::Text\">Text</a>&lt;'a, Unit&gt;, Unit&gt;<span class=\"where fmt-newline\">where\n    Unit: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</span>"],["impl&lt;Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Point&lt;Unit&gt;&gt; for <a class=\"struct\" href=\"kludgine/shapes/struct.Endpoint.html\" title=\"struct kludgine::shapes::Endpoint\">Endpoint</a>&lt;Unit&gt;"],["impl&lt;Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.Color.html\" title=\"struct kludgine::Color\">Color</a>&gt; for <a class=\"struct\" href=\"kludgine/shapes/struct.StrokeOptions.html\" title=\"struct kludgine::shapes::StrokeOptions\">StrokeOptions</a>&lt;Unit&gt;<span class=\"where fmt-newline\">where\n    Unit: <a class=\"trait\" href=\"kludgine/shapes/trait.DefaultStrokeWidth.html\" title=\"trait kludgine::shapes::DefaultStrokeWidth\">DefaultStrokeWidth</a>,</span>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.SharedTexture.html\" title=\"struct kludgine::SharedTexture\">SharedTexture</a>&gt; for <a class=\"struct\" href=\"kludgine/struct.TextureRegion.html\" title=\"struct kludgine::TextureRegion\">TextureRegion</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.Texture.html\" title=\"struct kludgine::Texture\">Texture</a>&gt; for <a class=\"enum\" href=\"kludgine/enum.AnyTexture.html\" title=\"enum kludgine::AnyTexture\">AnyTexture</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.CollectedTexture.html\" title=\"struct kludgine::CollectedTexture\">CollectedTexture</a>&gt; for <a class=\"enum\" href=\"kludgine/enum.AnyTexture.html\" title=\"enum kludgine::AnyTexture\">AnyTexture</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Error&lt;<a class=\"enum\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/enum.Infallible.html\" title=\"enum core::convert::Infallible\">Infallible</a>&gt;&gt; for <a class=\"enum\" href=\"kludgine/sprite/enum.SpriteParseError.html\" title=\"enum kludgine::sprite::SpriteParseError\">SpriteParseError</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Color&gt; for <a class=\"struct\" href=\"kludgine/struct.Color.html\" title=\"struct kludgine::Color\">Color</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.TextureRegion.html\" title=\"struct kludgine::TextureRegion\">TextureRegion</a>&gt; for <a class=\"enum\" href=\"kludgine/sprite/enum.SpriteSource.html\" title=\"enum kludgine::sprite::SpriteSource\">SpriteSource</a>"],["impl&lt;Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;(Point&lt;Unit&gt;, <a class=\"struct\" href=\"kludgine/struct.Color.html\" title=\"struct kludgine::Color\">Color</a>)&gt; for <a class=\"struct\" href=\"kludgine/shapes/struct.Endpoint.html\" title=\"struct kludgine::shapes::Endpoint\">Endpoint</a>&lt;Unit&gt;"],["impl&lt;'a, T, Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.74.1/std/primitive.reference.html\">&amp;'a T</a>&gt; for <a class=\"struct\" href=\"kludgine/struct.Drawable.html\" title=\"struct kludgine::Drawable\">Drawable</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.74.1/std/primitive.reference.html\">&amp;'a T</a>, Unit&gt;<span class=\"where fmt-newline\">where\n    T: <a class=\"trait\" href=\"kludgine/trait.DrawableSource.html\" title=\"trait kludgine::DrawableSource\">DrawableSource</a>,\n    Unit: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</span>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.Color.html\" title=\"struct kludgine::Color\">Color</a>&gt; for Color"],["impl&lt;'a, Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;&amp;'a <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.74.1/std/primitive.str.html\">str</a>&gt; for <a class=\"struct\" href=\"kludgine/text/struct.Text.html\" title=\"struct kludgine::text::Text\">Text</a>&lt;'a, Unit&gt;"],["impl&lt;'a, Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;&amp;'a <a class=\"struct\" href=\"https://doc.rust-lang.org/1.74.1/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>&gt; for <a class=\"struct\" href=\"kludgine/text/struct.Text.html\" title=\"struct kludgine::text::Text\">Text</a>&lt;'a, Unit&gt;"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;ImageError&gt; for <a class=\"enum\" href=\"kludgine/sprite/enum.SpriteParseError.html\" title=\"enum kludgine::sprite::SpriteParseError\">SpriteParseError</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.SharedTexture.html\" title=\"struct kludgine::SharedTexture\">SharedTexture</a>&gt; for <a class=\"enum\" href=\"kludgine/enum.AnyTexture.html\" title=\"enum kludgine::AnyTexture\">AnyTexture</a>"],["impl&lt;Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/shapes/struct.StrokeOptions.html\" title=\"struct kludgine::shapes::StrokeOptions\">StrokeOptions</a>&lt;Unit&gt;&gt; for StrokeOptions<span class=\"where fmt-newline\">where\n    Unit: FloatConversion&lt;Float = <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.74.1/std/primitive.f32.html\">f32</a>&gt;,</span>"],["impl&lt;Unit&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Unit&gt; for <a class=\"struct\" href=\"kludgine/shapes/struct.CornerRadii.html\" title=\"struct kludgine::shapes::CornerRadii\">CornerRadii</a>&lt;Unit&gt;<span class=\"where fmt-newline\">where\n    Unit: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/marker/trait.Copy.html\" title=\"trait core::marker::Copy\">Copy</a>,</span>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.CollectedTexture.html\" title=\"struct kludgine::CollectedTexture\">CollectedTexture</a>&gt; for <a class=\"enum\" href=\"kludgine/sprite/enum.SpriteSource.html\" title=\"enum kludgine::sprite::SpriteSource\">SpriteSource</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/sprite/struct.SpriteAnimations.html\" title=\"struct kludgine::sprite::SpriteAnimations\">SpriteAnimations</a>&gt; for <a class=\"struct\" href=\"kludgine/sprite/struct.Sprite.html\" title=\"struct kludgine::sprite::Sprite\">Sprite</a>"],["impl&lt;Unit, const TEXTURED: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.74.1/std/primitive.bool.html\">bool</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/shapes/struct.Path.html\" title=\"struct kludgine::shapes::Path\">Path</a>&lt;Unit, TEXTURED&gt;&gt; for <a class=\"struct\" href=\"kludgine/shapes/struct.PathBuilder.html\" title=\"struct kludgine::shapes::PathBuilder\">PathBuilder</a>&lt;Unit, TEXTURED&gt;<span class=\"where fmt-newline\">where\n    Unit: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</span>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.74.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"kludgine/struct.TextureRegion.html\" title=\"struct kludgine::TextureRegion\">TextureRegion</a>&gt; for <a class=\"enum\" href=\"kludgine/enum.AnyTexture.html\" title=\"enum kludgine::AnyTexture\">AnyTexture</a>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()