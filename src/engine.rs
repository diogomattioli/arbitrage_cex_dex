use std::collections::{ BTreeMap, HashMap, HashSet };
use core::hash::Hash;

use rust_decimal::Decimal;

#[derive(Default)]
pub struct Engine<I, P = Decimal> {
    ids: HashMap<I, P>, // O(1)
    prices: BTreeMap<P, HashSet<I>>, // O(log(n) + 1) - ordered
    // space O(2n)
}

impl<I, P> Engine<I, P> where I: Hash + Ord + Clone, P: Ord + Clone {
    pub fn update(&mut self, exchange_id: I, price: P) {
        if
            let Some(previous_price) = self.ids.insert(exchange_id.clone(), price.clone()) // O(1)
        {
            if previous_price != price {
                // search and remove
                if
                    let Some(set) = self.prices.get_mut(&previous_price) // O(log(n))
                {
                    set.remove(&exchange_id); // O(1)
                    if set.is_empty() {
                        self.prices.remove(&previous_price); // O(log(n))
                    }
                }
            }
        }

        let set = self.prices.entry(price).or_insert(HashSet::new()); // O(2log(n))
        set.insert(exchange_id); // O(1)
    }

    pub fn lowest_price(&mut self) -> Option<(&P, impl Iterator<Item = &I>)> {
        self.prices.first_key_value().map(|v| (v.0, v.1.iter()))
    }

    pub fn highest_price(&mut self) -> Option<(&P, impl Iterator<Item = &I>)> {
        self.prices.last_key_value().map(|v| (v.0, v.1.iter()))
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&P, impl Iterator<Item = &I>)> {
        self.prices
            .iter()
            .map(|v| (v.0, v.1.iter()))
            .into_iter()
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn update() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(2));
        map.update("c".into(), dec!(3));

        assert_eq!(3, map.len());
    }

    #[test]
    fn update_reduced() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(2));
        map.update("c".into(), dec!(3));
        map.update("c".into(), dec!(2));

        assert_eq!(2, map.len());
    }

    #[test]
    fn update_extended() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(2));
        map.update("c".into(), dec!(2));
        map.update("c".into(), dec!(3));

        assert_eq!(3, map.len());
    }

    #[test]
    fn iter() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(2));
        map.update("c".into(), dec!(2));

        let mut it = map.iter();

        let value = it.next().unwrap();
        assert_eq!(dec!(1), *value.0);
        assert_eq!(vec!["a"], value.1.collect::<Vec<_>>());

        let value = it.next().unwrap();
        let mut keys = value.1.collect::<Vec<_>>();
        keys.sort();

        assert_eq!(dec!(2), *value.0);
        assert_eq!(vec!["b", "c"], keys);
    }

    #[test]
    fn lowest_price() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(2));
        map.update("c".into(), dec!(2));
        map.update("d".into(), dec!(3));

        assert_eq!(dec!(1), *map.lowest_price().unwrap().0);
    }

    #[test]
    fn highest_price() {
        let mut map = Engine::<String>::default();

        map.update("a".into(), dec!(1));
        map.update("b".into(), dec!(5));
        map.update("c".into(), dec!(2));
        map.update("d".into(), dec!(2));

        assert_eq!(dec!(5), *map.highest_price().unwrap().0);
    }
}
