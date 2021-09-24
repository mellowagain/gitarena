use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use git2::PackBuilderStage;
use tracing::instrument;

#[derive(Clone, Debug)]
pub(crate) struct ProgressWriter {
    lines: Vec<String>,
    pub(crate) delta_total: Option<u32>
}

impl ProgressWriter {
    pub(crate) fn new() -> ProgressWriter {
        ProgressWriter {
            lines: Vec::<String>::new(),
            delta_total: None
        }
    }

    pub(crate) fn write_text(&mut self, text: String) {
        self.lines.push(text);
    }

    #[instrument]
    pub(crate) fn pack_builder_callback(&mut self) -> impl FnMut(PackBuilderStage, u32, u32) -> bool + '_ {
        let rc = Rc::new(RefCell::new(self));

        move |stage: PackBuilderStage, current: u32, total: u32| -> bool {
            let total = if total != 0 { total } else { current }; // Prevent divide by 0 when calculating percentage below

            let ending = if current == total { ", done.\n" } else { "\r" };
            let percentage = current * 100 / total;

            match stage {
                PackBuilderStage::AddingObjects => {
                    let ref_cell = &mut rc.borrow_mut();
                    ref_cell.lines.push(format!("Counting objects: {:>3}% ({}/{}){}", percentage, current, total, ending));
                }
                PackBuilderStage::Deltafication => {
                    let ref_cell = &mut rc.borrow_mut();

                    if ref_cell.delta_total.is_none() {
                        ref_cell.delta_total = Some(total);
                    }

                    ref_cell.lines.push(format!("Compressing objects: {:>3}% ({}/{}){}", percentage, current, total, ending));
                }
            }

            true
        }
    }

    pub(crate) async fn to_writer(&self) -> Result<GitWriter> {
        let mut writer = GitWriter::new();

        for line in &self.lines {
            writer.write_binary_sideband(Band::Progress, line.as_bytes()).await?;
        }

        Ok(writer)
    }
}
