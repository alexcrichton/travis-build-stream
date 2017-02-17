#[derive(Deserialize, Debug)]
pub struct History {
    pub project: Project,
    pub builds: Vec<Build>,
}

#[derive(Deserialize, Debug)]
pub struct Project {
    #[serde(rename = "projectId")]
    pub project_id: u32,
    #[serde(rename = "accountId")]
    pub account_id: u32,
    #[serde(rename = "accountName")]
    pub account_name: String,
    pub name: String,
    pub slug: String,
    #[serde(rename = "repositoryName")]
    pub repository_name: String,
    #[serde(rename = "repositoryType")]
    pub repository_type: String,
}

#[derive(Deserialize, Debug)]
pub struct Build {
    #[serde(rename = "buildId")]
    pub build_id: u32,
    pub jobs: Vec<Job>,
    #[serde(rename = "buildNumber")]
    pub build_number: u32,
    pub version: String,
    pub message: String,
    pub branch: String,
    #[serde(rename = "commitId")]
    pub commit_id: String,
    pub status: String,
    pub started: Option<String>,
    pub finished: Option<String>,
    pub created: String,
    pub updated: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Job {
    #[serde(rename = "jobId")]
    pub job_id: String,
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct LastBuild {
    pub build: Build,
}
