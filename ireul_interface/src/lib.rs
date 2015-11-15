#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

extern crate bincode;
extern crate ogg;
extern crate serde;

pub mod proxy;
pub mod oggutil;