use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Embedding {
    pub vector: Vec<f32>,
}
