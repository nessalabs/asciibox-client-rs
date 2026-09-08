//! Models for the published SDK 0.0.34 lifecycle operations.
use super::*;

#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ForkRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_env: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub ttl_seconds: Option<Option<u32>>,
}
impl fmt::Debug for ForkRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForkRequest").finish_non_exhaustive()
    }
}
