use super::{Point, pack, select_packing};

#[test]
fn packing_selection_matches_lean_cases() {
    for line in include_str!("../../../experiments/architecture/packing-cases.txt").lines() {
        let mut fields = line.split_whitespace();
        let name = fields.next().unwrap();
        let expected: usize = fields.next().unwrap().parse().unwrap();
        let candidates = fields
            .enumerate()
            .map(|(i, text)| {
                let mut numbers = text.split('/').map(|n| n.parse::<u128>().unwrap());
                let score = (
                    numbers.next().unwrap(),
                    numbers.next().unwrap(),
                    usize::try_from(numbers.next().unwrap()).unwrap(),
                );
                let cost = numbers.next().unwrap();
                assert!(numbers.next().is_none());
                (score, (i + 1, cost))
            })
            .collect();
        let winner = select_packing(candidates, |&(_, cost)| cost);
        assert_eq!(winner.map_or(0, |(_, (id, _))| id), expected, "{name}");
    }
}

#[test]
fn packing_uses_relationship_cost_within_the_compact_window() {
    let sizes = [(10, 40), (30, 20), (10, 40), (30, 20), (10, 40)];
    let mut positions = [Point { x: 0, y: 0 }; 5];
    let size = pack(&[vec![0, 1, 2, 3, 4]], &[(0, 4)], &sizes, &mut positions, 5);
    assert_eq!(size, (80, 65));
    assert_eq!(positions[4], Point { x: 70, y: 0 });
}
