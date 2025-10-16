use chrono::NaiveDateTime;
use pushkind_common::pagination::Pagination;
use serde::{Deserialize, Serialize};

/// Domain representation of a hierarchical product category belonging to a hub.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    /// Unique identifier of the category.
    pub id: i32,
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional identifier of the parent category when building a tree.
    pub parent_id: Option<i32>,
    /// Human-readable name of the category.
    pub name: String,
    /// Optional description that expands upon the category name.
    pub description: Option<String>,
    /// Flag indicating whether the category has been archived.
    pub is_archived: bool,
    /// Timestamp for when the category record was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the last update to the category record.
    pub updated_at: NaiveDateTime,
}

/// Payload required to insert a new category for a hub.
#[derive(Debug, Clone)]
pub struct NewCategory {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Optional identifier of the parent category when building a tree.
    pub parent_id: Option<i32>,
    /// Human-readable name of the category.
    pub name: String,
    /// Optional description that expands upon the category name.
    pub description: Option<String>,
    /// Timestamp captured when the category payload was created.
    pub updated_at: NaiveDateTime,
}

impl NewCategory {
    /// Build a new category payload with the supplied details and current timestamp.
    pub fn new(hub_id: i32, name: impl Into<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            parent_id: None,
            name: name.into(),
            description: None,
            updated_at: now,
        }
    }

    /// Attach a parent identifier to the category payload.
    pub fn with_parent_id(mut self, parent_id: i32) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Attach a descriptive text to the category payload.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Query definition used to retrieve the full category tree for a hub.
#[derive(Debug, Clone)]
pub struct CategoryTreeQuery {
    /// Owning hub identifier.
    pub hub_id: i32,
    /// Whether archived categories should be included in the results.
    pub include_archived: bool,
    /// Optional pagination options applied when retrieving a flattened list.
    pub pagination: Option<Pagination>,
}

impl CategoryTreeQuery {
    /// Construct a query that targets the category tree belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            include_archived: false,
            pagination: None,
        }
    }

    /// Include archived categories in the results.
    pub fn include_archived(mut self) -> Self {
        self.include_archived = true;
        self
    }

    /// Apply pagination to the query when the repository returns a flattened list.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

/// Node representation of a category and its children for tree traversal.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryTreeNode {
    /// Category data represented by this node.
    pub category: Category,
    /// Children that belong to this node.
    pub children: Vec<CategoryTreeNode>,
}

impl CategoryTreeNode {
    /// Create a new category tree node with no children.
    pub fn new(category: Category) -> Self {
        Self {
            category,
            children: Vec::new(),
        }
    }

    /// Attach a collection of children to the node.
    pub fn with_children(mut self, children: impl Into<Vec<CategoryTreeNode>>) -> Self {
        self.children = children.into();
        self
    }
}
