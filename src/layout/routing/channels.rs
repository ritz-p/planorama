#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/channels.rs"]
mod tests;

use super::coverage::Coverage;

struct HorizontalChannel {
    y: usize,
    occupied: Coverage,
}

pub(super) struct HorizontalChannels {
    channels: Vec<HorizontalChannel>,
}

impl HorizontalChannels {
    pub(super) fn new(ys: &[usize]) -> Self {
        Self {
            channels: ys
                .iter()
                .map(|&y| HorizontalChannel {
                    y,
                    occupied: Coverage::default(),
                })
                .collect(),
        }
    }

    #[cfg(test)]
    pub(super) fn allocate(
        &mut self,
        from: usize,
        to: usize,
        source_y: usize,
        target_y: usize,
    ) -> usize {
        self.allocate_candidates(&vec![(from, to); self.channels.len()], source_y, target_y)
    }

    pub(super) fn allocate_candidates(
        &mut self,
        ranges: &[(usize, usize)],
        source_y: usize,
        target_y: usize,
    ) -> usize {
        assert_eq!(ranges.len(), self.channels.len());
        let index = self
            .channels
            .iter()
            .enumerate()
            .min_by_key(|(index, channel)| {
                let (from, to) = ranges[*index];
                let overlap = channel.occupied.overlap(from, to);
                let distance =
                    source_y.abs_diff(channel.y) as u128 + target_y.abs_diff(channel.y) as u128;
                (overlap, distance, *index)
            })
            .map(|(index, _)| index)
            .expect("routing requires a horizontal channel");
        let channel = &mut self.channels[index];
        let (from, to) = ranges[index];
        channel.occupied.insert(from, to);
        channel.y
    }
}
