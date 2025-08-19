// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use crate::Id;

// Mirrors pw_map and provides a compacted list to hold objects (with indices mapping to those
// objects' ids.
pub(crate) struct IdMap<T> {
    objects: Vec<Item<T>>,
    // TODO: optimise by retaining lowest free position?
}

// A tristate representing one of:
//   - Empty: for a slot that was occupied but is not
//   - Reserved: for a slot that has been reserved pending a value
//   - Occupied: for a slot that has a value
enum Item<T> {
    Empty,
    Reserved,
    Occupied(T),
}

impl<T> Item<T> {
    fn is_empty(&self) -> bool {
        match self {
            Item::Empty => true,
            _ => false,
        }
    }

    fn as_ref(&self) -> Option<&T> {
        match self {
            Item::Occupied(ref object) => Some(object),
            _ => None,
        }
    }
}

impl<T> IdMap<T> {
    pub(crate) fn new() -> IdMap<T> {
        Self {
            objects: Vec::new(),
        }
    }

    #[allow(unused)]
    pub(crate) fn insert(&mut self, object: T) -> Id {
        let item = self.objects.iter().enumerate().find(|(_, o)| o.is_empty());

        if let Some((id, _)) = item {
            self.objects[id] = Item::Occupied(object);
            id as Id
        } else {
            self.objects.push(Item::Occupied(object));
            (self.objects.len() - 1) as Id
        }
    }

    pub(crate) fn insert_at(&mut self, id: Id, object: T) {
        assert!(self.objects.len() > id as usize);

        match &mut self.objects[id as usize] {
            val @ Item::Reserved => *val = Item::Occupied(object),
            _ => unreachable!("IdMap insert should only be into a reserved location"),
        }
    }

    pub(crate) fn reserve(&mut self) -> Id {
        let item = self.objects.iter().enumerate().find(|(_, o)| o.is_empty());

        if let Some((id, _)) = item {
            self.objects[id] = Item::Reserved;
            id as Id
        } else {
            self.objects.push(Item::Reserved);
            (self.objects.len() - 1) as Id
        }
    }

    pub(crate) fn remove(&mut self, id: Id) {
        if self.objects.len() <= id as usize {
            return;
        }

        self.objects[id as usize] = Item::Empty;
    }

    pub(crate) fn clear(&mut self) {
        self.objects.clear();
    }

    pub(crate) fn get(&self, id: Id) -> Option<&T> {
        if (id as usize) < self.objects.len() {
            self.objects[id as usize].as_ref()
        } else {
            None
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        // Iterate over (idx, object), get references and return valid (idx, &object)
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(i, obj)| obj.as_ref().map(|o| (i as Id, o)))
    }
}

#[test]
fn test_id_map() {
    let mut map = IdMap::new();

    assert_eq!(map.insert(0), 0);
    assert_eq!(map.insert(1), 1);
    map.remove(2);

    assert_eq!(map.get(0), Some(&0));
    assert_eq!(map.get(1), Some(&1));
    assert_eq!(map.get(2), None);

    map.remove(0);

    assert_eq!(map.get(0), None);
    assert_eq!(map.get(1), Some(&1));

    assert_eq!(map.insert(2), 0);

    assert_eq!(map.get(0), Some(&2));
    assert_eq!(map.get(1), Some(&1));
    assert_eq!(map.reserve(), 2);

    map.remove(1);

    assert_eq!(map.get(0), Some(&2));
    assert_eq!(map.get(1), None);
    assert_eq!(map.get(2), None);
}
