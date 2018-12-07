use crate::config::Config;
use crate::errors::RunError;
use crate::traces::{CoverageStat, TraceMap};
use coveralls_api::*;
use log::info;
use std::collections::HashMap;
use std::env;

fn get_git_info() -> Option<GitInfo> {
    if env::var_os("CIRCLECI").is_some() {
        let head = Head {
            id: env::var("CIRCLE_SHA1").unwrap_or(String::new()),
            author_name: env::var("CIRCLE_USERNAME").unwrap_or(String::new()),
            author_email: String::new(),
            committer_name: env::var("CIRCLE_USERNAME").unwrap_or(String::new()),
            committer_email: String::new(),
            message: String::new(),
        };
        Some(GitInfo {
            branch: env::var("CIRCLE_BRANCH").unwrap_or(String::new()),
            remotes: vec![],
            head: head,
        })
    }
    else if env::var_os("BUILDKITE").is_some() {
        let head = Head {
            id: env::var("BUILDKITE_COMMIT").unwrap_or(String::new()),
            author_name: env::var("BUILDKITE_BUILD_CREATOR").unwrap_or(String::new()),
            author_email: env::var("BUILDKITE_BUILD_CREATOR_EMAIL").unwrap_or(String::new()),
            committer_name: env::var("BUILDKITE_BUILD_CREATOR").unwrap_or(String::new()),
            committer_email: env::var("BUILDKITE_BUILD_CREATOR_EMAIL").unwrap_or(String::new()),
            message: env::var("BUILDKITE_MESSAGE").unwrap_or(String::new()),
        };
        Some(GitInfo {
            branch: env::var("BUILDKITE_BRANCH").unwrap_or(String::new()),
            remotes: vec![],
            head: head,
        })
    }
    else {
        None
    }
}

pub fn export(coverage_data: &TraceMap, config: &Config) -> Result<(), RunError> {
    if let Some(ref key) = config.coveralls {
        let id = match config.ci_tool {
            Some(ref service) => Identity::ServiceToken(Service {
                service_name: service.clone(),
                service_job_id: key.clone(),
            }),
            _ => Identity::RepoToken(key.clone()),
        };
        let mut report = CoverallsReport::new(id);
        if let Some(info) = get_git_info() {
            report.set_detailed_git_info(info);
        }
        for file in &coverage_data.files() {
            let rel_path = config.strip_project_path(file);
            let mut lines: HashMap<usize, usize> = HashMap::new();
            let fcov = coverage_data.get_child_traces(file);

            for c in &fcov {
                match c.stats {
                    CoverageStat::Line(hits) => {
                        lines.insert(c.line as usize, hits as usize);
                    }
                    _ => {
                        info!("Support for coverage statistic not implemented or supported for coveralls.io");
                    }
                }
            }
            if let Ok(source) = Source::new(&rel_path, file, &lines, &None, false) {
                report.add_source(source);
            }
        }

        let res = match config.report_uri {
            Some(ref uri) => {
                info!("Sending report to endpoint: {}", uri);
                report.send_to_endpoint(uri)
            }
            None => {
                info!("Sending coverage data to coveralls.io");
                report.send_to_coveralls()
            }
        };

        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(RunError::CovReport(format!("Coveralls send failed. {}", e))),
        }
    } else {
        Err(RunError::CovReport(
            "No coveralls key specified.".to_string(),
        ))
    }
}
