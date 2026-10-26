use domain::errors::DomainError;

use crate::images::deps::GetImageDeps;

pub async fn execute(deps: &GetImageDeps, key: &str) -> Result<Vec<u8>, DomainError> {
    deps.object_storage.get(key).await
}
