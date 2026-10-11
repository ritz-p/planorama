use crate::layout::{Bounds, Point};
use std::fmt::Write;

pub(super) struct Card {
    pub width: usize,
    pub height: usize,
    pub content: String,
}

pub(super) fn wrap(text: &str, limit: usize) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut width = 0;
    for (offset, character) in text.char_indices() {
        let advance = if character.is_ascii() { 1 } else { 2 };
        if width > 0 && width + advance > limit {
            lines.push(&text[start..offset]);
            start = offset;
            width = 0;
        }
        width += advance;
    }
    lines.push(&text[start..]);
    lines
}

pub(super) fn card_width<T: AsRef<str>>(texts: impl Iterator<Item = T>) -> usize {
    texts
        .map(|text| {
            text.as_ref()
                .chars()
                .map(|c| if c.is_ascii() { 8 } else { 16 })
                .sum::<usize>()
                + 32
        })
        .max()
        .unwrap_or(360)
        .clamp(360, 640)
}

pub(super) struct Area {
    cards: Vec<(Card, Bounds)>,
    pub width: usize,
    pub height: usize,
}

impl Area {
    pub fn new(cards: Vec<Card>, available_width: usize) -> Self {
        Self::flow(cards, available_width, Point { x: 40, y: 160 }, 40, 24)
    }

    pub fn flow(
        cards: Vec<Card>,
        available_width: usize,
        origin: Point,
        right: usize,
        bottom: usize,
    ) -> Self {
        let width = cards.iter().fold(available_width, |width, card| {
            width.max(card.width + origin.x + right)
        });
        let (mut x, mut y, mut row_height) = (origin.x, origin.y, 0);
        let mut placed = Vec::new();
        for card in cards {
            if x > origin.x && x + card.width > width - right {
                x = origin.x;
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
            y + row_height + bottom - origin.y
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
        svg.push_str(&self.content());
        svg.push_str("</g>\n");
        svg
    }

    pub fn content(&self) -> String {
        let mut svg = String::new();
        for (card, bounds) in &self.cards {
            writeln!(svg, r##"<g data-annotation-card="true" transform="translate({} {})"><rect width="{}" height="{}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>{}</g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height, card.content).unwrap();
        }
        svg
    }

    pub fn sections(
        sections: Vec<(&str, super::style::SectionPalette, Vec<Card>)>,
        available_width: usize,
    ) -> Self {
        let width = sections
            .iter()
            .flat_map(|(_, _, cards)| cards)
            .fold(available_width, |width, card| width.max(card.width + 112));
        let cards = sections.into_iter().filter_map(|(heading, palette, cards)| {
            if cards.is_empty() { return None; }
            let children = Self::flow(cards, width - 80, Point { x: 16, y: 48 }, 16, 16);
            let height = children.height + 48;
            let content = format!(r#"<g data-annotation-section="{heading}"><rect width="{}" height="{height}" rx="12" fill="{}" stroke="{}"/><text x="16" y="28" font-size="15" font-weight="700" fill="{}">{heading}</text>{}</g>"#, children.width, palette.background, palette.border, palette.heading, children.content());
            Some(Card { width: children.width, height, content })
        }).collect();
        Self::new(cards, width)
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/annotations.rs"]
mod tests;
