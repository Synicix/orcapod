use std::error::Error;

use crate::model::{Annotation, Pod};

pub trait Store {
    fn store_annotation(
        &self,
        annotation: &Annotation,
        owner_hash: &str,
    ) -> Result<(), Box<dyn Error>>;

    fn load_annotation(&self, hash: &str) -> Result<Annotation, Box<dyn Error>>;

    fn store_pod(&self, pod: &Pod) -> Result<(), Box<dyn Error>>;

    fn load_pod(&self, hash: &str) -> Result<Pod, Box<dyn Error>>;
}
