use serde_json;

#[derive(Deserialize, Debug)]
pub struct GetBuilds {
    pub builds: Vec<Build>,
    pub commits: Vec<Commit>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Build {
    pub id: u32,
    pub number: String,
    pub state: String,
    pub commit_id: u32,
    pub repository_id: u32,
    pub job_ids: Vec<u32>,
    pub pull_request: Option<bool>,
    pub pull_request_number: Option<u32>,
    pub started_at: String,
    pub finished_at: String,
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

#[derive(Deserialize, Debug)]
pub struct GetUser {
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub avatar_url: String,
    pub channels: Vec<String>,
    pub id: u32,
    pub name: String,
    pub login: String,
    pub email: String,
}

#[derive(Deserialize, Debug)]
pub struct GetRepo {
    pub repo: Repo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Repo {
    pub id: u32,
    pub slug: String,
}
