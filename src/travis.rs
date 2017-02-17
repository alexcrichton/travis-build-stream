use serde_json;

#[derive(Deserialize, Debug)]
pub struct GetBuilds {
    pub builds: Vec<Build>,
    pub commits: Vec<Commit>,
}

#[derive(Deserialize, Debug)]
pub struct Build {
    pub id: u32,
    pub number: String,
    pub state: String,
    pub commit_id: u32,
    pub job_ids: Vec<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Commit {
    pub id: u32,
    pub branch: String,
    pub sha: String,
    pub compare_url: String,
    pub committed_at: String,
}

#[derive(Deserialize, Debug)]
pub struct GetBuild {
    pub commit: Commit,
    pub build: Build,
    pub jobs: Vec<Job>,
}

#[derive(Deserialize, Debug)]
pub struct Job {
    pub id: u32,
    pub build_id: u32,
    pub allow_failure: bool,
    pub state: String,
    pub started_at: String,
    pub finished_at: String,
    pub config: serde_json::Value,
}
