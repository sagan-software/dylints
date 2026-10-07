#![allow(dead_code)]

struct Event;

struct Events(Vec<Event>);

impl Events {
    fn into_events(self) -> Vec<Event> {
        self.0
    }

    fn iter_events(&self) -> std::slice::Iter<'_, Event> {
        self.0.iter()
    }

    fn filtered_events(self) -> impl Iterator<Item = Event> {
        self.0.into_iter()
    }
}

fn main() {}
