#![allow(stable_features)]
#![feature(let_else)]
#![feature(iter_array_chunks)]
#![allow(dead_code)]
#![feature(iter_collect_into)]


pub mod annealing;
pub mod application;
pub mod build;
pub mod camera;
pub mod example;
pub mod experiment;
pub mod fabric;
pub mod face;
pub mod graphics;
pub mod gui;
pub mod interval;
pub mod joint;
pub mod physics;
pub mod scene;
pub mod test;
pub mod twist;

pub use application::run;
