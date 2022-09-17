use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub people: Option<Vec<Person>>,
}
