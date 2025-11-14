/// gRPC Protocol Buffer message definitions and service traits

pub mod tasks {
    use serde::{Deserialize, Serialize};

    pub mod task_service_server {
        use tonic::async_trait;

        #[async_trait]
        pub trait TaskService: Send + Sync + 'static {
            async fn create_task(
                &self,
                request: tonic::Request<super::CreateTaskRequest>,
            ) -> Result<tonic::Response<super::Task>, tonic::Status>;

            async fn get_task(
                &self,
                request: tonic::Request<super::GetTaskRequest>,
            ) -> Result<tonic::Response<super::Task>, tonic::Status>;

            async fn list_tasks(
                &self,
                request: tonic::Request<super::ListTasksRequest>,
            ) -> Result<tonic::Response<super::ListTasksResponse>, tonic::Status>;

            async fn update_task(
                &self,
                request: tonic::Request<super::UpdateTaskRequest>,
            ) -> Result<tonic::Response<super::Task>, tonic::Status>;

            async fn delete_task(
                &self,
                request: tonic::Request<super::DeleteTaskRequest>,
            ) -> Result<tonic::Response<super::DeleteTaskResponse>, tonic::Status>;

            type ExecuteTaskStream: futures::Stream<Item = Result<super::ExecutionEvent, tonic::Status>>
                + Send
                + 'static;

            async fn execute_task(
                &self,
                request: tonic::Request<super::ExecuteTaskRequest>,
            ) -> Result<tonic::Response<Self::ExecuteTaskStream>, tonic::Status>;
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CreateTaskRequest {
        pub title: String,
        pub description: String,
        pub task_type: String,
        pub config: Option<String>,
        pub metadata: Option<String>,
        pub workspace_path: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetTaskRequest {
        pub id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ListTasksRequest {
        pub limit: i32,
        pub offset: i32,
        pub status: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ListTasksResponse {
        pub tasks: Vec<Task>,
        pub total: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct UpdateTaskRequest {
        pub id: String,
        pub title: String,
        pub description: String,
        pub status: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DeleteTaskRequest {
        pub id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DeleteTaskResponse {
        pub success: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ExecuteTaskRequest {
        pub id: String,
        pub parameters: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Task {
        pub id: String,
        pub title: String,
        pub description: String,
        pub task_type: String,
        pub status: i32,
        pub config: Option<String>,
        pub metadata: Option<String>,
        pub workspace_path: String,
        pub created_at: String,
        pub updated_at: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ExecutionEvent {
        pub timestamp: String,
        pub event_type: String,
        pub message: String,
        pub status: String,
    }
}

pub mod workflows {
    use serde::{Deserialize, Serialize};

    pub mod workflow_service_server {
        use tonic::async_trait;

        #[async_trait]
        pub trait WorkflowService: Send + Sync + 'static {
            async fn create_workflow(
                &self,
                request: tonic::Request<super::CreateWorkflowRequest>,
            ) -> Result<tonic::Response<super::Workflow>, tonic::Status>;

            async fn get_workflow(
                &self,
                request: tonic::Request<super::GetWorkflowRequest>,
            ) -> Result<tonic::Response<super::Workflow>, tonic::Status>;

            async fn list_workflows(
                &self,
                request: tonic::Request<super::ListWorkflowsRequest>,
            ) -> Result<tonic::Response<super::ListWorkflowsResponse>, tonic::Status>;

            async fn update_workflow(
                &self,
                request: tonic::Request<super::UpdateWorkflowRequest>,
            ) -> Result<tonic::Response<super::Workflow>, tonic::Status>;

            async fn delete_workflow(
                &self,
                request: tonic::Request<super::DeleteWorkflowRequest>,
            ) -> Result<tonic::Response<super::DeleteWorkflowResponse>, tonic::Status>;

            type ExecuteWorkflowStream: futures::Stream<Item = Result<super::ExecutionEvent, tonic::Status>>
                + Send
                + 'static;

            async fn execute_workflow(
                &self,
                request: tonic::Request<super::ExecuteWorkflowRequest>,
            ) -> Result<tonic::Response<Self::ExecuteWorkflowStream>, tonic::Status>;
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct CreateWorkflowRequest {
        pub name: String,
        pub description: String,
        pub definition: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetWorkflowRequest {
        pub id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ListWorkflowsRequest {
        pub limit: i32,
        pub offset: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ListWorkflowsResponse {
        pub workflows: Vec<Workflow>,
        pub total: i32,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct UpdateWorkflowRequest {
        pub id: String,
        pub name: String,
        pub description: String,
        pub definition: String,
        pub status: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DeleteWorkflowRequest {
        pub id: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DeleteWorkflowResponse {
        pub success: bool,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ExecuteWorkflowRequest {
        pub id: String,
        pub parameters: Option<String>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Workflow {
        pub id: String,
        pub name: String,
        pub description: String,
        pub definition: String,
        pub status: String,
        pub created_at: String,
        pub updated_at: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ExecutionEvent {
        pub timestamp: String,
        pub event_type: String,
        pub message: String,
        pub status: String,
    }
}

pub mod auth {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AuthenticateRequest {
        pub username: String,
        pub password: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AuthenticateResponse {
        pub access_token: String,
        pub expires_in: i64,
        pub username: String,
    }
}

pub mod health {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct HealthCheckRequest {
        pub service: String,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct HealthCheckResponse {
        pub status: String,
        pub version: String,
        pub uptime_seconds: i64,
    }
}
