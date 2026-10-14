use crate::layout::{Bounds, Point};
use std::fmt::Write;

pub(super) struct Card {
    pub width: usize,
    pub height: usize,
    pub content: String,
}

pub(super) use crate::view::text::wrap;

pub(super) fn card_width<T: AsRef<str>>(texts: impl Iterator<Item = T>) -> usize {
    texts
        .map(|text| crate::view::text::width(text.as_ref()) * 8 + 32)
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
        let mut y = 160;
        let mut cards = Vec::new();
        for (heading, palette, children) in sections {
            if children.is_empty() {
                continue;
            }
            let children = Self::flow(children, width - 80, Point { x: 16, y: 48 }, 16, 16);
            let content_width = children
                .cards
                .iter()
                .map(|(_, bounds)| bounds.right() + 16)
                .max()
                .unwrap_or(0)
                .max(crate::view::text::width(heading) * 9 + 32);
            let height = children.height + 48;
            let content = format!(
                r#"<g data-annotation-section="{heading}"><rect width="{content_width}" height="{height}" rx="12" fill="{}" stroke="{}"/><text x="16" y="28" font-size="15" font-weight="700" fill="{}">{heading}</text>{}</g>"#,
                palette.background,
                palette.border,
                palette.heading,
                children.content()
            );
            let bounds = Bounds {
                origin: Point { x: 40, y },
                width: content_width,
                height,
            };
            y += height + 20;
            cards.push((
                Card {
                    width: content_width,
                    height,
                    content,
                },
                bounds,
            ));
        }
        Self {
            height: if cards.is_empty() {
                0
            } else {
                y - 160 - 20 + 24
            },
            width,
            cards,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/annotations.rs"]
mod tests;
