use super::{Segment, SegmentData};
use crate::config::{InputData, PullRequestKind, SegmentId};
use crate::core::format::printable;

/// Host-provided session, worktree, and review identity; no filesystem or network queries.
pub enum IdentitySegment {
    SessionName,
    Worktree,
    PullRequest,
}

impl Segment for IdentitySegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let mut secondary = String::new();
        let primary = match self {
            Self::SessionName => printable(input.session_name.as_ref()?),
            Self::Worktree => printable(input.workspace.git_worktree.as_ref()?),
            Self::PullRequest => {
                let pr = input.pr.as_ref()?;
                let prefix = match pr.kind {
                    Some(PullRequestKind::MergeRequest) => "MR !",
                    None => "PR #",
                };
                if let Some(state) = &pr.review_state {
                    secondary = format!("· {}", state.label());
                }
                format!(
                    "\x1b]8;;{}\x1b\\{prefix}{}\x1b]8;;\x1b\\",
                    printable(&pr.url),
                    pr.number
                )
            }
        };
        Some(SegmentData {
            primary,
            secondary,
            metadata: Default::default(),
        })
    }

    fn id(&self) -> SegmentId {
        match self {
            Self::SessionName => SegmentId::SessionName,
            Self::Worktree => SegmentId::Worktree,
            Self::PullRequest => SegmentId::PullRequest,
        }
    }
}
