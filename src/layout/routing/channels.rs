#[cfg(test)]
#[path = "../../../tests/unit/layout/routing/channels.rs"]
mod tests;

struct OccupiedSegment {
    from: usize,
    to: usize,
}

struct HorizontalChannel {
    y: usize,
    occupied: Vec<OccupiedSegment>,
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
                    occupied: Vec::new(),
                })
                .collect(),
        }
    }

    pub(super) fn allocate(
        &mut self,
        from: usize,
        to: usize,
        source_y: usize,
        target_y: usize,
    ) -> usize {
        let segment = OccupiedSegment {
            from: from.min(to),
            to: from.max(to),
        };
        let index = self
            .channels
            .iter()
            .enumerate()
            .min_by_key(|(index, channel)| {
                let overlap: u128 = channel
                    .occupied
                    .iter()
                    .map(|existing| {
                        existing
                            .to
                            .min(segment.to)
                            .saturating_sub(existing.from.max(segment.from))
                            as u128
                    })
                    .sum();
                let distance =
                    source_y.abs_diff(channel.y) as u128 + target_y.abs_diff(channel.y) as u128;
                (overlap, distance, *index)
            })
            .map(|(index, _)| index)
            .expect("routing requires a horizontal channel");
        let channel = &mut self.channels[index];
        if segment.from != segment.to {
            channel.occupied.push(segment);
        }
        channel.y
    }
}
