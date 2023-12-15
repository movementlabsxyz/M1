use serde::{ Deserialize, Serialize };

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pagination {
  pub start: Option<usize>,
  pub limit: Option<usize>,
}