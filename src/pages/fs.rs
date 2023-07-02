use std::{borrow::Cow, error::Error, path::PathBuf};

use actix_files::NamedFile;
use actix_web::{
    get,
    http::header::{ContentDisposition, ContentType},
    web::Path,
    HttpRequest, HttpResponse,
};
use goodmorning_bindings::services::v1::V1Error;
use goodmorning_services::{
    functions::{dir_items, get_user_dir},
    structs::{Account, GMServices, ItemVisibility, Visibilities},
    traits::CollectionItem,
    ACCOUNTS, MIME_DB,
};
use tokio::fs;

use crate::{
    components::{self, audio, topbar_from_req, FsItem, FsItemProp, Img, ImgProp, PathProp},
    functions::{from_res, gen_nonce},
};

#[get("/fs/{id}/{path:.*}")]
pub async fn fspath(path: Path<(i64, String)>, req: HttpRequest) -> HttpResponse {
    from_res(fs_task(path, &req).await, &req).await
}

#[get("/fs/{id}")]
pub async fn root(path: Path<i64>, req: HttpRequest) -> HttpResponse {
    from_res(
        fs_task(Path::from((path.into_inner(), String::new())), &req).await,
        &req,
    )
    .await
}

async fn fs_task(
    path: Path<(i64, String)>,
    req: &HttpRequest,
) -> Result<HttpResponse, Box<dyn Error>> {
    let (id, path) = path.into_inner();

    let (topbar, account) = match topbar_from_req(req).await? {
        Ok(stuff) => stuff,
        Err(res) => return Ok(res),
    };

    let is_owner = account.as_ref().is_some_and(|account| account.id == id);

    let account = if account.is_some() && account.as_ref().unwrap().id == id {
        account.unwrap()
    } else {
        match Account::find_by_id(id, ACCOUNTS.get().unwrap()).await? {
            Some(account) => account,
            None => {
                return Ok(NamedFile::open_async("static/htmls/notfound.html")
                    .await?
                    .into_response(req))
            }
        }
    };

    // get_user_dir(account.id, None).join(&path);
    let pathbuf = get_user_dir(account.id, Some(GMServices::Tex)).join(&path);

    if !fs::try_exists(&pathbuf).await? {
        return Err(V1Error::FileNotFound.into());
    }

    let metadata = fs::metadata(&pathbuf).await?;

    if metadata.is_dir() {
        dir(account, path, topbar, is_owner).await
    } else {
        file(
            account,
            pathbuf,
            path,
            topbar,
            is_owner,
            metadata.len(),
            req,
        )
        .await
    }
}

async fn dir(
    account: Account,
    path: String,
    topbar: Cow<'_, str>,
    is_owner: bool,
) -> Result<HttpResponse, Box<dyn Error>> {
    let items = dir_items(
        account.id,
        &std::path::Path::new("tex").join(&path),
        is_owner,
        false,
    )
    .await?;
    let nonce = gen_nonce();
    let csp_heaher = format!("default-src 'self'; script-src 'self' 'nonce-{nonce}'");
    let items_props = FsItemProp {
        nonce,
        id: account.id,
        items: items.into_iter().map(FsItem::from).collect(),
        path: path.clone(),
    };
    let items_display = yew::ServerRenderer::<components::FsItems>::with_props(|| items_props)
        .render()
        .await;

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="/static/css/main.css" />
    <link rel="stylesheet" href="/static/css/topbar.css" />
    <link rel="stylesheet" href="/static/css/topbar-signedout.css" />
    <link rel="stylesheet" href="/static/css/fs.css" />
    <link rel="stylesheet" href="/static/css/path.css" />
    <link rel="stylesheet" href="/static/css/topbar-loggedin.css" />
    <link rel="stylesheet" href="/static/css/dark/main.css" />
    <link rel="stylesheet" href="/static/css/dark/topbar.css" />
    <link rel="stylesheet" href="/static/css/dark/fs.css" />
    <link rel="stylesheet" href="/static/css/dark/path.css" />
    <link rel="stylesheet" href="/static/css/dark/topbar-signedout.css" />
    <link
      rel="shortcut icon"
      href="/static/images/favicon-dark.svg"
      type="image/x-icon"
    />
    <title>{}</title>
  </head>
  <body>
  {topbar}
  {items_display}
  <script src="/static/scripts/fs.js" defer></script>
  </body>
</html>"#,
        html_escape::encode_safe(&format!("{}/{path}", account.id))
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .insert_header(("Content-Security-Policy", csp_heaher))
        .body(html))
}

async fn file(
    account: Account,
    pathbuf: PathBuf,
    path: String,
    topbar: Cow<'_, str>,
    is_owner: bool,
    size: u64,
    req: &HttpRequest,
) -> Result<HttpResponse, Box<dyn Error>> {
    let visibility = Visibilities::visibility(&pathbuf).await?;

    if visibility.visibility == ItemVisibility::Private && !is_owner {
        return Err(V1Error::FileNotFound.into());
    }

    let url = if is_owner {
        format!("/api/storage/v1/file/{}/tex/{}", account.token, path)
    } else {
        format!("/api/usercontent/v1/file/id/{}/tex/{}", account.id, path)
    };

    let mimes = MIME_DB
        .get()
        .unwrap()
        .get_mime_types_from_file_name(pathbuf.file_name().unwrap().to_str().unwrap());
    let mime = match mimes.first() {
        Some(mime) => mime.clone(),
        None => todo!(),
    };

    let (display, css) = match (mime.type_(), mime.subtype()) {
        (mime::IMAGE, _) => (
            yew::ServerRenderer::<Img>::with_props(move || ImgProp { url })
                .render()
                .await,
            "<link rel=\"stylesheet\" href=\"/static/css/img.css\" />",
        ),
        (mime::AUDIO, _) => (
            audio(&mime, &url),
            "<link rel=\"stylesheet\" href=\"/static/css/audio.css\" />",
        ),
        (_, mime::PDF) => {
            return Ok(NamedFile::open_async(&pathbuf)
                .await?
                .set_content_disposition(ContentDisposition {
                    disposition: actix_web::http::header::DispositionType::Inline,
                    parameters: Vec::new(),
                })
                .into_response(req))
        }
        mime => todo!("{mime:?}"),
    };

    let path_display = yew::ServerRenderer::<components::Path>::with_props(move || PathProp {
        id: account.id,
        path,
    })
    .render()
    .await;
    let info_unsafe = format!("{} {}", &mime, crate::functions::size(size));
    let info = html_escape::encode_safe(&info_unsafe);

    let html = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="/static/css/main.css" />
    <link rel="stylesheet" href="/static/css/topbar.css" />
    <link rel="stylesheet" href="/static/css/topbar-signedout.css" />
    <link rel="stylesheet" href="/static/css/path.css" />
    <link rel="stylesheet" href="/static/css/topbar-loggedin.css" />
    <link rel="stylesheet" href="/static/css/dark/main.css" />
    <link rel="stylesheet" href="/static/css/dark/topbar.css" />
    <link rel="stylesheet" href="/static/css/dark/path.css" />
    <link rel="stylesheet" href="/static/css/dark/topbar-signedout.css" />
    {css}
    <script src="/static/scripts/file.js" defer></script>
    <link
      rel="shortcut icon"
      href="/static/images/favicon-dark.svg"
      type="image/x-icon"
    />
    <title>Usercontent - GoodMorning Tex</title>
  </head>
  <body>
    {topbar}
    {path_display}
    <div id="display">
        {display}
        <br />
        <code id="info">{info}</code>
    </div>
  </body>
</html>"#
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .insert_header((
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'",
        ))
        .body(html))
}
