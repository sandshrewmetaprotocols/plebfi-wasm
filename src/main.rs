use leptos::*;
use pulldown_cmark::{html, Parser};

const README: &str = include_str!("../README.md");

#[component]
fn App() -> impl IntoView {
    let parser = Parser::new(README);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    view! {
        <div class="container" inner_html=html_output>
        </div>
    }
}

fn main() {
    mount_to_body(|| {
        view! {
            <App />
        }
    })
}