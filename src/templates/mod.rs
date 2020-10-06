use crate::templates::plain::Template;
use lazy_static::lazy_static;
use std::collections::HashMap;

pub(crate) mod plain;

lazy_static! {
    pub(crate) static ref VERIFY_EMAIL: Template = parse_template("email/user/verify_email.txt".to_owned());
}

fn parse_template(template_path: String) -> Template {
    match plain::parse(template_path) {
        Ok(template) => template,
        Err(err) => panic!("Failed to parse template: {}", err)
    }
}
