(function() {var type_impls = {
"kludgine":[["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Clone-for-App%3CAppMessage%3E\" class=\"impl\"><a href=\"#impl-Clone-for-App%3CAppMessage%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;AppMessage&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.76.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a> for App&lt;AppMessage&gt;<div class=\"where\">where\n    AppMessage: <a class=\"trait\" href=\"kludgine/app/trait.Message.html\" title=\"trait kludgine::app::Message\">Message</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone\" class=\"method trait-impl\"><a href=\"#method.clone\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.76.0/core/clone/trait.Clone.html#tymethod.clone\" class=\"fn\">clone</a>(&amp;self) -&gt; App&lt;AppMessage&gt;</h4></section></summary><div class='docblock'>Returns a copy of the value. <a href=\"https://doc.rust-lang.org/1.76.0/core/clone/trait.Clone.html#tymethod.clone\">Read more</a></div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.clone_from\" class=\"method trait-impl\"><span class=\"rightside\"><span class=\"since\" title=\"Stable since Rust version 1.0.0\">1.0.0</span> · <a class=\"src\" href=\"https://doc.rust-lang.org/1.76.0/src/core/clone.rs.html#169\">source</a></span><a href=\"#method.clone_from\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"https://doc.rust-lang.org/1.76.0/core/clone/trait.Clone.html#method.clone_from\" class=\"fn\">clone_from</a>(&amp;mut self, source: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.76.0/std/primitive.reference.html\">&amp;Self</a>)</h4></section></summary><div class='docblock'>Performs copy-assignment from <code>source</code>. <a href=\"https://doc.rust-lang.org/1.76.0/core/clone/trait.Clone.html#method.clone_from\">Read more</a></div></details></div></details>","Clone","kludgine::app::App"],["<details class=\"toggle implementors-toggle\" open><summary><section id=\"impl-Application%3CAppMessage%3E-for-App%3CAppMessage%3E\" class=\"impl\"><a href=\"#impl-Application%3CAppMessage%3E-for-App%3CAppMessage%3E\" class=\"anchor\">§</a><h3 class=\"code-header\">impl&lt;AppMessage&gt; <a class=\"trait\" href=\"kludgine/app/trait.Application.html\" title=\"trait kludgine::app::Application\">Application</a>&lt;AppMessage&gt; for App&lt;AppMessage&gt;<div class=\"where\">where\n    AppMessage: <a class=\"trait\" href=\"kludgine/app/trait.Message.html\" title=\"trait kludgine::app::Message\">Message</a>,</div></h3></section></summary><div class=\"impl-items\"><details class=\"toggle method-toggle\" open><summary><section id=\"method.app\" class=\"method trait-impl\"><a href=\"#method.app\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"kludgine/app/trait.Application.html#tymethod.app\" class=\"fn\">app</a>(&amp;self) -&gt; App&lt;AppMessage&gt;</h4></section></summary><div class='docblock'>Returns a handle to the running application.</div></details><details class=\"toggle method-toggle\" open><summary><section id=\"method.send\" class=\"method trait-impl\"><a href=\"#method.send\" class=\"anchor\">§</a><h4 class=\"code-header\">fn <a href=\"kludgine/app/trait.Application.html#tymethod.send\" class=\"fn\">send</a>(\n    &amp;mut self,\n    message: AppMessage\n) -&gt; <a class=\"enum\" href=\"https://doc.rust-lang.org/1.76.0/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;&lt;AppMessage as <a class=\"trait\" href=\"kludgine/app/trait.Message.html\" title=\"trait kludgine::app::Message\">Message</a>&gt;::<a class=\"associatedtype\" href=\"kludgine/app/trait.Message.html#associatedtype.Response\" title=\"type kludgine::app::Message::Response\">Response</a>&gt;</h4></section></summary><div class='docblock'>Sends an app message to the main event loop to be handled by the\ncallback provided when the app was created. <a href=\"kludgine/app/trait.Application.html#tymethod.send\">Read more</a></div></details></div></details>","Application<AppMessage>","kludgine::app::App"]]
};if (window.register_type_impls) {window.register_type_impls(type_impls);} else {window.pending_type_impls = type_impls;}})()