#![allow(stable_features)]
#![feature(let_else)]
#![feature(iter_array_chunks)]
#![allow(dead_code)]
#![feature(iter_collect_into)]


pub mod application;
pub mod build;
pub mod camera;
pub mod experiment;
pub mod fabric;
pub mod graphics;
pub mod gui;
pub mod scene;
pub mod test;

pub use application::run;
