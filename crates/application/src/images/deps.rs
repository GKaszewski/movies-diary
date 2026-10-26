use std::sync::Arc;

use domain::ports::ObjectStorage;

pub struct GetImageDeps {
    pub object_storage: Arc<dyn ObjectStorage>,
}
