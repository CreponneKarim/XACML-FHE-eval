use std::fmt::Debug;

use rkyv::{
    rancor::Fallible,
    Archive,
    Deserialize as RkyvDeserialize,
    Serialize as RkyvSerialize,
};
use serde::{
    Deserialize as SerdeDeserialize,
    Serialize as SerdeSerialize,
};

pub trait GetType<'de, S, Ser, De>
where
    Ser: Fallible + ?Sized,
    De: Fallible + ?Sized,
    S: Archive
        + RkyvSerialize<Ser>
        + Debug
        + SerdeSerialize
        + SerdeDeserialize<'de>,
    S::Archived: RkyvDeserialize<S, De>,
{
    fn get_type(&self) -> S;
}
