//! Administration and resource operations using the shared HTTP transport.
use super::*;

impl BoxApi {
    /// Add a repository to an environment.
    ///
    /// `POST /environments/{environmentId}/repos`.
    pub async fn add_environment_repo(
        &self,
        environment_id: &str,
        request: AddEnvironmentRepoRequest,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!("/environments/{}/repos", segment(environment_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Read usage and resource totals for an API key.
    ///
    /// `GET /api-keys/{apiKeyId}/usage`.
    pub async fn api_key_usage(&self, api_key_id: &str) -> Result<ApiKeyUsageResponse> {
        let api_path = format!("/api-keys/{}/usage", segment(api_key_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// List API keys and their usage summaries.
    ///
    /// `GET /api-keys`.
    pub async fn api_keys(&self) -> Result<ApiKeysResponse> {
        let api_path = "/api-keys";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// Download an artifact from a Box as bytes.
    ///
    /// `GET /boxes/{boxId}/artifacts`.
    pub async fn artifact(&self, box_id: &str, path: &str) -> Result<Vec<u8>> {
        let api_path = format!("/boxes/{}/artifacts", validate_box_id(box_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        let req = req.header("Accept", "*/*");
        let req = req.query(&[("path", path)]);
        self.execute_bytes(req, true).await
    }

    /// Create a webhook and return its signing secret.
    ///
    /// `POST /webhooks`.
    pub async fn create_webhook(
        &self,
        request: WebhookCreateRequest,
    ) -> Result<WebhookSecretResponse> {
        let api_path = "/webhooks";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Remove a repository from an environment.
    ///
    /// `DELETE /environments/{environmentId}/repos/{repositoryId}`.
    pub async fn delete_environment_repo(
        &self,
        environment_id: &str,
        repository_id: &str,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!(
            "/environments/{}/repos/{}",
            segment(environment_id)?,
            segment(repository_id)?
        );
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// Remove a secret file from an environment.
    ///
    /// `DELETE /environments/{environmentId}/secret-files`.
    pub async fn delete_environment_secret_file(
        &self,
        environment_id: &str,
        path: &str,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!("/environments/{}/secret-files", segment(environment_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        let req = req.query(&[("path", path)]);
        self.execute(req, false).await
    }

    /// Remove an environment variable.
    ///
    /// `DELETE /environments/{environmentId}/vars/{key}`.
    pub async fn delete_environment_var(
        &self,
        environment_id: &str,
        key: &str,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!(
            "/environments/{}/vars/{}",
            segment(environment_id)?,
            segment(key)?
        );
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// Delete a named snapshot.
    ///
    /// `DELETE /named-snapshots/{name}`.
    pub async fn delete_named_snapshot(&self, name: &str) -> Result<NamedSnapshotDeletedResponse> {
        let api_path = format!("/named-snapshots/{}", segment(name)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// Delete a webhook.
    ///
    /// `DELETE /webhooks/{webhookId}`.
    pub async fn delete_webhook(&self, webhook_id: &str) -> Result<WebhookDeleteResponse> {
        let api_path = format!("/webhooks/{}", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// Create an independent Box from an existing Box.
    ///
    /// `POST /boxes/{boxId}/fork`.
    pub async fn fork(
        &self,
        box_id: &str,
        request: Option<ForkRequest>,
        idempotency_key: Option<&str>,
    ) -> Result<BoxActionResponse> {
        let api_path = format!("/boxes/{}/fork", validate_box_id(box_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = json_body(req, request.as_ref());
        let req = match idempotency_key {
            Some(key) => req.header("Idempotency-Key", key),
            None => req,
        };
        self.execute(req, false).await
    }

    /// Read the account data-retention policy.
    ///
    /// `GET /account/data-retention`.
    pub async fn get_data_retention(&self) -> Result<DataRetentionPolicyResponse> {
        let api_path = "/account/data-retention";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// Read a named snapshot.
    ///
    /// `GET /named-snapshots/{name}`.
    pub async fn get_named_snapshot(&self, name: &str) -> Result<NamedSnapshotInfoResponse> {
        let api_path = format!("/named-snapshots/{}", segment(name)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// Read webhook settings.
    ///
    /// `GET /webhooks/{webhookId}`.
    pub async fn get_webhook(&self, webhook_id: &str) -> Result<WebhookResponse> {
        let api_path = format!("/webhooks/{}", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// List named snapshots.
    ///
    /// `GET /named-snapshots`.
    pub async fn list_named_snapshots(&self) -> Result<NamedSnapshotListResponse> {
        let api_path = "/named-snapshots";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// List webhooks.
    ///
    /// `GET /webhooks`.
    pub async fn list_webhooks(&self) -> Result<WebhookListResponse> {
        let api_path = "/webhooks";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// List available repositories with optional filters and pagination.
    ///
    /// `GET /repos`.
    pub async fn repos(&self, query: Option<&ReposQuery>) -> Result<ReposResponse> {
        let api_path = "/repos";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        let req = match query {
            Some(q) => req.query(q),
            None => req,
        };
        self.execute(req, true).await
    }

    /// Rotate and return a webhook signing secret.
    ///
    /// `POST /webhooks/{webhookId}/rotate`.
    pub async fn rotate_webhook_signing_secret(
        &self,
        webhook_id: &str,
    ) -> Result<WebhookSecretResponse> {
        let api_path = format!("/webhooks/{}/rotate", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        self.execute(req, false).await
    }

    /// Save a named snapshot from a Box.
    ///
    /// `POST /named-snapshots`.
    pub async fn save_named_snapshot(
        &self,
        request: NamedSnapshotSaveRequest,
    ) -> Result<NamedSnapshotSavingResponse> {
        let api_path = "/named-snapshots";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Read the configured secrets and secret files.
    ///
    /// `GET /secrets`.
    pub async fn secrets(&self) -> Result<SecretsResponse> {
        let api_path = "/secrets";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// Select a repository for new Boxes.
    ///
    /// `POST /repos`.
    pub async fn select_repo(
        &self,
        request: RepoSelectionRequest,
    ) -> Result<RepoSelectionResponse> {
        let api_path = "/repos";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Create or replace an environment secret file.
    ///
    /// `PUT /environments/{environmentId}/secret-files`.
    pub async fn set_environment_secret_file(
        &self,
        environment_id: &str,
        request: SetEnvironmentSecretFileRequest,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!("/environments/{}/secret-files", segment(environment_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::PUT, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Create or replace an environment variable.
    ///
    /// `PUT /environments/{environmentId}/vars/{key}`.
    pub async fn set_environment_var(
        &self,
        environment_id: &str,
        key: &str,
        request: SetEnvironmentVarRequest,
    ) -> Result<EnvironmentItemChangeResponse> {
        let api_path = format!(
            "/environments/{}/vars/{}",
            segment(environment_id)?,
            segment(key)?
        );
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::PUT, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Update the account data-retention policy.
    ///
    /// `PATCH /account/data-retention`.
    /// Requires an interactive session token. Enabling deletion also requires
    /// the confirmation phrase in the request.
    pub async fn update_data_retention(
        &self,
        request: DataRetentionUpdateRequest,
    ) -> Result<DataRetentionPolicyResponse> {
        let api_path = "/account/data-retention";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::PATCH, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Replace the configured secrets and secret files.
    /// Include every value that should remain in the configuration.
    ///
    /// `POST /secrets`.
    pub async fn update_secrets(&self, request: SecretsUpdateRequest) -> Result<SecretsResponse> {
        let api_path = "/secrets";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Update webhook settings.
    ///
    /// `PATCH /webhooks/{webhookId}`.
    pub async fn update_webhook(
        &self,
        webhook_id: &str,
        request: WebhookUpdateRequest,
    ) -> Result<WebhookResponse> {
        let api_path = format!("/webhooks/{}", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::PATCH, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// Upgrade an environment to a new version.
    ///
    /// `POST /environments/{environmentId}/upgrade`.
    pub async fn upgrade_environment(
        &self,
        environment_id: &str,
        request: Option<UpgradeBoxEnvironmentRequest>,
    ) -> Result<UpgradeBoxEnvironmentResponse> {
        let api_path = format!("/environments/{}/upgrade", segment(environment_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = json_body(req, request.as_ref());
        self.execute(req, false).await
    }
}
