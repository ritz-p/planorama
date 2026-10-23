use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use std::cell::Cell;

#[cfg(test)]
#[path = "../../tests/unit/generation_budget.rs"]
mod tests;

pub(super) fn validate(json: &str) -> Result<(), super::Error> {
    let remaining = Cell::new(32_768);
    let exceeded = Cell::new(false);
    let mut deserializer = serde_json::Deserializer::from_str(json.trim_start_matches('\u{feff}'));
    Seed {
        remaining: &remaining,
        exceeded: &exceeded,
        depth: 0,
    }
    .deserialize(&mut deserializer)
    .and_then(|()| deserializer.end())
    .map_err(|_| {
        if exceeded.get() {
            super::Error::ResourceLimit
        } else {
            super::Error::InvalidJson
        }
    })
}

#[derive(Clone, Copy)]
struct Seed<'a> {
    remaining: &'a Cell<usize>,
    exceeded: &'a Cell<bool>,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(mut self, deserializer: D) -> Result<(), D::Error> {
        if self.depth >= 64 || self.remaining.get() == 0 {
            self.exceeded.set(true);
            return Err(D::Error::custom("browser input structure budget exceeded"));
        }
        self.remaining.set(self.remaining.get() - 1);
        self.depth += 1;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        while sequence.next_element_seed(self)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while map.next_key_seed(self)?.is_some() {
            map.next_value_seed(self)?;
        }
        Ok(())
    }
}
