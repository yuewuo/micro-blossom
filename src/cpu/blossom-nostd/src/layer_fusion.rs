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
    pub fn iterate_pending_breaks(&mut self, mut func: impl FnMut(&Self, CompactNodeIndex) -> bool) {
        let mut new_length = 0;
        for index in 0..self.count_pending_breaks {
            let remove = func(self, self.pending_breaks[index]);
            if !remove {
                self.pending_breaks[new_length] = self.pending_breaks[index];
                new_length += 1;
            }
        }
        self.count_pending_breaks = new_length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_fusion_size() {
        // cargo test layer_fusion_size -- --nocapture
        // cargo test --features u8_layer_id layer_fusion_size -- --nocapture
        const N: usize = 100000;
        let total_size = core::mem::size_of::<LayerFusionData<N>>();
        println!("memory: {} bytes per node", total_size / N);
        println!("memory overhead: {} bytes", total_size - (total_size / N) * N);
        cfg_if::cfg_if! {
            if #[cfg(feature="u8_layer_id")] {
                assert_eq!(total_size / N, 4 + 1);
            } else {
                assert_eq!(total_size / N, 4 + 4);
            }
        }
    }

    #[test]
    fn layer_fusion_iterate_pending_breaks() {
        // cargo test layer_fusion_iterate_pending_breaks -- --nocapture
        const N: usize = 100;
        let mut layer_fusion: LayerFusionData<N> = LayerFusionData::new();
        layer_fusion.append_break(ni!(100));
        layer_fusion.append_break(ni!(200));
        layer_fusion.append_break(ni!(300));
        layer_fusion.append_break(ni!(400));
        // verify state
        let check_state = |layer_fusion: &mut LayerFusionData<N>, expected: Vec<usize>| {
            assert_eq!(layer_fusion.count_pending_breaks, expected.len());
            for (index, value) in expected.iter().enumerate() {
                assert_eq!(layer_fusion.pending_breaks[index].get() as usize, *value);
            }
            // also check using iterate function
            let mut index = 0;
            layer_fusion.iterate_pending_breaks(|_, node_index| -> bool {
                assert_eq!(node_index.get() as usize, expected[index]);
                index += 1;
                false
            })
        };
        check_state(&mut layer_fusion, vec![100, 200, 300, 400]);
        // first remove 300
        layer_fusion.iterate_pending_breaks(|_, node_index| -> bool { node_index == ni!(300) });
        check_state(&mut layer_fusion, vec![100, 200, 400]);
        // then remove 400 and 100
        layer_fusion.iterate_pending_breaks(|_, node_index| -> bool { node_index == ni!(400) || node_index == ni!(100) });
        check_state(&mut layer_fusion, vec![200]);
        // remove nothing
        layer_fusion.iterate_pending_breaks(|_, _node_index| -> bool { false });
        check_state(&mut layer_fusion, vec![200]);
        // remove 200
        layer_fusion.iterate_pending_breaks(|_, node_index| -> bool { node_index == ni!(200) });
        check_state(&mut layer_fusion, vec![]);
    }
}
