#[derive(Deserialize, Debug)]
pub struct Commit {
    pub url: String,
    pub sha: String,
    pub parents: Vec<CommitParent>,
}

#[derive(Deserialize, Debug)]
pub struct CommitParent {
    pub url: String,
    pub sha: String,
}
