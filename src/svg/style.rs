use crate::model::Action;

pub(super) fn label(action: Action) -> &'static str {
    match action {
        Action::Unchanged => "unchanged",
        Action::Create => "create",
        Action::Update => "update",
        Action::Delete => "delete",
        Action::Replace => "replace",
        Action::Read => "read",
        Action::Other => "other",
    }
}

pub(super) fn color(action: Action) -> (&'static str, &'static str) {
    match action {
        Action::Create => ("#ecfdf5", "#047857"),
        Action::Update => ("#eff6ff", "#1d4ed8"),
        Action::Delete => ("#fef2f2", "#b91c1c"),
        Action::Replace => ("#fff7ed", "#c2410c"),
        Action::Read => ("#faf5ff", "#7e22ce"),
        Action::Unchanged => ("#f8fafc", "#64748b"),
        Action::Other => ("#fefce8", "#854d0e"),
    }
}

pub(super) fn shorten(text: &str, length: usize) -> String {
    match text.chars().count() {
        count if count <= length => text.into(),
        _ => format!("{}…", text.chars().take(length - 1).collect::<String>()),
    }
}
