//! One conjunction rule for the compatibility and temporal interpreters.
use canon_core::Verdict;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionState {
    Proposed,
    Admitted,
    Refused,
}

pub(crate) fn resolve<'a>(
    requirements: impl IntoIterator<Item = &'a Verdict>,
    floor: i64,
) -> (AdmissionState, Option<i64>) {
    let (mut status, mut since) = (AdmissionState::Admitted, floor);
    for verdict in requirements {
        match verdict {
            Verdict::Refused { .. } => return (AdmissionState::Refused, None),
            Verdict::Proposed { .. } => status = AdmissionState::Proposed,
            Verdict::Ratified { since: at, .. } => since = since.max(*at),
        }
    }
    (
        status,
        (status == AdmissionState::Admitted).then_some(since),
    )
}
