use std::{error::Error, ffi::OsStr};

use log::*;
use tokio::fs;
use yew::{function_component, html, Html, Properties};

#[derive(PartialEq, Properties)]
pub struct ImgProp {
    pub url: String,
}

#[function_component]
pub fn Img(prop: &ImgProp) -> Html {
    html! {
        <img src={prop.url.clone()} id="img" />
    }
}

pub fn audio(url: &str) -> String {
    format!(
        r#"<audio controls autoplay id="audio">
  <source src="{}">
Your browser does not support the audio element.
</audio>"#,
        html_escape::encode_safe(url),
    )
}

pub fn video(url: &str) -> String {
    format!(
        r#"<video id="player" controls>
    <source src="{}">
Your browser does not support the video tag.
</video> "#,
        html_escape::encode_safe(url)
    )
}

pub fn html_friendly_mime(mime: &str) -> &str {
    match mime {
        "audio/x-opus+ogg" => "audio/ogg",
        _ => mime,
    }
}

pub async fn text(path: &std::path::Path) -> Result<(String, &'static str), Box<dyn Error>> {
    let content = fs::read_to_string(path).await?;
    let (class, css) = match path.extension().unwrap_or(OsStr::new("")).to_str().unwrap() {
        "tex" => ("latex", "<link href=\"/static/css/prism/tex.css\" rel=\"stylesheet\" /><script src=\"/static/scripts/prism/tex.js\"></script>"),
        "rs" => ("rust", "<link href=\"/static/css/prism/rust.css\" rel=\"stylesheet\" /><script src=\"/static/scripts/prism/rust.js\"></script>"),
        "md" => ("markdown", "<link href=\"/static/css/prism/md.css\" rel=\"stylesheet\" /><script src=\"/static/scripts/prism/md.js\"></script>"),
        ext => {
            warn!("no highlighter for {ext}");
            ("text", "<link href=\"/static/css/prism/plain.css\" rel=\"stylesheet\" /><script src=\"/static/scripts/prism/plain.js\"></script>")
        }
    };

    let content_safe = html_escape::encode_safe(&content);

    let display = format!("<pre id=\"display\" class=\"line-numbers\"><code class=\"language-{class}\">{content_safe}</code></pre>");

    Ok((display, css))
}

pub async fn html(path: &std::path::Path) -> Result<(String, &'static str), Box<dyn Error>> {
    let content = fs::read_to_string(path).await?;
    Ok((
        content,
        "<link href=\"/static/css/html.css\" rel=\"stylesheet\" />",
    ))
}
