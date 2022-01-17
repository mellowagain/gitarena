use crate::templates::plain::Template;
use crate::utils::time_function;

use std::path::Path;

use anyhow::Result;
use async_compat::Compat;
use futures::executor;
use futures_locks::RwLock;
use lazy_static::lazy_static;
use log::{error, info};
use notify::{Error as NotifyError, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tera::Tera;

mod filters;
mod tests;

pub(crate) mod plain;
pub(crate) mod web;

lazy_static! {
    pub(crate) static ref VERIFY_EMAIL: Template = parse_template("email/user/verify_email.txt".to_owned());
    pub(crate) static ref TERA: RwLock<Tera> = RwLock::new(init_tera());
}

pub(crate) async fn init() -> Result<RecommendedWatcher> {
    info!("Loading templates. This may take a while.");

    let elapsed = time_function(|| async {
        // Initialize the `TERA` lazy variable immediately in order to check for template errors at init
        TERA.read().await;
    }).await;

    info!("Successfully loaded templates. Took {} seconds.", elapsed);

    let mut watcher = RecommendedWatcher::new(|result: std::result::Result<Event, NotifyError>| {
        let event = match result {
            Ok(event) => event,
            Err(err) => {
                error!("Failed to unwrap file system notify event: {}", err);
                return;
            }
        };

        if !event.kind.is_modify() {
            return;
        }

        for path in &event.paths {
            if path.is_dir() {
                return;
            }

            match path.file_name() {
                Some(file_name) => match file_name.to_str() {
                    Some(file_name) => if !file_name.ends_with(".html") {
                        return
                    }
                    None => return
                }
                None => return
            }
        }

        info!("Detected modification in templates directory, reloading...");

        executor::block_on(Compat::new(async {
            match TERA.write().await.full_reload() {
                Ok(_) => info!("Successfully reloaded templates."),
                Err(err) => error!("Failed to reload templates: {}", err)
            }
        }));
    })?;

    watcher.watch(Path::new("templates/html"), RecursiveMode::Recursive)?;

    info!("Started watching ./templates/html for changes...");

    Ok(watcher)
}

fn parse_template(template_path: String) -> Template {
    match plain::parse(template_path) {
        Ok(template) => template,
        Err(err) => panic!("Failed to parse template: {}", err)
    }
}

fn init_tera() -> Tera {
    let mut tera = match Tera::new("templates/html/**/*") {
        Ok(tera) => tera,
        Err(err) => panic!("{}", err)
    };

    tera.register_filter("human_prefix", filters::human_prefix);
    tera.register_filter("human_time", filters::human_time);

    tera.register_tester("empty", tests::empty);
    tera.register_tester("none", tests::none);
    tera.register_tester("some", tests::some);

    tera
}

#[macro_export]
macro_rules! template_context {
    ($input:expr) => {
        Some($input.iter().cloned().collect())
    }
}

/// Renders a template and returns `Ok(HttpResponse)`. If an error occurs, returns `Err`.
///
/// - If `$transaction` is passed, both `debug` (if in debug mode) and `domain` gets inserted into the context additionally.
/// - If `$transaction` is not passed, only `debug` (if in debug mode) gets inserted into the context additionally.
#[macro_export]
macro_rules! render_template {
    ($template_name:literal, $context:expr) => {{
        render_template!(actix_web::http::StatusCode::OK, $template_name, $context)
    }};
    ($template_name:literal, $context:expr, $transaction:expr) => {{
        render_template!(actix_web::http::StatusCode::OK, $template_name, $context, $transaction)
    }};
    ($status:expr, $template_name:literal, $context:expr) => {{
        if cfg!(debug_assertions) {
            $context.try_insert("debug", &true)?;
        }

        let template = $crate::templates::TERA.read().await.render($template_name, &$context)?;
        Ok(actix_web::HttpResponseBuilder::new($status).body(template))
    }};
    ($status:expr, $template_name:literal, $context:expr, $transaction:expr) => {{
        let domain = $crate::config::get_optional_setting::<String, _>("domain", &mut $transaction).await?.unwrap_or_default();
        $context.try_insert("domain", &domain)?;

        if cfg!(debug_assertions) {
            $context.try_insert("debug", &true)?;
        }

        let template = $crate::templates::TERA.read().await.render($template_name, &$context)?;

        $transaction.commit().await?;

        Ok(actix_web::HttpResponseBuilder::new($status).body(template))
    }};
}
