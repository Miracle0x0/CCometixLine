use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use crate::updater::UpdateState;

pub struct UpdateSegment {
    state_file: std::path::PathBuf,
}

impl UpdateSegment {
    pub fn new(state_file: std::path::PathBuf) -> Self {
        Self { state_file }
    }
}

impl Segment for UpdateSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        // Load update state and check for update status
        let update_state = UpdateState::load(&self.state_file);

        update_state.status_text().map(|status_text| SegmentData {
            primary: status_text,
            secondary: String::new(),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Update
    }
}
