use crate::runtime::Runtime;

use crate::completion::engine::WorkspaceProvider;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RuntimeWorkspaceProvider;

impl WorkspaceProvider for RuntimeWorkspaceProvider {
    fn list_workspaces(&self, runtime: Runtime) -> Result<Vec<String>, String> {
        crate::launcher::completion_workspace_names(runtime)
    }
}
