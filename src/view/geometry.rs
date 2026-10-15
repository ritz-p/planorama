use crate::model::EdgeKind;

macro_rules! stroke {
    ($width:ident, $style:ident, $value:literal, $extra:literal) => {
        pub(crate) const $width: usize = ($value as f64 * 4.0) as usize;
        const $style: &str = concat!("stroke-width=\"", stringify!($value), "\"", $extra);
    };
}

stroke!(DEPENDENCY_STROKE, DEPENDENCY_STYLE, 1.5, "");
stroke!(
    ASSOCIATION_STROKE,
    ASSOCIATION_STYLE,
    2,
    " stroke-dasharray=\"7 4\""
);
stroke!(
    CONNECTION_STROKE,
    CONNECTION_STYLE,
    2.5,
    " stroke-dasharray=\"0 3.5\" stroke-linecap=\"round\""
);

pub(crate) const CARD_STROKE: usize = DEPENDENCY_STROKE;
pub(crate) const CONTAINER_STROKE: usize = ASSOCIATION_STROKE;
pub(crate) const ROUTE_STROKE: usize = 4;
pub(crate) const CARD_RADIUS: usize = 8;
pub(crate) const CONTAINER_RADIUS: usize = 12;
pub(crate) const ROUTE_RADIUS: usize = 6;
pub(crate) const MASK_PADDING: usize = 2;
pub(crate) const RELATIONSHIP_RADIUS: f64 = 8.0;

pub(crate) fn relationship_stroke(kind: EdgeKind) -> usize {
    match kind {
        EdgeKind::Dependency | EdgeKind::Containment => DEPENDENCY_STROKE,
        EdgeKind::Association | EdgeKind::Subscription => ASSOCIATION_STROKE,
        EdgeKind::Connection => CONNECTION_STROKE,
    }
}

pub(crate) fn relationship_style(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Dependency | EdgeKind::Containment => DEPENDENCY_STYLE,
        EdgeKind::Association | EdgeKind::Subscription => ASSOCIATION_STYLE,
        EdgeKind::Connection => CONNECTION_STYLE,
    }
}

pub(crate) fn relationship_radius(incoming: usize, outgoing: usize) -> f64 {
    RELATIONSHIP_RADIUS
        .min(incoming as f64 / 2.0)
        .min(outgoing as f64 / 2.0)
}
