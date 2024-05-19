use crate::util::*;

#[cfg_attr(any(test, feature = "std"), derive(Debug))]
pub struct LayerFusionData<const VN: usize> {
    /// const information about the fusion id; only vertices with layer id will be recorded as pending breaks
    pub vertex_layer_id: [OptionCompactLayerId; VN],
    /// pending breaks will eventually be empty when the last round is fused
    pub count_pending_breaks: usize,
    pub pending_breaks: [CompactNodeIndex; VN],
}

impl<const VN: usize> LayerFusionData<VN> {
    pub const fn new() -> Self {
        Self {
            vertex_layer_id: [OptionCompactLayerId::NONE; VN],
            count_pending_breaks: 0,
            pending_breaks: [CompactNodeIndex::new(0).unwrap(); VN],
        }
    }

    pub fn get_layer_id(&self, vertex_index: CompactVertexIndex) -> OptionCompactLayerId {
        self.vertex_layer_id[vertex_index.get() as usize]
    }

    /// record a matching with some vertex that is currently virtual but will be fused later;
    /// when `fuse_layer` is called, we should break such matchings becausse they are no longer valid
    pub fn append_break(&mut self, node_index: CompactNodeIndex) {
        debug_assert!(self.count_pending_breaks < VN);
        self.pending_breaks[self.count_pending_breaks] = node_index;
        self.count_pending_breaks += 1;
    }

    /// iterate the pending breaks and remove the breaks that returns true
    pub fn iterate_pending_breaks(&mut self, mut func: impl FnMut(CompactNodeIndex) -> bool) {
        let mut new_length = 0;
        for index in 0..self.count_pending_breaks {
            let remove = func(self.pending_breaks[index]);
            if !remove {
                self.pending_breaks[new_length] = self.pending_breaks[index];
                new_length += 1;
            }
        }
        self.count_pending_breaks = new_length;
    }
}

// TODO: test `iterate_pending_breaks`
