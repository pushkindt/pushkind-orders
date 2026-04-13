use crate::domain::category::CategoryTreeNode;

/// Hierarchical category data returned by the categories resource API.
pub struct CategoryTreeData {
    /// Hierarchical representation of the categories.
    pub tree: Vec<CategoryTreeNode>,
}
