//! Right-hand inspector: details of the selected file or folder and every
//! action on it (open, open with, rename, copy/cut, duplicate, trash, delete).
//! It never lists files: the one grid lives in the files pane, and the
//! actions here run on that pane's selection (see app.rs `Message::Inspector`).

use crate::filetype::Kind;
use crate::fsops::{self, Meta};
use crate::theme;
use crate::app::MenuAction;
use iced::widget::{button, column, row, scrollable, text, Space};
use iced::{Element, Length, Task};
use std::path::PathBuf;

#[derive(Default)]
pub struct Inspector {
    target: Option<PathBuf>,
    meta: Option<Result<Meta, String>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(PathBuf, Result<Meta, String>),
    /// an action button; the app runs it on the selection
    Action(MenuAction),
}

impl Inspector {
    pub fn target(&self) -> Option<&PathBuf> {
        self.target.as_ref()
    }

    pub fn focus(&mut self, target: Option<(PathBuf, Kind)>) -> Task<Msg> {
        self.meta = None;
        self.target = target.map(|(p, _)| p);
        match &self.target {
            Some(p) => {
                let p = p.clone();
                Task::perform(fsops::metadata(p.clone()), move |r| {
                    Msg::Loaded(p.clone(), r)
                })
            }
            None => Task::none(),
        }
    }

    pub fn update(&mut self, msg: Msg) {
        match msg {
            // drop stale results if focus moved on while loading
            Msg::Loaded(p, r) if self.target.as_ref() == Some(&p) => self.meta = Some(r),
            Msg::Loaded(..) | Msg::Action(_) => {}
        }
    }

    pub fn view(&self) -> Element<'_, Msg> {
        let field = |k: &'static str, v: String| -> Element<'_, Msg> {
            row![
                text(k).size(12).color(theme::MUTED).width(80),
                text(v).size(12)
            ]
            .spacing(6)
            .into()
        };
        let body: Element<'_, Msg> = match &self.meta {
            None if self.target.is_none() => text("Nothing selected").color(theme::MUTED).into(),
            None => text("...").into(),
            Some(Err(e)) => text(e.clone()).into(),
            Some(Ok(m)) => {
                let mut c = column![
                    text(
                        m.path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    )
                    .size(16),
                    field("Type", m.kind.label().into()),
                    field("Size", fsops::human_size(m.size)),
                    field("Modified", fsops::ago(m.modified)),
                ]
                .spacing(6);
                // unix permission bits; Windows only has the read-only flag
                if let Some(x) = m.mode {
                    c = c.push(field("Mode", format!("{:o}", x & 0o7777)));
                }
                c = c.push(field(
                    if m.mode.is_some() {
                        "Writable"
                    } else {
                        "Read-only"
                    },
                    match (m.mode.is_some(), m.readonly) {
                        (true, ro) => if ro { "no" } else { "yes" }.into(),
                        (false, ro) => if ro { "yes" } else { "no" }.into(),
                    },
                ));
                if let Some(n) = m.child_count {
                    c = c.push(field("Items", n.to_string()));
                }
                c = c.push(Space::with_height(8));
                c = c.push(actions(m.kind == Kind::Folder));
                c.into()
            }
        };
        column![
            text("Inspector").size(15),
            scrollable(body).height(Length::Fill)
        ]
        .spacing(10)
        .padding(12)
        .into()
    }
}

fn actions<'a>(folder: bool) -> Element<'a, Msg> {
    let b = |label: &'static str, a: MenuAction| {
        button(text(label).size(12))
            .on_press(Msg::Action(a))
            .style(button::secondary)
            .padding([3, 8])
    };
    let mut r = row![b("Open", MenuAction::Open)].spacing(4);
    if !folder {
        r = r.push(b("Open in default app", MenuAction::OpenDefault));
        r = r.push(b("Open with…", MenuAction::OpenWith));
    }
    for (l, a) in [
        ("Rename", MenuAction::Rename),
        ("Copy", MenuAction::Copy),
        ("Cut", MenuAction::Cut),
        ("Duplicate", MenuAction::Duplicate),
        ("Copy path", MenuAction::CopyPath),
        ("Move to Trash", MenuAction::Trash),
        ("Delete", MenuAction::Delete),
    ] {
        r = r.push(b(l, a));
    }
    row(vec![r.wrap().into()]).into()
}
