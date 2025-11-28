use crate::domain::category::CategoryTreeNode;

/// Data required to render the categories index template.
pub struct CategoryTreeData {
    /// Hierarchical representation of the categories.
    pub tree: Vec<CategoryTreeNode>,
}
