use crate::templates::plain::Template;

use std::path::Path;
use std::sync::RwLock;

use anyhow::Result;
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
    info!("Loading templates...");

    #[allow(unused_must_use)]
    {
        TERA.read().unwrap();
    }

    info!("Successfully loaded templates.");

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

        match TERA.write() {
            Ok(mut lock) => match lock.full_reload() {
                Ok(_) => info!("Successfully reloaded templates."),
                Err(err) => error!("Failed to reload templates: {}", err)
            }
            Err(err) => error!("Lock is poisoned: {}", err)
        }
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

#[macro_export]
macro_rules! render_template {
    ($template_name:literal, $context:expr) => {{
        render_template!(actix_web::http::StatusCode::OK, $template_name, $context)
    }};
    ($template_name:literal, $context:expr, $transaction:expr) => {{
        render_template!(actix_web::http::StatusCode::OK, $template_name, $context, $transaction)
    }};
    ($status:expr, $template_name:literal, $context:expr) => {{
        let domain: &str = $crate::CONFIG.domain.borrow();
        $context.try_insert("domain", &domain)?;

        let template = $crate::templates::TERA.read().unwrap().render($template_name, &$context)?;
        Ok(actix_web::dev::HttpResponseBuilder::new($status).body(template))
    }};
    ($status:expr, $template_name:literal, $context:expr, $transaction:expr) => {{
        let domain: &str = $crate::CONFIG.domain.borrow();
        $context.try_insert("domain", &domain)?;

        let template = $crate::templates::TERA.read().unwrap().render($template_name, &$context)?;

        $transaction.commit().await?;

        Ok(actix_web::dev::HttpResponseBuilder::new($status).body(template))
    }};
}
