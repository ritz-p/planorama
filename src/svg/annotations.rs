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

pub(super) struct MeasuredAnnotationSection {
    heading: String,
    palette: super::style::SectionPalette,
    bounds: Bounds,
    children: Area,
}

pub(super) struct Area {
    cards: Vec<(Card, Bounds)>,
    sections: Vec<MeasuredAnnotationSection>,
    pub packing_width: usize,
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
            sections: Vec::new(),
            packing_width: width,
            height,
        }
    }

    pub fn render(&self) -> String {
        if self.cards.is_empty() && self.sections.is_empty() {
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
        for section in &self.sections {
            let bounds = &section.bounds;
            let content = section.render();
            writeln!(svg, r##"<g data-annotation-card="true" transform="translate({} {})"><rect width="{}" height="{}" rx="12" fill="#f8fafc" stroke="#cbd5e1"/>{content}</g>"##, bounds.origin.x, bounds.origin.y, bounds.width, bounds.height).unwrap();
        }
        svg
    }

    pub fn required_width(
        sections: &[(&str, super::style::SectionPalette, Vec<Card>)],
        available_width: usize,
    ) -> usize {
        sections
            .iter()
            .flat_map(|(_, _, cards)| cards)
            .fold(available_width, |width, card| width.max(card.width + 112))
    }

    pub fn sections(
        sections: Vec<(&str, super::style::SectionPalette, Vec<Card>)>,
        available_width: usize,
    ) -> Self {
        let width = Self::required_width(&sections, available_width);
        let mut y = 160;
        let mut measured = Vec::new();
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
            let bounds = Bounds {
                origin: Point { x: 40, y },
                width: content_width,
                height,
            };
            y += height + 20;
            measured.push(MeasuredAnnotationSection {
                heading: heading.into(),
                palette,
                bounds,
                children,
            });
        }
        Self {
            height: if measured.is_empty() {
                0
            } else {
                y - 160 - 20 + 24
            },
            packing_width: width,
            cards: Vec::new(),
            sections: measured,
        }
    }
}

impl MeasuredAnnotationSection {
    fn render(&self) -> String {
        let heading = &self.heading;
        let content_width = self.bounds.width;
        let height = self.bounds.height;
        format!(
            r#"<g data-annotation-section="{heading}"><rect width="{content_width}" height="{height}" rx="12" fill="{}" stroke="{}"/><text x="16" y="28" font-size="15" font-weight="700" fill="{}">{heading}</text>{}</g>"#,
            self.palette.background,
            self.palette.border,
            self.palette.heading,
            self.children.content()
        )
    }
}

#[cfg(test)]
#[path = "../../tests/unit/svg/annotations.rs"]
mod tests;
