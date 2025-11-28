use serde::Deserialize;

use pushkind_common::pagination::Paginated;

use crate::domain::tag::Tag;

/// Query parameters accepted by the tags index page.
#[derive(Debug, Default, Deserialize)]
pub struct TagQuery {
    /// Optional case-insensitive search applied to tag names.
    pub search: Option<String>,
    /// Page number requested by the UI (1-based).
    pub page: Option<usize>,
}

/// Data required to render the tags index template.
pub struct TagsPageData {
    /// Paginated list of tags displayed in the table.
    pub tags: Paginated<Tag>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}
