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

fn flow_positions(width: usize, origin: Point, right: usize) -> impl FnMut(&Card) -> Bounds {
    let (mut x, mut y, mut row_height) = (origin.x, origin.y, 0);
    move |card| {
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
        bounds
    }
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
        let mut position = flow_positions(width, origin, right);
        let placed: Vec<_> = cards
            .into_iter()
            .map(|card| {
                let bounds = position(&card);
                (card, bounds)
            })
            .collect();
        let height = placed
            .iter()
            .map(|(_, bounds)| bounds.origin.y + bounds.height + bottom - origin.y)
            .max()
            .unwrap_or(0);
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
        let mut svg = format!(
            "<g data-annotation-area=\"true\" data-packing-width=\"{}\" data-measured-height=\"{}\">\n",
            self.packing_width, self.height
        );
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

    pub fn section_bounds(&self) -> impl Iterator<Item = Bounds> + '_ {
        self.sections.iter().map(|s| s.bounds)
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

    pub fn single_plan_width(
        sections: &[(&str, super::style::SectionPalette, Vec<Card>)],
        width: usize,
        fixed_height: usize,
    ) -> usize {
        let pair_width = sections
            .iter()
            .flat_map(|(_, _, cards)| cards.windows(2))
            .map(|pair| 112 + pair[0].width + 20 + pair[1].width)
            .max()
            .unwrap_or(0);
        if pair_width <= width {
            return width;
        }
        let height = |width| {
            fixed_height
                + sections
                    .iter()
                    .filter(|(_, _, cards)| !cards.is_empty())
                    .map(|(_, _, cards)| {
                        cards
                            .iter()
                            .map(flow_positions(width - 80, Point { x: 16, y: 48 }, 16))
                            .map(|bounds| bounds.origin.y + bounds.height)
                            .max()
                            .unwrap_or(0)
                            + 16
                            + 20
                    })
                    .sum::<usize>()
                + 4
        };
        let original = height(width);
        if original > width * 2
            && (height(pair_width) as u128) * (pair_width as u128)
                < (original as u128) * (width as u128)
        {
            pair_width
        } else {
            width
        }
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
            r#"<g data-annotation-section="{heading}" data-packing-width="{}"><rect width="{content_width}" height="{height}" rx="12" fill="{}" stroke="{}"/><text x="16" y="28" font-size="15" font-weight="700" fill="{}">{heading}</text>{}</g>"#,
            self.children.packing_width,
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
