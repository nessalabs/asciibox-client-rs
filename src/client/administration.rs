//! Remaining SDK operations share the parent client's transport and error policy.
use super::*;

impl BoxApi {
    /// TypeScript `addEnvironmentRepo`: `POST /environments/{environmentId}/repos`.
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

    /// TypeScript `apiKeyUsage`: `GET /api-keys/{apiKeyId}/usage`.
    pub async fn api_key_usage(&self, api_key_id: &str) -> Result<ApiKeyUsageResponse> {
        let api_path = format!("/api-keys/{}/usage", segment(api_key_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `apiKeys`: `GET /api-keys`.
    pub async fn api_keys(&self) -> Result<ApiKeysResponse> {
        let api_path = "/api-keys";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `artifact`: `GET /boxes/{boxId}/artifacts`.
    pub async fn artifact(&self, box_id: &str, path: &str) -> Result<Vec<u8>> {
        let api_path = format!("/boxes/{}/artifacts", validate_box_id(box_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        let req = req.header("Accept", "*/*");
        let req = req.query(&[("path", path)]);
        self.execute_bytes(req, true).await
    }

    /// TypeScript `createWebhook`: `POST /webhooks`.
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

    /// TypeScript `deleteEnvironmentRepo`: `DELETE /environments/{environmentId}/repos/{repositoryId}`.
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

    /// TypeScript `deleteEnvironmentSecretFile`: `DELETE /environments/{environmentId}/secret-files`.
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

    /// TypeScript `deleteEnvironmentVar`: `DELETE /environments/{environmentId}/vars/{key}`.
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

    /// TypeScript `deleteNamedSnapshot`: `DELETE /named-snapshots/{name}`.
    pub async fn delete_named_snapshot(&self, name: &str) -> Result<NamedSnapshotDeletedResponse> {
        let api_path = format!("/named-snapshots/{}", segment(name)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// TypeScript `deleteWebhook`: `DELETE /webhooks/{webhookId}`.
    pub async fn delete_webhook(&self, webhook_id: &str) -> Result<WebhookDeleteResponse> {
        let api_path = format!("/webhooks/{}", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::DELETE, &url);
        self.execute(req, false).await
    }

    /// TypeScript `fork`: `POST /boxes/{boxId}/fork`.
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

    /// TypeScript `getDataRetention`: `GET /account/data-retention`.
    pub async fn get_data_retention(&self) -> Result<DataRetentionPolicyResponse> {
        let api_path = "/account/data-retention";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `getNamedSnapshot`: `GET /named-snapshots/{name}`.
    pub async fn get_named_snapshot(&self, name: &str) -> Result<NamedSnapshotInfoResponse> {
        let api_path = format!("/named-snapshots/{}", segment(name)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `getWebhook`: `GET /webhooks/{webhookId}`.
    pub async fn get_webhook(&self, webhook_id: &str) -> Result<WebhookResponse> {
        let api_path = format!("/webhooks/{}", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `listNamedSnapshots`: `GET /named-snapshots`.
    pub async fn list_named_snapshots(&self) -> Result<NamedSnapshotListResponse> {
        let api_path = "/named-snapshots";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `listWebhooks`: `GET /webhooks`.
    pub async fn list_webhooks(&self) -> Result<WebhookListResponse> {
        let api_path = "/webhooks";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `repos`: `GET /repos`.
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

    /// TypeScript `rotateWebhookSigningSecret`: `POST /webhooks/{webhookId}/rotate`.
    pub async fn rotate_webhook_signing_secret(
        &self,
        webhook_id: &str,
    ) -> Result<WebhookSecretResponse> {
        let api_path = format!("/webhooks/{}/rotate", segment(webhook_id)?);
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        self.execute(req, false).await
    }

    /// TypeScript `saveNamedSnapshot`: `POST /named-snapshots`.
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

    /// TypeScript `secrets`: `GET /secrets`.
    pub async fn secrets(&self) -> Result<SecretsResponse> {
        let api_path = "/secrets";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::GET, &url);
        self.execute(req, true).await
    }

    /// TypeScript `selectRepo`: `POST /repos`.
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

    /// TypeScript `setEnvironmentSecretFile`: `PUT /environments/{environmentId}/secret-files`.
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

    /// TypeScript `setEnvironmentVar`: `PUT /environments/{environmentId}/vars/{key}`.
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

    /// TypeScript `updateDataRetention`: `PATCH /account/data-retention`.
    /// Enabling retention deletion requires the explicit confirmation from the caller.
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

    /// TypeScript `updateSecrets`: `POST /secrets`.
    pub async fn update_secrets(&self, request: SecretsUpdateRequest) -> Result<SecretsResponse> {
        let api_path = "/secrets";
        let url = format!("{}{api_path}", self.base);
        let req = self.base_request(Method::POST, &url);
        let req = req.json(&request);
        self.execute(req, false).await
    }

    /// TypeScript `updateWebhook`: `PATCH /webhooks/{webhookId}`.
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

    /// TypeScript `upgradeEnvironment`: `POST /environments/{environmentId}/upgrade`.
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
