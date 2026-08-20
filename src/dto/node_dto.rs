#[derive(Clone)]
pub struct NodeDto {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub node_type: String,
    pub parent_id: Option<i64>,
    pub key: Option<String>,
    pub parent: Option<ParentNodeDto>,
}

#[derive(Clone)]
pub struct ParentNodeDto {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub node_type: String,
    pub parent_id: Option<i64>,
    pub key: Option<String>,
}