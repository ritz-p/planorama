use crate::layout::{Bounds, Point};
use std::fmt::Write;

pub(super) struct Card {
    pub width: usize,
    pub height: usize,
    pub content: String,
}

pub(super) struct Area {
    cards: Vec<(Card, Bounds)>,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(cards: Vec<Card>, available_width: usize) -> Self {
        let width = cards
            .iter()
            .fold(available_width, |width, card| width.max(card.width + 80));
        let (mut x, mut y, mut row_height) = (40, 160, 0);
        let mut placed = Vec::new();
        for card in cards {
            if x > 40 && x + card.width > width - 40 {
                x = 40;
                y += row_height + 20;
                row_height = 0;
            }
            let bounds = Bounds {
                origin: Point { x, y },
                width: card.width,
                height: card.height,
            };
            x += card.width + 20;
            row_height = row_height.max(card.height);
            placed.push((card, bounds));
        }
        let height = if placed.is_empty() {
            0
        } else {
            y + row_height + 24 - 160
        };
        Self {
            cards: placed,
            width,
            height,
        }
    }

    pub fn render(&self) -> String {
        if self.cards.is_empty() {
            return String::new();
        }
        let mut svg = String::from("<g data-annotation-area=\"true\">\n");
        for (card, bounds) in &self.cards {
            writeln!(svg, r##"<g data-annotation-card="true" transform="translate({} {})"><rect width="{}" height="{}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>{}</g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, card.content).unwrap();
        }
        svg.push_str("</g>\n");
        svg
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/annotations.rs"]
mod tests;
