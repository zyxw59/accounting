use std::{collections::BTreeMap, ops};

use derivative::Derivative;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper around [`BTreeMap`] that (de)serializes as an array of key-value pairs.
#[derive(Clone, Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Map<K, V>(pub BTreeMap<K, V>);

impl<K, V> ops::Deref for Map<K, V> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> ops::DerefMut for Map<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V> IntoIterator for Map<K, V> {
    type IntoIter = <BTreeMap<K, V> as IntoIterator>::IntoIter;
    type Item = <BTreeMap<K, V> as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'de, K, V> Deserialize<'de> for Map<K, V>
where
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // deserializing into a vec is free, since `BTreeMap::from_iter` collects into a vec, and
        // collecting from a vec into a vec is specialized to a no-op.
        Vec::<(K, V)>::deserialize(deserializer)
            .map(Vec::into_iter)
            .map(BTreeMap::from_iter)
            .map(Self)
    }
}

impl<K, V> Serialize for Map<K, V>
where
    K: Serialize,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for (k, v) in self.0.iter() {
            seq.serialize_element(&(k, v))?;
        }
        seq.end()
    }
}
