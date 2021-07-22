use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(crate) type Template = (String, HashMap<String, String>);
pub(crate) type TemplateContext = HashMap<String, String>;

pub(crate) fn parse(template_path: String) -> Result<Template> {
    let template_dir = Path::new("templates/");
    let path = template_dir.join(&template_path);

    let content = fs::read_to_string(path)?;
    let mut skip_lines = 0;

    let mut tags = HashMap::new();
    tags.insert("template_name".to_owned(), template_path.clone());

    for (index, line) in content.lines().enumerate() {
        if line == "---" {
            skip_lines = index + 2;
            break;
        }

        let mut splitter = line.splitn(2, ": ");
        let key = splitter.next().unwrap_or_default();
        let value = splitter.next().unwrap_or_default();

        if key.is_empty() || value.is_empty() {
            return Err(anyhow!("Template `{}` meta data contains empty values", template_path));
        }

        tags.insert(key.to_owned(), value.to_owned());
    }

    let vec: Vec<&str> = content.lines().skip(skip_lines).collect();

    Ok((vec.join("\n"), tags))
}

pub(crate) fn render(template_content: String, context_option: Option<TemplateContext>) -> String {
    let mut result = template_content;

    if let Some(context) = context_option {
        for (key, value) in context {
            result = result.replace(format!("{{{{{}}}}}", key).as_str(), value.as_str());
        }
    }

    result
}
