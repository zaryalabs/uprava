//! Pure Core domain identity rules.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementIdentity {
    node_id: String,
    workspace_path: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlacementIdentityError {
    #[error("node identity is required")]
    MissingNode,
    #[error("workspace path is required")]
    MissingWorkspace,
}

impl PlacementIdentity {
    pub fn try_new(node_id: &str, workspace_path: &str) -> Result<Self, PlacementIdentityError> {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            return Err(PlacementIdentityError::MissingNode);
        }
        let workspace_path = workspace_path.trim();
        if workspace_path.is_empty() {
            return Err(PlacementIdentityError::MissingWorkspace);
        }
        Ok(Self {
            node_id: node_id.to_owned(),
            workspace_path: workspace_path.to_owned(),
        })
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn workspace_path(&self) -> &str {
        &self.workspace_path
    }
}

#[cfg(test)]
mod tests {
    use super::{PlacementIdentity, PlacementIdentityError};

    #[test]
    fn placement_identity_normalizes_owned_values() {
        let identity = PlacementIdentity::try_new(" node-1 ", " /work ").expect("identity");
        assert_eq!(identity.node_id(), "node-1");
        assert_eq!(identity.workspace_path(), "/work");
    }

    #[test]
    fn placement_identity_rejects_missing_values() {
        assert_eq!(
            PlacementIdentity::try_new("", "/work").expect_err("node required"),
            PlacementIdentityError::MissingNode
        );
        assert_eq!(
            PlacementIdentity::try_new("node-1", " ").expect_err("workspace required"),
            PlacementIdentityError::MissingWorkspace
        );
    }
}
