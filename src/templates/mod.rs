use crate::templates::plain::Template;
use crate::utils::time_function;

use anyhow::Result;
use log::info;
use once_cell::sync::OnceCell;
use tera::{Context, Tera};
use tracing_unwrap::{OptionExt, ResultExt};

mod filters;
mod tests;

pub(crate) mod plain;
pub(crate) mod web;

#[cfg(debug_assertions)]
type GlobalTera = futures_locks::RwLock<Tera>;

#[cfg(debug_assertions)]
type TemplateInitResult = notify::RecommendedWatcher;

#[cfg(not(debug_assertions))]
type GlobalTera = Tera;

#[cfg(not(debug_assertions))]
type TemplateInitResult = ();

pub(crate) static VERIFY_EMAIL: OnceCell<Template> = OnceCell::new();
static TERA: OnceCell<GlobalTera> = OnceCell::new();

pub(crate) async fn init() -> Result<TemplateInitResult> {
    info!("Loading templates. This may take a few seconds.");

    let elapsed = time_function(|| async {
        VERIFY_EMAIL
            .set(parse_template("email/user/verify_email.txt".to_owned()))
            .expect_or_log("Verify email template should only be initialized once");

        // This additionally checks the templates for errors
        TERA.set(init_tera())
            .expect_or_log("Tera should only be initialized once");
    })
    .await;

    info!("Successfully loaded templates. Took {} seconds.", elapsed);

    #[cfg(debug_assertions)]
    {
        use std::path::Path;

        use actix_web::rt::Runtime;
        use log::error;
        use notify::{Error as NotifyError, Event, RecommendedWatcher, RecursiveMode, Watcher};

        let mut watcher =
            notify::recommended_watcher(|result: std::result::Result<Event, NotifyError>| {
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
                            Some(file_name) => {
                                if !file_name.ends_with(".html") {
                                    return;
                                }
                            }
                            None => return,
                        },
                        None => return,
                    }
                }

                if let Ok(runtime) = Runtime::new() {
                    info!("Detected modification in templates directory, reloading...");

                    runtime.block_on(async {
                        match tera().write().await.full_reload() {
                            Ok(_) => info!("Successfully reloaded templates."),
                            Err(err) => error!("Failed to reload templates: {}", err),
                        }
                    });
                }
            })?;

        watcher.watch(Path::new("templates/html"), RecursiveMode::Recursive)?;

        info!("Started watching ./templates/html for changes...");

        Ok(watcher)
    }

    #[cfg(not(debug_assertions))]
    Ok(())
}

fn parse_template(template_path: String) -> Template {
    match plain::parse(template_path) {
        Ok(template) => template,
        Err(err) => panic!("Failed to parse template: {}", err),
    }
}

pub(crate) async fn render(template: &str, context: &Context) -> Result<String> {
    #[cfg(debug_assertions)]
    return Ok(tera().read().await.render(template, context)?);

    #[cfg(not(debug_assertions))]
    return Ok(tera().render(template, context)?);
}

fn init_tera() -> GlobalTera {
    let mut tera = match Tera::new("templates/html/**/*") {
        Ok(tera) => tera,
        Err(err) => panic!("{}", err),
    };

    tera.register_filter("human_prefix", filters::human_prefix);
    tera.register_filter("human_time", filters::human_time);

    tera.register_tester("empty", tests::empty);
    tera.register_tester("none", tests::none);
    tera.register_tester("some", tests::some);

    #[cfg(debug_assertions)]
    return futures_locks::RwLock::new(tera);

    #[cfg(not(debug_assertions))]
    return tera;
}

pub(crate) fn tera() -> &'static GlobalTera {
    TERA.get().unwrap_or_log()
}

#[macro_export]
macro_rules! template_context {
    ($input:expr) => {
        Some($input.iter().cloned().collect())
    };
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
        render_template!(
            actix_web::http::StatusCode::OK,
            $template_name,
            $context,
            $transaction
        )
    }};
    ($status:expr, $template_name:literal, $context:expr) => {{
        if cfg!(debug_assertions) {
            $context.try_insert("debug", &true)?;
        }

        let template = $crate::templates::render($template_name, &$context).await?;
        Ok(actix_web::HttpResponseBuilder::new($status).body(template))
    }};
    ($status:expr, $template_name:literal, $context:expr, $transaction:expr) => {{
        let domain = $crate::config::get_optional_setting::<String, _>("domain", &mut $transaction)
            .await?
            .unwrap_or_default();
        $context.try_insert("domain", &domain)?;

        if cfg!(debug_assertions) {
            $context.try_insert("debug", &true)?;
        }

        let template = $crate::templates::render($template_name, &$context).await?;

        $transaction.commit().await?;

        Ok(actix_web::HttpResponseBuilder::new($status).body(template))
    }};
}
