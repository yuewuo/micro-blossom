pub struct DefectsReader<'a> {
    pub defects: &'a [u32],
    pub cursor: usize,
}

impl<'a> DefectsReader<'a> {
    pub fn new(defects: &'a [u32]) -> Self {
        assert!(!defects.is_empty());
        assert_eq!(defects[defects.len() - 1], u32::MAX);
        Self { defects, cursor: 0 }
    }

    pub fn next(&mut self) -> Option<&[u32]> {
        if self.cursor >= self.defects.len() {
            return None;
        }
        let start = self.cursor;
        while self.defects[self.cursor] != u32::MAX {
            self.cursor += 1;
        }
        let end = self.cursor;
        self.cursor += 1;
        Some(&self.defects[start..end])
    }
}
