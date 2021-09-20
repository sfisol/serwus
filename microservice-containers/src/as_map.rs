use diesel::Identifiable;
use std::{
    cmp::Ord,
    collections::BTreeMap,
};

pub trait AsMap<'a, D: 'a>
where
    &'a D: Identifiable,
{
    fn as_map(&'a self) -> BTreeMap<<&'a D as Identifiable>::Id, D>;
}

impl<'a, D> AsMap<'a, D> for Vec<D>
where
    D: Clone + 'a,
    &'a D: Identifiable,
    <&'a D as Identifiable>::Id: Ord,
{
    fn as_map(&'a self) -> BTreeMap<<&'a D as Identifiable>::Id, D> {
        let mut acc = BTreeMap::new();
        for item in self {
            acc.insert(item.id(), (*item).clone());
        }
        acc
    }
}
